//! publish — builds one immutable generation from `site/` and flips `data/current`.
//!
//! Authoring rules:
//!   * every page is a Python script: `site/<id>.py`
//!   * the page title comes from a `# nac:title <文字>` line
//!   * scripts build markup with the generated `nac` helper module
//!   * images live under `site/`; `nac.img("img/x.png", 60)` inserts one
//!
//! The tool bakes the page/image tables into `nac.py`, checks that every
//! script parses, and refuses to publish a broken generation.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

mod images;

const NAC_PY: &str = include_str!("nac_py.tmpl");

struct PageSource {
    rel: String,
    url: String,
    text: String,
    /// Prefix this page answers for, from `# nac:match /item/`.
    matches: Option<String>,
}

fn main() {
    let mut site = PathBuf::from("site");
    let mut data = PathBuf::from("data");
    let mut archive: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--site" => site = PathBuf::from(args.next().unwrap_or_else(|| "site".into())),
            "--data" => data = PathBuf::from(args.next().unwrap_or_else(|| "data".into())),
            "--archive" => archive = args.next().map(PathBuf::from),
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }

    if let Err(e) = run(&site, &data, archive.as_deref()) {
        eprintln!("publish failed: {e}");
        std::process::exit(1);
    }
}

fn run(site: &Path, data: &Path, archive: Option<&Path>) -> Result<(), String> {
    let mut files = Vec::new();
    walk(site, &mut files)?;

    struct PageFile {
        rel: String,
        url: String,
        text: String,
        explicit_id: Option<u16>,
        matches: Option<String>,
    }
    let mut page_files: Vec<PageFile> = Vec::new();
    let mut urls: BTreeSet<String> = BTreeSet::new();
    let mut used_ids: BTreeSet<u16> = BTreeSet::new();
    let mut images: Vec<(String, PathBuf)> = Vec::new();
    // Files the pages need at run time: modules to import, data to read, anything
    // that is not itself a page. Copied verbatim so a page can do what a normal
    // Python program does.
    let mut support: Vec<(String, PathBuf)> = Vec::new();

    for path in files {
        let rel = rel_string(site, &path);
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        // A leading `_` on any part of the path means "support material, not a
        // page" — the same convention Python itself uses for "leave this alone".
        let support_only = is_support(&rel);
        match ext.as_str() {
            "py" if !support_only => {
                let text = fs::read_to_string(&path).map_err(|e| format!("{rel}: {e}"))?;
                // Notepad 之类会存 BOM；Python 会因此报"非法字符"，先摘掉。
                let text = text.strip_prefix('\u{feff}').unwrap_or(&text).to_string();
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_string();
                let url = page_url(site, &path, &text)?;
                if !urls.insert(url.clone()) {
                    return Err(format!("{rel}: 路径 {url} 被用了两次"));
                }
                let explicit_id = stem.parse::<u16>().ok();
                if let Some(id) = explicit_id {
                    if !used_ids.insert(id) {
                        return Err(format!("{rel}: 页号 {id} 被用了两次"));
                    }
                }
                let matches = page_match(&text, &rel)?;
                page_files.push(PageFile {
                    rel,
                    url,
                    text,
                    explicit_id,
                    matches,
                });
            }
            "py" => {
                check_syntax(
                    &fs::read_to_string(&path).map_err(|e| format!("{rel}: {e}"))?,
                    &rel,
                )?;
                support.push((rel, path));
            }
            "page" => {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("<id>");
                return Err(format!(
                    "{rel}: 不再支持 .page，改成同名 Python 页面 {stem}.py"
                ));
            }
            "png" | "jpg" | "jpeg" | "webp" | "gif" if !support_only => {
                images.push((rel, path))
            }
            "bmp" | "tif" | "tiff" | "avif" | "heic" if !support_only => {
                return Err(format!(
                    "{rel}: 不支持 {ext}；请导出成 png（无损）或 jpeg / webp / gif"
                ));
            }
            _ => support.push((rel, path)),
        }
    }

    if page_files.is_empty() {
        return Err("site/ 里没有页面（需要 index.py 或 <名字>.py）".into());
    }

    page_files.sort_by(|a, b| a.url.cmp(&b.url));
    let mut pages: BTreeMap<u16, PageSource> = BTreeMap::new();
    for page in page_files {
        let id = match page.explicit_id {
            Some(id) => id,
            None => allocate_page_id(&page.url, &used_ids)
                .ok_or_else(|| format!("{}: 页号空间耗尽", page.rel))?,
        };
        used_ids.insert(id);
        pages.insert(
            id,
            PageSource {
                rel: page.rel,
                url: page.url,
                text: page.text,
                matches: page.matches,
            },
        );
    }

    // data 目录若在 site 里，上一代产物会被当成页面源文件扫进来。
    if let (Ok(site_abs), Ok(data_abs)) = (
        std::path::absolute(site),
        std::path::absolute(data),
    ) {
        if data_abs.starts_with(&site_abs) {
            return Err(format!(
                "data 目录 {} 在 site 目录 {} 里面：上一代产物会被当成页面源文件",
                data_abs.display(),
                site_abs.display()
            ));
        }
    }
    if let Some(dir) = archive {
        if let Ok(dir_abs) = std::path::absolute(dir) {
            if let Ok(site_abs) = std::path::absolute(site) {
                if dir_abs.starts_with(&site_abs) {
                    return Err(format!(
                        "archive 目录 {} 在 site 目录 {} 里面：归档的原图会被当成新素材再发一次",
                        dir_abs.display(),
                        site_abs.display()
                    ));
                }
            }
        }
    }

    images.sort_by(|a, b| a.0.cmp(&b.0));
    let mut image_ids: BTreeMap<String, u16> = BTreeMap::new();
    for (i, (rel, _)) in images.iter().enumerate() {
        let id = u16::try_from(i + 1).map_err(|_| "too many images".to_string())?;
        image_ids.insert(rel.clone(), id);
    }

    let previous_gen = fs::read_to_string(data.join("current"))
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && !s.contains('/') && !s.contains('\\'));
    let gen = next_gen_name(data)?;
    let gen_dir = data.join("gen").join(&gen);
    fs::create_dir_all(gen_dir.join("scripts")).map_err(|e| e.to_string())?;
    fs::create_dir_all(gen_dir.join("img")).map_err(|e| e.to_string())?;

    // Images first: the helper module carries their ids and sizes.
    let mut image_entries = Vec::new();
    let mut image_sizes: BTreeMap<String, (u16, u32, u32)> = BTreeMap::new();
    for (rel, src) in &images {
        let id = image_ids[rel];
        // 先归档原件（带元数据），再剥一份用于发布——两者互不影响。
        if let Some(dir) = archive {
            let dest = dir.join(rel);
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            archive_original(src, &dest)?;
        }
        let raw = fs::read(src).map_err(|e| format!("{rel}: {e}"))?;
        let clean = images::strip(&raw).map_err(|e| format!("{rel}: {e}"))?;
        let rel_file = format!("img/{id}.bin");
        let dest = gen_dir.join(&rel_file);
        let linked = previous_gen.as_ref().and_then(|prev| {
            let old = data.join("gen").join(prev).join(&rel_file);
            let old_bytes = fs::read(&old).ok()?;
            if old_bytes == clean.bytes {
                fs::hard_link(&old, &dest).ok()
            } else {
                None
            }
        });
        if linked.is_none() {
            fs::write(&dest, &clean.bytes).map_err(|e| e.to_string())?;
        }
        image_sizes.insert(rel.clone(), (id, clean.width, clean.height));
        image_entries.push(serde_json::json!({
            "id": id,
            "file": rel_file,
            "w": clean.width,
            "h": clean.height,
        }));
    }

    let helper = NAC_PY
        .replace("@PAGES@", &page_list(&pages))
        .replace("@PATHS@", &path_map(&pages))
        .replace("@MATCHES@", &match_map(&pages))
        .replace("@IMAGES@", &image_map(&image_sizes))
        .replace("@TITLES@", &title_map(&pages))
        .replace("@GEN@", &gen);
    fs::write(gen_dir.join("scripts").join("nac.py"), helper).map_err(|e| e.to_string())?;

    // Support material goes next to the pages, keeping its own path, so a page can
    // `import _lib.hn` or `open("_data/seed.json")` the way any Python program
    // would — both resolve relative to the working directory, which is `scripts/`.
    for (rel, src) in &support {
        let dest = gen_dir.join("scripts").join(rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::copy(src, &dest).map_err(|e| format!("{rel}: {e}"))?;
    }

    let mut page_entries = Vec::new();
    for (id, page) in &pages {
        let text = &page.text;
        check_syntax(text, &page.rel)?;
        let title = page_title(text).unwrap_or_default();
        let rel_file = format!("scripts/{id}.py");
        fs::write(gen_dir.join(&rel_file), text.as_bytes()).map_err(|e| e.to_string())?;

        let is_static = text.lines().any(|line| line.trim() == "# nac:static");
        if is_static {
            let body = render_static(&gen_dir.join("scripts"), *id, data)?;
            if body.len() > protocol::CHUNK {
                return Err(format!(
                    "{}: static output exceeds {} bytes",
                    page.rel,
                    protocol::CHUNK
                ));
            }
            let static_file = format!("static/{id}.txt");
            fs::create_dir_all(gen_dir.join("static")).map_err(|e| e.to_string())?;
            fs::write(gen_dir.join(&static_file), &body).map_err(|e| e.to_string())?;
            page_entries.push(serde_json::json!({
                "id": id,
                "kind": "static",
                "title": title,
                "hash": format!("{:016x}", protocol::fnv1a64(&body)),
                "file": static_file,
                "path": page.url,
            }));
        } else {
            page_entries.push(serde_json::json!({
                "id": id,
                "kind": "dynamic",
                "title": title,
                "hash": format!("{:016x}", protocol::fnv1a64(text.as_bytes())),
                "file": rel_file,
                "path": page.url,
                "match": page.matches,
            }));
        }
    }

    let manifest = serde_json::json!({
        "gen": gen,
        "pages": page_entries,
        "images": image_entries,
    });
    fs::write(
        gen_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    switch_current(&data, &gen)?;

    println!(
        "published {} ({} pages, {} images)",
        gen,
        pages.len(),
        image_entries.len()
    );
    prune(data);
    Ok(())
}

/// Any path component starting with `_` marks support material: modules to
/// import, data files to read — not pages. Same idea as Python's own rule for
/// names that are internal.
fn is_support(rel: &str) -> bool {
    rel.split('/').any(|part| part.starts_with('_'))
}

/// Reads `# nac:match /item/` — the prefix a dynamic page answers for.
fn page_match(text: &str, rel: &str) -> Result<Option<String>, String> {
    for line in text.lines() {
        let Some(rest) = line.trim().strip_prefix("# nac:match") else {
            continue;
        };
        if !rest.chars().next().map_or(true, char::is_whitespace) {
            continue;
        }
        let value = rest.trim();
        if value.is_empty() {
            return Err(format!("{rel}: # nac:match 后面要写前缀路径，例如 /item/"));
        }
        if !value.starts_with('/') || !value.ends_with('/') {
            return Err(format!(
                "{rel}: # nac:match 必须以 / 开头并以 / 结尾，例如 /item/（现在是 {value}）"
            ));
        }
        if text.lines().any(|l| l.trim() == "# nac:static") {
            return Err(format!(
                "{rel}: 静态页不能声明 # nac:match —— 预渲染的结果只有一份，没法按地址区分"
            ));
        }
        return Ok(Some(value.to_string()));
    }
    Ok(None)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            walk(&path, out)?;
        } else {
            out.push(path);
        }
    }
    Ok(())
}

