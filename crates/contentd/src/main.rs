//! contentd — serves one published generation over a raw TCP stream.
//!
//! Every failure path answers `EMPTY`: the client must not be able to tell
//! "missing page" from "script crashed" from "bad request".

use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use protocol::{
    decode_client, encode_server, ManifestEntry, Request, ServerMsg, CHUNK, MAX_FRAME,
    SESSION_LEN,
};

const PY_TIMEOUT: Duration = Duration::from_secs(2);
const PY_MAX_OUTPUT: usize = 128 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_CONNECTIONS: usize = 32;

#[derive(serde::Deserialize, Clone)]
struct PageDef {
    id: u16,
    kind: String,
    title: String,
    hash: String,
    file: String,
}

#[derive(serde::Deserialize, Clone)]
struct ImageDef {
    id: u16,
    file: String,
}

#[derive(serde::Deserialize, Clone)]
struct Manifest {
    pages: Vec<PageDef>,
    images: Vec<ImageDef>,
}

struct State {
    data: PathBuf,
    python: String,
    /// Extra environment handed to page scripts (e.g. NAC_ROOT for the ops console).
    env: Vec<(String, String)>,
    /// Optional JSONL access log; off unless `--log` is given.
    log: Option<PathBuf>,
    /// Sessions untouched for this long are deleted.
    session_ttl: Duration,
    cache: Mutex<Option<(String, Arc<Manifest>)>>,
}

impl State {
    fn gen_dir(&self, gen: &str) -> PathBuf {
        self.data.join("gen").join(gen)
    }

    fn current_gen(&self) -> Option<String> {
        let raw = fs::read_to_string(self.data.join("current")).ok()?;
        let name = raw.trim().to_string();
        if name.is_empty() || name.contains("..") || name.contains('/') || name.contains('\\') {
            return None;
        }
        Some(name)
    }

    fn manifest(&self) -> Option<(String, Arc<Manifest>)> {
        let gen = self.current_gen()?;
        {
            let guard = self.cache.lock().ok()?;
            if let Some((cached_gen, m)) = guard.as_ref() {
                if *cached_gen == gen {
                    return Some((gen, m.clone()));
                }
            }
        }
        let path = self.gen_dir(&gen).join("manifest.json");
        let text = fs::read_to_string(path).ok()?;
        let parsed: Manifest = serde_json::from_str(&text).ok()?;
        let arc = Arc::new(parsed);
        if let Ok(mut guard) = self.cache.lock() {
            *guard = Some((gen.clone(), arc.clone()));
        }
        Some((gen, arc))
    }
}

fn proc_status() -> String {
    #[cfg(target_os = "linux")]
    {
        std::fs::read_to_string("/proc/self/status").unwrap_or_default()
    }
    #[cfg(not(target_os = "linux"))]
    {
        String::new()
    }
}

/// Reads the real UID out of `/proc/self/status`.
///
/// Returns `None` when the text is not a status file at all, which is the case
/// on every platform without `/proc` — there the check does not apply and the
/// caller lets it through.
fn root_from_status(status: &str) -> Option<bool> {
    let uid = status
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))?
        .split_whitespace()
        .next()?
        .parse::<u32>()
        .ok()?;
    Some(uid == 0)
}

