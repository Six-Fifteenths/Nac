//! probe — minimal smoke client for the wire format.
//!
//! usage:
//!   probe manifest
//!   probe page <id> [session-hex]
//!   probe image <id> [session-hex]
//!   probe submit <id> key=value [key=value ...]

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, OnceLock};

use protocol::{
    decode_server, encode_client, ClientMsg, Request, ServerMsg, SESSION_LEN,
};
use tls_client::{ReadWrite, TlsClientConfig};

static TLS_CONFIG: OnceLock<Option<Arc<TlsClientConfig>>> = OnceLock::new();

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut addr = "127.0.0.1:7000".to_string();
    if args.len() >= 2 && args[0] == "--tls-pin-file" {
        let path = args[1].clone();
        args.drain(0..2);
        let text = std::fs::read_to_string(&path).map_err(|e| e.to_string());
        let text = text.unwrap_or_else(|e| { eprintln!("tls pin: {e}"); std::process::exit(1) });
        let text = text.trim();
        if text.len() != 64 || !text.chars().all(|c| c.is_ascii_hexdigit()) {
            eprintln!("tls pin: expected 64 hex chars in {path}");
            std::process::exit(1);
        }
        let mut bytes = [0u8; 32];
        for i in 0..32 {
            bytes[i] = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).unwrap();
        }
        TLS_CONFIG.set(Some(tls_client::client_config(bytes))).ok();
    }
    if args.len() >= 2 && args[0] == "--addr" {
        addr = args[1].clone();
        args.drain(0..2);
    }
    let _ = TLS_CONFIG.get();
    if args.is_empty() {
        eprintln!(
            "usage: probe [--tls-pin-file pin] [--addr host:port] [--path /地址] \
             manifest|hello|health|page <id>|image <id>|image-stream <id>\
             |submit <id> k=v...|resolve <名字>"
        );
        std::process::exit(2);
    }

    let session = args
        .iter()
        .position(|a| a.len() == 32 && a.chars().all(|c| c.is_ascii_hexdigit()))
        .map(|i| {
            let s = args.remove(i);
            parse_session(&s)
        })
        .unwrap_or([0u8; SESSION_LEN]);

    // Optional `--path /address` so a page answering a prefix can be probed.
    let mut path = String::new();
    if let Some(i) = args.iter().position(|a| a == "--path") {
        if i + 1 < args.len() {
            path = args.remove(i + 1);
            args.remove(i);
        }
    }

    let cmd = args.remove(0);
    let result: Result<(), String> = match cmd.as_str() {
        "manifest" => call(&addr, ClientMsg { session, path: path.clone(), request: Request::Manifest })
            .and_then(|msgs| print_msgs(msgs, 0, false)),
        "hello" => call(&addr, ClientMsg { session, path: path.clone(), request: Request::Hello })
            .and_then(|msgs| print_msgs(msgs, 0, false)),
        "health" => call(&addr, ClientMsg { session, path: path.clone(), request: Request::Manifest })
            .and_then(|msgs| match msgs.first() {
                Some(ServerMsg::Manifest { entries, .. }) if !entries.is_empty() => {
                    println!("ok {}", entries.len());
                    Ok(())
                }
                _ => Err("empty manifest".into()),
            }),
        // `probe resolve shop` — the ops way to check a registry entry: this is
        // exactly the question a client asks when it meets an unknown name.
        "resolve" => {
            let name = args.first().cloned().unwrap_or_default();
            if name.is_empty() {
                Err("resolve 需要站点名".to_string())
            } else {
                call(
                    &addr,
                    ClientMsg {
                        session,
                        path: path.clone(),
                        request: Request::Resolve(name),
                    },
                )
                .and_then(|msgs| match msgs.first() {
                    Some(ServerMsg::Resolved {
                        name,
                        endpoint,
                        tls,
                        title,
                    }) => {
                        println!(
                            "{name} -> {endpoint}  {}  {title}",
                            if *tls { "tls" } else { "plain" }
                        );
                        Ok(())
                    }
                    _ => Err("这个站它不认识（EMPTY）".to_string()),
                })
            }
        }
        "page" => {
            let id = parse_id(&args);
            fetch(&addr, session, id, false, &path)
        }
        "image" => {
            let id = parse_id(&args);
            fetch(&addr, session, id, true, &path)
        }
        "image-stream" => {
            let id = parse_id(&args);
            fetch_stream(&addr, session, id)
        }
        "submit" => {
            let id = parse_id(&args);
            let fields: Vec<(String, String)> = args
                .iter()
                .filter_map(|kv| kv.split_once('='))
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            call(
                &addr,
                ClientMsg {
                    session,
                    path: path.clone(),
                    request: Request::Submit(id, fields),
                },
            )
            .and_then(|msgs| print_msgs(msgs, id, false))
        }
        other => {
            eprintln!("unknown command: {other}");
            std::process::exit(2);
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn parse_id(args: &[String]) -> u16 {
    args.first()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
            eprintln!("missing page id");
            std::process::exit(2);
        })
}