fn rel_string(base: &Path, path: &Path) -> String {
    let rel = path.strip_prefix(base).unwrap_or(path);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn page_url(site: &Path, path: &Path, text: &str) -> Result<String, String> {
    let rel = rel_string(site, path);
    if let Some(value) = text.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("# nac:path")?;
        if rest.chars().next().map_or(true, char::is_whitespace) {
            Some(rest.trim())
        } else {
            None
        }
    }) {
        if value.is_empty() {
            return Err(format!("{rel}: # nac:path 后面要写路径"));
        }
        let raw = if value.starts_with('/') {
            value.to_string()
        } else {
            format!("/{value}")
        };
        return normalize_page_path(&raw).map_err(|e| format!("{rel}: {e}"));
    }

    let rel_path = path.strip_prefix(site).unwrap_or(path);
    let stem = rel_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let mut dirs: Vec<String> = Vec::new();
    if let Some(parent) = rel_path.parent() {
        for part in parent.components() {
            let part = part.as_os_str().to_string_lossy();
            if !part.is_empty() && part != "." {
                dirs.push(part.to_string());
            }
        }
    }
    if stem == "index" {
        let dir = dirs.join("/");
        Ok(if dir.is_empty() {
            "/".to_string()
        } else {
            format!("/{dir}/")
        })
    } else {
        dirs.push(stem.to_string());
        normalize_page_path(&format!("/{}", dirs.join("/")))
    }
}

