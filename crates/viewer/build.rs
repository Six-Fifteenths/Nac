use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=NAC_EMBED_ROUTES");
    println!("cargo:rerun-if-env-changed=NAC_TLS_PIN");
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let routes = env::var("NAC_EMBED_ROUTES").unwrap_or_default();
    let source = if routes.is_empty() {
        "pub fn has_embedded() -> bool { false }\npub fn embedded_text() -> Option<String> { None }\n"
            .to_string()
    } else {
        format!(
            "pub fn has_embedded() -> bool {{ true }}\npub fn embedded_text() -> Option<String> {{ Some(obfstr::obfstring!({routes:?})) }}\n"
        )
    };
    fs::write(out.join("embedded.rs"), source).expect("write embedded.rs");

    let pin = env::var("NAC_TLS_PIN").unwrap_or_default();
    let tls_source = if pin.len() == 64 && pin.chars().all(|c| c.is_ascii_hexdigit()) {
        let mut bytes = Vec::with_capacity(32);
        for i in 0..32 {
            bytes.push(u8::from_str_radix(&pin[i * 2..i * 2 + 2], 16).expect("pin hex"));
        }
        let list = bytes
            .iter()
            .map(|b| format!("0x{b:02x}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("pub fn pin() -> Option<[u8; 32]> {{ Some([{list}]) }}\n")
    } else {
        "pub fn pin() -> Option<[u8; 32]> { None }\n".to_string()
    };
    fs::write(out.join("tls_pin.rs"), tls_source).expect("write tls_pin.rs");
}