fn parse_session(hex: &str) -> [u8; SESSION_LEN] {
    let mut out = [0u8; SESSION_LEN];
    for i in 0..SESSION_LEN {
        out[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap_or(0);
    }
    out
}

fn connect(addr: &str) -> Result<Box<dyn ReadWrite>, String> {
    let stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;
    let config = TLS_CONFIG.get().cloned().flatten();
    tls_client::wrap(stream, config).map_err(|_| "TLS handshake setup failed".to_string())
}

fn call(addr: &str, msg: ClientMsg) -> Result<Vec<ServerMsg>, String> {
    let mut stream = connect(addr)?;
    stream.write_all(&encode_client(&msg)).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    loop {
        let frame = read_frame(&mut stream)?;
        let msg = decode_server(&frame).ok_or_else(|| "bad response frame".to_string())?;
        let done = !matches!(msg, ServerMsg::Page { .. } | ServerMsg::Image { .. });
        out.push(msg);
        if done {
            break;
        }
        // A single chunk is enough for the smoke test.
        break;
    }
    Ok(out)
}

fn fetch(
    addr: &str,
    session: [u8; SESSION_LEN],
    id: u16,
    image: bool,
    path: &str,
) -> Result<(), String> {
    let mut offset: u32 = 0;
    let mut body = Vec::new();
    loop {
        let req = if image {
            Request::Image(id, offset)
        } else {
            Request::Page(id)
        };
        let msgs = call(addr, ClientMsg { session, path: path.to_string(), request: req })?;
        match msgs.into_iter().next() {
            Some(ServerMsg::Page { total, data, .. }) | Some(ServerMsg::Image { total, data, .. }) => {
                body.extend_from_slice(&data);
                offset = body.len() as u32;
                if body.len() >= total as usize || data.is_empty() {
                    break;
                }
            }
            Some(ServerMsg::Empty(reason)) => {
                println!("EMPTY ({reason:?})");
                return Ok(());
            }
            other => return Err(format!("unexpected: {other:?}")),
        }
    }
    if image {
        println!("image {id}: {} bytes", body.len());
    } else {
        print!("{}", String::from_utf8_lossy(&body));
    }
    Ok(())
}

fn fetch_stream(addr: &str, session: [u8; SESSION_LEN], id: u16) -> Result<(), String> {
    let mut stream = connect(addr)?;
    let msg = ClientMsg {
        session,
        path: String::new(),
        request: Request::ImageStream(id, 0),
    };
    stream.write_all(&encode_client(&msg)).map_err(|e| e.to_string())?;
    let mut body = Vec::new();
    loop {
        let frame = read_frame(&mut stream)?;
        match decode_server(&frame).ok_or_else(|| "bad response frame".to_string())? {
            ServerMsg::Image { total, data, .. } => {
                body.extend_from_slice(&data);
                if body.len() >= total as usize || data.is_empty() {
                    break;
                }
            }
            ServerMsg::Empty(reason) => {
                println!("EMPTY ({reason:?})");
                return Ok(());
            }
            other => return Err(format!("unexpected: {other:?}")),
        }
    }
    println!("image {id}: {} bytes", body.len());
    Ok(())
}

fn print_msgs(msgs: Vec<ServerMsg>, id: u16, _image: bool) -> Result<(), String> {
    for m in msgs {
        match m {
            ServerMsg::Manifest { session, entries } => {
                println!("session {}", hex(&session));
                for e in entries {
                    println!(
                        "  {}\t{}\t{:016x}\t{}\t{}",
                        e.id,
                        if e.dynamic { "dyn " } else { "stat" },
                        e.hash,
                        e.path,
                        e.title
                    );
                }
            }
            ServerMsg::Page { data, .. } => {
                print!("{}", String::from_utf8_lossy(&data));
            }
            ServerMsg::Image { total, .. } => println!("image {id}: {total} bytes"),
            ServerMsg::Hello { version, site } => println!("version {version}  site {site}"),
            ServerMsg::Resolved {
                name,
                endpoint,
                tls,
                title,
            } => println!("{name} -> {endpoint} tls={tls} {title}"),
            ServerMsg::Empty(reason) => println!("EMPTY ({reason:?})"),
        }
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::new();
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn read_frame<R: Read + ?Sized>(stream: &mut R) -> Result<Vec<u8>, String> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).map_err(|e| e.to_string())?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).map_err(|e| e.to_string())?;
    let mut frame = Vec::with_capacity(4 + len);
    frame.extend_from_slice(&len_buf);
    frame.extend_from_slice(&buf);
    Ok(frame)
}