fn normalize_page_path(raw: &str) -> Result<String, String> {
    if raw.len() > 1024 {
        return Err("路径太长（上限 1024 字节）".into());
    }
    if !raw.starts_with('/') {
        return Err("路径必须以 / 开头".into());
    }
    let mut parts = Vec::new();
    for part in raw.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err("路径不能包含 ..".into());
        }
        if part
            .chars()
            .any(|c| c.is_control() || matches!(c, '?' | '#' | '|' | '\\' | '[' | ']'))
        {
            return Err(format!("路径里有不支持的字符：{part}"));
        }
        parts.push(part);
    }
    let mut out = format!("/{}", parts.join("/"));
    if raw.ends_with('/') && out != "/" {
        out.push('/');
    }
    Ok(out)
}

fn allocate_page_id(url: &str, used: &BTreeSet<u16>) -> Option<u16> {
    const START: u32 = 1000;
    const SPAN: u32 = u16::MAX as u32 - START + 1;
    let mut id = START + (protocol::fnv1a64(url.as_bytes()) % SPAN as u64) as u32;
    for _ in 0..SPAN {
        let candidate = id as u16;
        if !used.contains(&candidate) {
            return Some(candidate);
        }
        id += 1;
        if id >= START + SPAN {
            id = START;
        }
    }
    None
}

