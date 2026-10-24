//! Site directory: maps a user-visible name to a server endpoint.
//!
//! Users address pages as `/12`; they never see or type an endpoint. The
//! endpoint normally lives in a file outside the binary, and is stored base64
//! encoded so the file does not read as a plain address at a glance. It can
//! also be embedded at build time with `NAC_EMBED_ROUTES` for standalone
//! distribution.
//!
//! That is all it is: a glance-level disguise. A client that speaks TCP
//! directly to a server must know where that server is, so anything stronger
//! (relay, Tor) belongs to the transport layer, not here.

use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "nac.routes";
pub const EXAMPLE_NAME: &str = "nac.routes.example";
pub const DEFAULT_PORT: &str = "7000";

include!(concat!(env!("OUT_DIR"), "/embedded.rs"));

pub fn is_embedded() -> bool {
    has_embedded()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Site {
    pub name: String,
    pub endpoint: String,
    pub default: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Routes {
    pub sites: Vec<Site>,
}

impl Routes {
    pub fn parse(text: &str) -> Self {
        let mut sites = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (default, line) = match line.strip_prefix('*') {
                Some(rest) => (true, rest.trim()),
                None => (false, line),
            };
            let mut parts = line.split_whitespace();
            let (Some(name), Some(token)) = (parts.next(), parts.next()) else {
                continue;
            };
            let endpoint = match decode(token) {
                Some(endpoint) => endpoint,
                None => token.to_string(),
            };
            let endpoint = with_default_port(endpoint);
            sites.push(Site {
                name: name.to_string(),
                endpoint,
                default,
            });
        }
        if !sites.iter().any(|s| s.default) {
            if let Some(first) = sites.first_mut() {
                first.default = true;
            }
        }
        Self { sites }
    }

    pub fn single(name: &str, endpoint: &str) -> Self {
        Self {
            sites: vec![Site {
                name: name.to_string(),
                endpoint: with_default_port(endpoint.to_string()),
                default: true,
            }],
        }
    }

    pub fn embedded() -> Option<Self> {
        let text = embedded_text()?;
        let routes = Self::parse(&text);
        if routes.is_empty() {
            None
        } else {
            Some(routes)
        }
    }

    pub fn get(&self, name: &str) -> Option<&Site> {
        self.sites.iter().find(|s| s.name == name)
    }

    pub fn default_site(&self) -> Option<&Site> {
        self.sites
            .iter()
            .find(|s| s.default)
            .or_else(|| self.sites.first())
    }

    pub fn is_empty(&self) -> bool {
        self.sites.is_empty()
    }

    /// Looks for the directory next to the executable, then in the working
    /// directory, unless a path was given explicitly.
    pub fn locate(explicit: Option<&str>) -> Option<PathBuf> {
        if let Some(path) = explicit {
            return Some(PathBuf::from(path));
        }
        if let Ok(path) = std::env::var("NAC_ROUTES") {
            if !path.is_empty() {
                return Some(PathBuf::from(path));
            }
        }
        let mut candidates = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                candidates.push(dir.join(FILE_NAME));
                candidates.push(dir.join(EXAMPLE_NAME));
            }
        }
        candidates.push(PathBuf::from(FILE_NAME));
        candidates.push(PathBuf::from(EXAMPLE_NAME));
        candidates.into_iter().find(|p| p.is_file())
    }

    pub fn load(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        let routes = Self::parse(&text);
        if routes.is_empty() {
            None
        } else {
            Some(routes)
        }
    }
}

fn with_default_port(endpoint: String) -> String {
    if endpoint.contains(':') {
        endpoint
    } else {
        format!("{endpoint}:{DEFAULT_PORT}")
    }
}

const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn encode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

pub fn decode(input: &str) -> Option<String> {
    let mut acc: u32 = 0;
    let mut bits = 0;
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut seen = 0;
    for byte in input.bytes() {
        if byte == b'=' {
            break;
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32;
        acc = (acc << 6) | value;
        bits += 6;
        seen += 1;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    if seen == 0 || seen % 4 == 1 || out.is_empty() {
        return None;
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_roundtrip() {
        for text in ["127.0.0.1:7000", "a", "ab", "abc", "abcd", "示例.test:9000"] {
            let token = encode(text);
            assert_eq!(decode(&token).as_deref(), Some(text), "token={token}");
        }
    }

    #[test]
    fn parses_named_sites() {
        let text = "\
# 站点目录
site.test:9000
*home   MTI3LjAuMC4xOjcwMDA=
";
        let routes = Routes::parse(text);
        assert_eq!(routes.sites.len(), 1);
        assert_eq!(routes.sites[0].name, "home");
        assert_eq!(routes.sites[0].endpoint, "127.0.0.1:7000");
        assert!(routes.sites[0].default);
    }

    #[test]
    fn first_site_becomes_default_and_port_is_filled_in() {
        let routes = Routes::parse("only  example.test\n");
        assert_eq!(routes.default_site().map(|s| s.name.as_str()), Some("only"));
        assert_eq!(routes.sites[0].endpoint, "example.test:7000");
    }
}