fn main() {
    let mut data = PathBuf::from("data");
    let mut port: u16 = 7000;
    let mut bind = "127.0.0.1".to_string();
    let mut env: Vec<(String, String)> = Vec::new();
    let mut log: Option<PathBuf> = None;
    let mut session_ttl = Duration::from_secs(7 * 24 * 3600);
    let mut allow_root = false;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--data" => data = PathBuf::from(args.next().unwrap_or_else(|| "data".into())),
            "--port" => port = args.next().and_then(|v| v.parse().ok()).unwrap_or(7000),
            "--bind" => {
                bind = args
                    .next()
                    .unwrap_or_else(|| "127.0.0.1".to_string())
            }
            "--log" => log = args.next().map(PathBuf::from),
            "--allow-root" => allow_root = true,
            "--session-ttl-hours" => {
                let hours = args.next().and_then(|v| v.parse::<u64>().ok()).unwrap_or(168);
                session_ttl = Duration::from_secs(hours.saturating_mul(3600));
            }
            "--env" => {
                let pair = args.next().unwrap_or_default();
                let (key, value) = pair.split_once('=').unwrap_or((pair.as_str(), ""));
                if !key.is_empty() {
                    env.push((key.to_string(), value.to_string()));
                }
            }
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }

    // Page scripts run with this process's privileges, so starting as root would
    // turn any page bug into a root compromise. Refuse rather than warn: a unit
    // file with `User=` missing is exactly the mistake worth catching.
    if !allow_root {
        if let Some(true) = root_from_status(&proc_status()) {
            eprintln!("拒绝以 root 运行：页面脚本会继承 root 权限。请用普通用户启动，");
            eprintln!("或确实需要时加 --allow-root（不推荐）。");
            std::process::exit(3);
        }
    }

    let python = std::env::var("NAC_PYTHON").unwrap_or_else(|_| {
        if cfg!(windows) {
            "python".to_string()
        } else {
            "python3".to_string()
        }
    });

    // Everything handed to a child process must be absolute: the child runs
    // with its cwd set to the generation directory.
    let data = std::path::absolute(&data).unwrap_or(data);

    let state = Arc::new(State {
        data,
        python,
        env,
        log,
        session_ttl,
        cache: Mutex::new(None),
    });

    spawn_session_pruner(state.clone());

    let addr = format!("{bind}:{port}");
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("bind {addr}: {e}");
            std::process::exit(1);
        }
    };
    eprintln!("contentd listening on {addr}");

    let active = Arc::new(AtomicUsize::new(0));
    for conn in listener.incoming() {
        match conn {
            Ok(stream) => {
                if active.fetch_add(1, Ordering::Relaxed) >= MAX_CONNECTIONS {
                    active.fetch_sub(1, Ordering::Relaxed);
                    drop(stream);
                    continue;
                }
                let st = state.clone();
                let active_thread = active.clone();
                if let Err(e) = thread::Builder::new().spawn(move || {
                    if let Err(e) = handle(stream, st) {
                        eprintln!("connection: {e}");
                    }
                    active_thread.fetch_sub(1, Ordering::Relaxed);
                }) {
                    active.fetch_sub(1, Ordering::Relaxed);
                    eprintln!("spawn: {e}");
                }
            }
            Err(e) => eprintln!("accept: {e}"),
        }
    }
}