fn page_title(text: &str) -> Option<String> {
    text.lines()
        .map(|l| l.trim())
        .find_map(|l| l.strip_prefix("# nac:title").map(|t| t.trim().to_string()))
        .filter(|t| !t.is_empty())
}

fn page_list(pages: &BTreeMap<u16, PageSource>) -> String {
    let ids: Vec<String> = pages.keys().map(|id| id.to_string()).collect();
    format!("({})", ids.join(", "))
}

fn path_map(pages: &BTreeMap<u16, PageSource>) -> String {
    let mut out = String::from("{");
    for (i, (id, page)) in pages.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("{:?}: {}", page.url, id));
    }
    out.push('}');
    out
}

/// Prefix -> page id, so `link_path("/item/42", ...)` can tell that `/item/42`
/// belongs to the page that answers `/item/`.
fn match_map(pages: &BTreeMap<u16, PageSource>) -> String {
    let mut out = String::from("{");
    let mut first = true;
    for (id, page) in pages.iter() {
        let Some(prefix) = &page.matches else {
            continue;
        };
        if !first {
            out.push_str(", ");
        }
        first = false;
        out.push_str(&format!("{prefix:?}: {id}"));
    }
    out.push('}');
    out
}

fn title_map(pages: &BTreeMap<u16, PageSource>) -> String {
    let mut out = String::from("{");
    for (i, (id, page)) in pages.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        let title = page_title(&page.text).unwrap_or_default();
        out.push_str(&format!("{id}: {:?}", title));
    }
    out.push('}');
    out
}

