//! Bakes two build-time values into the viewer: the site directory and the
//! certificate pin.
//!
//! Both used to come only from environment variables, which made a plain
//! `cargo build` quietly produce a client that speaks plaintext and therefore
//! cannot talk to a TLS server at all — a failure that looks like "cannot
//! connect" and gives no hint. Reading the pin from a file removes that trap.

use std::env;
use std::fs;
use std::path::PathBuf;

/// Name of the local pin file, looked up at the workspace root.
const PIN_FILE: &str = "nac.tls-pin";

fn main() {
    println!("cargo:rerun-if-env-changed=NAC_EMBED_ROUTES");
    println!("cargo:rerun-if-env-changed=NAC_TLS_PIN");
    println!("cargo:rerun-if-env-changed=NAC_TLS_PIN_FILE");
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));

    write_embedded(&out);
    write_pin(&out);
}

fn write_embedded(out: &PathBuf) {
    let routes = env::var("NAC_EMBED_ROUTES").unwrap_or_default();
    let source = if routes.trim().is_empty() {
        "pub fn has_embedded() -> bool { false }\npub fn embedded_text() -> Option<String> { None }\n"
            .to_string()
    } else {
        format!(
            "pub fn has_embedded() -> bool {{ true }}\npub fn embedded_text() -> Option<String> {{ Some(obfstr::obfstring!({:?})) }}\n",
            routes
        )
    };
    fs::write(out.join("embedded.rs"), source).expect("write embedded.rs");
}

fn write_pin(out: &PathBuf) {
    let found = find_pin();
    let pin = match &found {
        Some((text, source)) => match parse_pin(text) {
            Some(bytes) => Some(bytes),
            None => {
                println!(
                    "cargo:warning=证书 pin 格式不对（应为 64 位十六进制），来源：{source}。这份构建会使用明文。"
                );
                None
            }
        },
        None => None,
    };

    let source = match pin {
        Some(bytes) => {
            let list = bytes
                .iter()
                .map(|b| format!("0x{b:02x}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("pub fn pin() -> Option<[u8; 32]> {{ Some([{list}]) }}\n")
        }
        None => {
            println!(
                "cargo:warning=这份 viewer 没有证书 pin，连接加密站点会失败。请把服务器 /etc/ssl/nac/public.pin 的内容写进工作区根目录的 {PIN_FILE}，或设置 NAC_TLS_PIN。"
            );
            "pub fn pin() -> Option<[u8; 32]> { None }\n".to_string()
        }
    };
    fs::write(out.join("tls_pin.rs"), source).expect("write tls_pin.rs");
}

/// Resolution order: `NAC_TLS_PIN`, then `NAC_TLS_PIN_FILE`, then the workspace
/// root's `nac.tls-pin`, then one next to the crate. First non-empty wins.
fn find_pin() -> Option<(String, String)> {
    if let Ok(value) = env::var("NAC_TLS_PIN") {
        if !value.trim().is_empty() {
            return Some((value, "NAC_TLS_PIN".to_string()));
        }
    }

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(path) = env::var("NAC_TLS_PIN_FILE") {
        if !path.trim().is_empty() {
            candidates.push(PathBuf::from(path));
        }
    }
    candidates.push(manifest.join("..").join("..").join(PIN_FILE));
    candidates.push(manifest.join(PIN_FILE));

    for path in candidates {
        // Track it either way: creating the file later must trigger a rebuild.
        println!("cargo:rerun-if-changed={}", path.display());
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        if !text.trim().is_empty() {
            return Some((text, path.display().to_string()));
        }
    }
    None
}

fn parse_pin(text: &str) -> Option<[u8; 32]> {
    let text = text.trim();
    if text.len() != 64 || !text.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let mut bytes = [0u8; 32];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}