fn handle(mut stream: TcpStream, state: Arc<State>) -> std::io::Result<()> {
    let started = Instant::now();
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    let Some(frame) = read_frame(&mut stream)? else {
        return Ok(());
    };
    let Some(msg) = decode_client(&frame) else {
        log_line(&state, None, "malformed", started, 0, None);
        return send(&mut stream, &[ServerMsg::Empty]);
    };

    let (manifest_gen, manifest) = match state.manifest() {
        Some(v) => v,
        None => {
            log_line(&state, None, "no-generation", started, 0, None);
            return send(&mut stream, &[ServerMsg::Empty]);
        }
    };
    let gen_dir = state.gen_dir(&manifest_gen);

    let session = resolve_session(&state, &msg.session);
    let sess = hex8(&session.id);

    let (responses, page, result) = match &msg.request {
        Request::Manifest => {
            let mut entries: Vec<ManifestEntry> = manifest
                .pages
                .iter()
                .map(|p| ManifestEntry {
                    id: p.id,
                    dynamic: p.kind == "dynamic",
                    hash: u64::from_str_radix(&p.hash, 16).unwrap_or(0),
                    title: p.title.clone(),
                })
                .collect();
            entries.sort_by_key(|e| e.id);
            let count = entries.len();
            (
                vec![ServerMsg::Manifest {
                    session: session.id,
                    entries,
                }],
                None,
                format!("manifest:{count}"),
            )
        }
        Request::Image(id, offset) => match manifest.images.iter().find(|i| i.id == *id) {
            Some(def) => match fs::read(gen_dir.join(&def.file)) {
                Ok(bytes) => (vec![image_frame(*id, &bytes, *offset)], Some(*id), "image".to_string()),
                Err(_) => (vec![ServerMsg::Empty], Some(*id), "image-missing".to_string()),
            },
            None => (vec![ServerMsg::Empty], Some(*id), "image-unknown".to_string()),
        },
        Request::Page(id) => {
            let out = serve_page(&state, &manifest, &gen_dir, &session, *id, &[], 0);
            let result = outcome(&out);
            (out, Some(*id), result)
        }
        Request::Submit(id, fields) => {
            let out = serve_page(&state, &manifest, &gen_dir, &session, *id, fields, 0);
            let result = outcome(&out);
            (out, Some(*id), result)
        }
    };

    let bytes: usize = responses
        .iter()
        .map(|m| match m {
            ServerMsg::Page { data, .. } | ServerMsg::Image { data, .. } => data.len(),
            ServerMsg::Empty => 16,
            _ => 0,
        })
        .sum();
    log_line(&state, page, &result, started, bytes, Some(&sess));
    send(&mut stream, &responses)
}

fn outcome(responses: &[ServerMsg]) -> String {
    match responses.first() {
        Some(ServerMsg::Empty) | None => "fail".to_string(),
        Some(ServerMsg::Page { id, .. }) => format!("ok:{id}"),
        _ => "ok".to_string(),
    }
}

fn hex8(id: &[u8; 16]) -> String {
    id[..4].iter().map(|b| format!("{b:02x}")).collect()
}

/// One JSONL line per request. Never carries the peer address: the whole point
/// of the client is that it does not identify itself.
fn log_line(
    state: &State,
    page: Option<u16>,
    result: &str,
    started: Instant,
    bytes: usize,
    session: Option<&str>,
) {
    let Some(path) = state.log.as_ref() else {
        return;
    };
    let ms = started.elapsed().as_millis();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let line = format!(
        "{{\"t\":{stamp},\"page\":{},\"result\":\"{result}\",\"ms\":{ms},\"bytes\":{bytes},\"sess\":\"{}\"}}\n",
        page.map(|p| p.to_string()).unwrap_or_else(|| "null".to_string()),
        session.unwrap_or("")
    );
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = file.write_all(line.as_bytes());
    }
}

// ------------------------------------------------------------------ pages

fn serve_page(
    state: &State,
    manifest: &Manifest,
    gen_dir: &Path,
    session: &SessionRef,
    id: u16,
    fields: &[(String, String)],
    depth: u8,
) -> Vec<ServerMsg> {
    let Some(def) = manifest.pages.iter().find(|p| p.id == id) else {
        return vec![ServerMsg::Empty];
    };

    if def.kind != "dynamic" {
        let Ok(bytes) = fs::read(gen_dir.join(&def.file)) else {
            return vec![ServerMsg::Empty];
        };
        if bytes.len() > CHUNK {
            return vec![ServerMsg::Empty];
        }
        return vec![page_frame(id, &bytes, 0)];
    }

    if depth >= 1 {
        return vec![ServerMsg::Empty];
    }

    match run_python(state, gen_dir, &def.file, id, fields, session) {
        Ok(PyOut::Body(bytes)) => {
            if bytes.len() > CHUNK {
                return vec![ServerMsg::Empty];
            }
            vec![page_frame(id, &bytes, 0)]
        }
        Ok(PyOut::Redirect(target)) => {
            serve_page(state, manifest, gen_dir, session, target, &[], depth + 1)
        }
        Err(_) => vec![ServerMsg::Empty],
    }
}