fn image_map(images: &BTreeMap<String, (u16, u32, u32)>) -> String {
    let mut out = String::from("{");
    for (i, (path, (id, w, h))) in images.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("{path:?}: ({id}, {w}, {h})"));
    }
    out.push('}');
    out
}

/// Parses every page before publishing: a syntax error found here is one you
/// do not find as an empty page later. Silently skipped when Python is absent.
fn check_syntax(text: &str, rel: &str) -> Result<(), String> {
    let python = std::env::var("NAC_PYTHON").unwrap_or_else(|_| {
        if cfg!(windows) {
            "python".to_string()
        } else {
            "python3".to_string()
        }
    });
    let script = "import sys;compile(sys.stdin.read(),'<page>','exec')";
    let mut child = match std::process::Command::new(&python)
        .args(["-B", "-X", "utf8", "-c", script])
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return Ok(()),
    };
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(text.as_bytes()).ok();
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if out.status.success() {
        return Ok(());
    }
    let err = String::from_utf8_lossy(&out.stderr);
    let detail = err
        .lines()
        .filter(|l| !l.trim().is_empty())
        .next_back()
        .unwrap_or("语法错误")
        .trim()
        .to_string();
    Err(format!("{rel}: {detail}"))
}


fn render_static(scripts: &Path, id: u16, data: &Path) -> Result<Vec<u8>, String> {
    let python = std::env::var("NAC_PYTHON").unwrap_or_else(|_| {
        if cfg!(windows) {
            "python".to_string()
        } else {
            "python3".to_string()
        }
    });
    let temp = data
        .join("state")
        .join("tmp")
        .join(format!("static-{id}-{}", std::process::id()));
    fs::create_dir_all(&temp).map_err(|e| e.to_string())?;
    let output = Command::new(&python)
        .arg("-B")
        .arg(format!("{id}.py"))
        .current_dir(scripts)
        .env("NAC_INPUT", format!("{{\"page\":{id},\"fields\":{{}}}}"))
        .env("NAC_PAGE", id.to_string())
        .env("NAC_STATIC", "1")
        .env("NAC_DATA_DIR", &temp)
        .env("NAC_SESSION_FILE", temp.join("session.json"))
        .env("PYTHONUTF8", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .output()
        .map_err(|e| format!("static {id}.py: {e}"))?;
    let _ = fs::remove_dir_all(&temp);
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        let detail = err.lines().last().unwrap_or("static page failed").trim();
        return Err(format!("static {id}.py: {detail}"));
    }
    Ok(output.stdout)
}

/// Copy an original into the archive without inheriting its mode bits.
///
/// `fs::copy` also copies permissions, which fails with EPERM when the target
/// belongs to the other publishing user; the install script and the console
/// share this directory, so overwrite the bytes in place instead.
fn archive_original(src: &Path, dest: &Path) -> Result<(), String> {
    let mut input = fs::File::open(src).map_err(|e| format!("{}: {e}", src.display()))?;
    let mut output = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(dest)
        .map_err(|e| format!("{}: {e}", dest.display()))?;
    io::copy(&mut input, &mut output).map_err(|e| format!("{}: {e}", dest.display()))?;
    output
        .sync_all()
        .map_err(|e| format!("{}: {e}", dest.display()))?;
    Ok(())
}

fn switch_current(data: &Path, gen: &str) -> Result<(), String> {
    let tmp = data.join(".current.tmp");
    let current = data.join("current");
    {
        let mut file = fs::File::create(&tmp).map_err(|e| e.to_string())?;
        file.write_all(gen.as_bytes()).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&tmp, fs::Permissions::from_mode(0o660));
    }
    #[cfg(windows)]
    {
        let _ = fs::remove_file(&current);
    }
    fs::rename(&tmp, &current).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        if let Ok(dir) = fs::File::open(data) {
            let _ = dir.sync_all();
        }
    }
    Ok(())
}

fn next_gen_name(data: &Path) -> Result<String, String> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    let mut name = format!("{secs}");
    let mut n = 1;
    while data.join("gen").join(&name).exists() {
        name = format!("{secs}-{n}");
        n += 1;
    }
    Ok(name)
}

