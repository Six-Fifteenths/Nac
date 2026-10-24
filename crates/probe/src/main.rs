//! probe — minimal smoke client for the wire format.
//!
//! usage:
//!   probe manifest
//!   probe page <id> [session-hex]
//!   probe image <id> [session-hex]
//!   probe submit <id> key=value [key=value ...]

use std::io::{Read, Write};
use std::net::TcpStream;

use protocol::{
    decode_server, encode_client, ClientMsg, Request, ServerMsg, SESSION_LEN,
};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut addr = "127.0.0.1:7000".to_string();
    if args.len() >= 2 && args[0] == "--addr" {
        addr = args[1].clone();
        args.drain(0..2);
    }
    if args.is_empty() {
        eprintln!("usage: probe [--addr host:port] manifest|page <id>|image <id>|submit <id> k=v...");
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

    let cmd = args.remove(0);
    let result: Result<(), String> = match cmd.as_str() {
        "manifest" => call(&addr, ClientMsg { session, request: Request::Manifest })
            .and_then(|msgs| print_msgs(msgs, 0, false)),
        "hello" => call(&addr, ClientMsg { session, request: Request::Hello })
            .and_then(|msgs| print_msgs(msgs, 0, false)),
        "health" => call(&addr, ClientMsg { session, request: Request::Manifest })
            .and_then(|msgs| match msgs.first() {
                Some(ServerMsg::Manifest { entries, .. }) if !entries.is_empty() => {
                    println!("ok {}", entries.len());
                    Ok(())
                }
                _ => Err("empty manifest".into()),
            }),
        "page" => {
            let id = parse_id(&args);
            fetch(&addr, session, id, false)
        }
        "image" => {
            let id = parse_id(&args);
            fetch(&addr, session, id, true)
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

fn call(addr: &str, msg: ClientMsg) -> Result<Vec<ServerMsg>, String> {
    let mut stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;
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

fn fetch(addr: &str, session: [u8; SESSION_LEN], id: u16, image: bool) -> Result<(), String> {
    let mut offset: u32 = 0;
    let mut body = Vec::new();
    loop {
        let req = if image {
            Request::Image(id, offset)
        } else {
            Request::Page(id)
        };
        let msgs = call(addr, ClientMsg { session, request: req })?;
        match msgs.into_iter().next() {
            Some(ServerMsg::Page { total, data, .. }) | Some(ServerMsg::Image { total, data, .. }) => {
                body.extend_from_slice(&data);
                offset = body.len() as u32;
                if body.len() >= total as usize || data.is_empty() {
                    break;
                }
            }
            Some(ServerMsg::Empty) => {
                println!("EMPTY");
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
    let mut stream = TcpStream::connect(addr).map_err(|e| e.to_string())?;
    let msg = ClientMsg {
        session,
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
            ServerMsg::Empty => {
                println!("EMPTY");
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
                        "  {}\t{}\t{:016x}\t{}",
                        e.id,
                        if e.dynamic { "dyn " } else { "stat" },
                        e.hash,
                        e.title
                    );
                }
            }
            ServerMsg::Page { data, .. } => {
                print!("{}", String::from_utf8_lossy(&data));
            }
            ServerMsg::Image { total, .. } => println!("image {id}: {total} bytes"),
            ServerMsg::Hello { version } => println!("version {version}"),
            ServerMsg::Empty => println!("EMPTY"),
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

fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>, String> {
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