fn page_frame(id: u16, bytes: &[u8], offset: u32) -> ServerMsg {
    let total = bytes.len() as u32;
    let start = (offset as usize).min(bytes.len());
    let end = (start + CHUNK).min(bytes.len());
    ServerMsg::Page {
        id,
        total,
        offset: start as u32,
        data: bytes[start..end].to_vec(),
    }
}

fn image_frame(id: u16, bytes: &[u8], offset: u32) -> ServerMsg {
    let total = bytes.len() as u32;
    let start = (offset as usize).min(bytes.len());
    let end = (start + CHUNK).min(bytes.len());
    ServerMsg::Image {
        id,
        total,
        offset: start as u32,
        data: bytes[start..end].to_vec(),
    }
}

fn send(stream: &mut TcpStream, msgs: &[ServerMsg]) -> std::io::Result<()> {
    for m in msgs {
        stream.write_all(&encode_server(m))?;
    }
    stream.flush()
}

fn read_frame(stream: &mut TcpStream) -> std::io::Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    if stream.read_exact(&mut len_buf).is_err() {
        return Ok(None);
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 || len > MAX_FRAME {
        return Ok(None);
    }
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf)?;
    // Re-attach the length prefix: decoders expect the whole frame.
    let mut frame = Vec::with_capacity(4 + len);
    frame.extend_from_slice(&len_buf);
    frame.extend_from_slice(&buf);
    Ok(Some(frame))
}

// ---------------------------------------------------------------- session

struct SessionRef {
    id: [u8; SESSION_LEN],
    file: PathBuf,
    persist: bool,
}

impl Drop for SessionRef {
    fn drop(&mut self) {
        if !self.persist {
            let _ = fs::remove_file(&self.file);
        }
    }
}

fn resolve_session(state: &State, given: &[u8; SESSION_LEN]) -> SessionRef {
    let zero = given.iter().all(|b| *b == 0);
    let id = if zero { random_session() } else { *given };
    let hex = to_hex(&id);

    let dir = if zero {
        state.data.join("state").join("tmp")
    } else {
        state.data.join("state").join("sessions")
    };
    let _ = fs::create_dir_all(&dir);
    let file = dir.join(format!("{hex}.json"));
    if !file.exists() {
        let _ = fs::write(&file, b"{}");
    }

    SessionRef {
        id,
        file,
        persist: !zero,
    }
}

// ------------------------------------------------------------- session 清理

/// Sessions and leftover temp files would otherwise accumulate forever: one
/// file per visitor, never removed. Prune on start and then every 10 minutes.
fn spawn_session_pruner(state: Arc<State>) {
    thread::spawn(move || loop {
        let removed = prune_sessions(&state);
        if removed > 0 {
            eprintln!("pruned {removed} stale session file(s)");
        }
        thread::sleep(Duration::from_secs(600));
    });
}

fn prune_sessions(state: &State) -> usize {
    let now = SystemTime::now();
    let mut removed = 0;
    for (name, cutoff) in [
        ("sessions", state.session_ttl),
        ("tmp", Duration::from_secs(3600)),
    ] {
        let dir = state.data.join("state").join(name);
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Ok(meta) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = meta.modified() else {
                continue;
            };
            if now.duration_since(modified).unwrap_or_default() > cutoff
                && fs::remove_file(&path).is_ok()
            {
                removed += 1;
            }
        }
    }
    removed
}