fn prune(data: &Path) {
    let gen_root = data.join("gen");
    let Ok(entries) = fs::read_dir(&gen_root) else {
        return;
    };
    let mut gens: Vec<(String, SystemTime)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let mtime = e.metadata().ok()?.modified().ok()?;
            Some((name, mtime))
        })
        .collect();
    gens.sort_by_key(|(_, t)| *t);
    while gens.len() > 5 {
        let (name, _) = gens.remove(0);
        let _ = fs::remove_dir_all(gen_root.join(name));
    }
}

#[cfg(test)]
mod tests {
    use super::{
        allocate_page_id, is_support, normalize_page_path, page_match, page_url, switch_current,
    };
    use std::collections::BTreeSet;
    use std::fs;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn underscore_names_are_support_material_not_pages() {
        assert!(!is_support("index.py"));
        assert!(!is_support("blog/post.py"));
        assert!(is_support("_lib/util.py"));
        assert!(is_support("_data/seed.json"));
        assert!(is_support("blog/_draft.py"));
        assert!(is_support("_notes.txt"));
    }

    #[test]
    fn match_prefixes_are_read_and_validated() {
        assert_eq!(page_match("x = 1", "a.py").unwrap(), None);
        assert_eq!(
            page_match("# comment\n# nac:match /item/\n#!py\n", "a.py")
                .unwrap()
                .as_deref(),
            Some("/item/")
        );
        // A comment that merely starts with the same words is not a directive.
        assert_eq!(page_match("# nac:matches /item/", "a.py").unwrap(), None);
        assert!(page_match("# nac:match item/", "a.py").is_err());
        assert!(page_match("# nac:match /item", "a.py").is_err());
        assert!(page_match("# nac:match\n", "a.py").is_err());
        // Pre-rendered output is a single frozen file, so it cannot answer a family.
        assert!(page_match("# nac:static\n# nac:match /item/\n", "a.py").is_err());
    }

    #[test]
    fn derives_paths_from_names_and_directories() {
        let site = Path::new("site");
        assert_eq!(page_url(site, Path::new("site/index.py"), "").unwrap(), "/");
        assert_eq!(page_url(site, Path::new("site/about.py"), "").unwrap(), "/about");
        assert_eq!(
            page_url(site, Path::new("site/blog/index.py"), "").unwrap(),
            "/blog/"
        );
        assert_eq!(
            page_url(site, Path::new("site/blog/post.py"), "").unwrap(),
            "/blog/post"
        );
        assert_eq!(
            page_url(site, Path::new("site/2.py"), "# nac:path /second").unwrap(),
            "/second"
        );
        assert_eq!(
            page_url(site, Path::new("site/2.py"), "# nac:path second/").unwrap(),
            "/second/"
        );
    }

    #[test]
    fn page_paths_are_normalized_and_checked() {
        assert_eq!(normalize_page_path("//a//b/.").unwrap(), "/a/b");
        assert_eq!(normalize_page_path("/a/b/").unwrap(), "/a/b/");
        assert!(normalize_page_path("/../x").is_err());
        assert!(normalize_page_path("/a?b").is_err());
        assert!(normalize_page_path("a/b").is_err());
    }

    #[test]
    fn generated_ids_avoid_numeric_pages() {
        let mut used = BTreeSet::new();
        used.insert(1000);
        let id = allocate_page_id("/about", &used).unwrap();
        assert!((1000..=u16::MAX).contains(&id));
        assert_ne!(id, 1000);
        assert_eq!(allocate_page_id("/about", &used), Some(id));
    }

    #[test]
    fn switches_current_and_leaves_no_temp_file() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("nac-current-{}-{stamp}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("current"), b"old").unwrap();

        switch_current(&dir, "new").unwrap();

        assert_eq!(fs::read_to_string(dir.join("current")).unwrap(), "new");
        assert!(!dir.join(".current.tmp").exists());
        fs::remove_dir_all(dir).ok();
    }
}
