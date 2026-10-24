use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=NAC_EMBED_ROUTES");
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
}