fn random_session() -> [u8; SESSION_LEN] {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut x = nanos ^ (COUNTER.fetch_add(1, Ordering::Relaxed) << 32) ^ 0x9e37_79b9_7f4a_7c15;
    let mut out = [0u8; SESSION_LEN];
    for chunk in out.chunks_mut(8) {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let bytes = x.to_be_bytes();
        chunk.copy_from_slice(&bytes[..chunk.len()]);
    }
    out
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

// ----------------------------------------------------------------- python

enum PyOut {
    Body(Vec<u8>),
    Redirect(u16),
}

fn run_python(
    state: &State,
    gen_dir: &Path,
    script_rel: &str,
    page: u16,
    fields: &[(String, String)],
    session: &SessionRef,
) -> Result<PyOut, ()> {
    let script_path = gen_dir.join(script_rel);
    if !script_path.is_file() {
        return Err(());
    }

    let mut field_map = serde_json::Map::new();
    for (k, v) in fields {
        field_map.insert(k.clone(), serde_json::Value::String(v.clone()));
    }
    let payload = serde_json::json!({ "page": page, "fields": field_map }).to_string();

    let data_dir = state.data.join("appdata");
    let _ = fs::create_dir_all(&data_dir);

    let mut command = Command::new(&state.python);
    command
        .arg("-B")
        .arg(script_rel)
        .current_dir(gen_dir)
        .env("PYTHONIOENCODING", "utf-8")
        .env("PYTHONUTF8", "1")
        .env("NAC_PAGE", page.to_string())
        .env("NAC_INPUT", payload)
        .env("NAC_SESSION_FILE", &session.file)
        .env("NAC_DATA_DIR", &data_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in &state.env {
        command.env(key, value);
    }

    let mut child = command.spawn().map_err(|_| ())?;

    let stdout = child.stdout.take().ok_or(())?;
    let stderr = child.stderr.take().ok_or(())?;
    let out_handle = thread::spawn(move || read_capped(stdout, PY_MAX_OUTPUT));
    let err_handle = thread::spawn(move || read_capped(stderr, 8 * 1024));

    let deadline = Instant::now() + PY_TIMEOUT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                thread::sleep(Duration::from_millis(10));
            }
            Err(_) => break None,
        }
    };

    let (out, out_truncated) = out_handle.join().unwrap_or((Vec::new(), true));
    let (err, _) = err_handle.join().unwrap_or((Vec::new(), true));

    if !err.is_empty() {
        eprintln!("[page {page}] {}", String::from_utf8_lossy(&err));
    }

    let Some(status) = status else {
        eprintln!("[page {page}] timeout or wait failure");
        return Err(());
    };
    if !status.success() || out_truncated {
        return Err(());
    }

    let text = String::from_utf8(out).map_err(|_| ())?;
    if let Some(rest) = text.trim_start().strip_prefix("@@REDIRECT ") {
        let id = rest
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .parse::<u16>()
            .map_err(|_| ())?;
        return Ok(PyOut::Redirect(id));
    }

    Ok(PyOut::Body(text.into_bytes()))
}

fn read_capped<R: Read>(mut r: R, limit: usize) -> (Vec<u8>, bool) {
    let mut out = Vec::new();
    let mut buf = [0u8; 8192];
    let mut truncated = false;
    loop {
        match r.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if out.len() < limit {
                    let take = (limit - out.len()).min(n);
                    out.extend_from_slice(&buf[..take]);
                    if take < n {
                        truncated = true;
                    }
                } else {
                    truncated = true;
                }
            }
            Err(_) => break,
        }
    }
    (out, truncated)
}


#[cfg(test)]
mod tests {
    use super::root_from_status;

    #[test]
    fn detects_root_from_proc_status() {
        let root = "Name:\tcontentd\nUid:\t0\t0\t0\t0\nGid:\t0\t0\t0\t0\n";
        assert_eq!(root_from_status(root), Some(true));
    }

    #[test]
    fn detects_a_normal_user() {
        let user = "Name:\tcontentd\nUid:\t998\t998\t998\t998\nGid:\t998\t998\t998\t998\n";
        assert_eq!(root_from_status(user), Some(false));
    }

    #[test]
    fn no_proc_status_means_the_check_does_not_apply() {
        assert_eq!(root_from_status(""), None);
        assert_eq!(root_from_status("Name:\tcontentd\n"), None);
        assert_eq!(root_from_status("Uid:\tnot-a-number\n"), None);
    }
}
