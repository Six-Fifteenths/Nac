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

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = "nac.routes";
pub const EXAMPLE_NAME: &str = "nac.routes.example";
pub const DEFAULT_PORT: &str = "7000";

include!(concat!(env!("OUT_DIR"), "/embedded.rs"));

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TlsMode {
    /// Use TLS when the build carries a certificate pin, plaintext otherwise.
    #[default]
    Auto,
    /// Force TLS; fails if this build has no pin.
    On,
    /// Force plaintext. Needed for a loopback service reached through an SSH
    /// tunnel, which the client must not wrap in TLS.
    Off,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Site {
    pub name: String,
    pub endpoint: String,
    pub default: bool,
    pub tls: TlsMode,
    /// `ssh=user@host`: the operator build opens this tunnel itself. Only
    /// parsed for convenience; the distributed build has no code to act on it.
    pub ssh: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Routes {
    pub sites: Vec<Site>,
    /// The display suffix users type, e.g. `nac.local` in `shop.nac.local`.
    /// Arbitrary: it never reaches a real resolver, it only distinguishes a site
    /// name from a path. Set it with an `@domain` line.
    pub domain: String,
}

impl Default for Routes {
    fn default() -> Self {
        Self {
            sites: Vec::new(),
            domain: DEFAULT_DOMAIN.to_string(),
        }
    }
}

impl Routes {
    /// The suffix to show and accept. Never empty.
    pub fn domain(&self) -> &str {
        if self.domain.is_empty() {
            DEFAULT_DOMAIN
        } else {
            &self.domain
        }
    }

    /// Lets a deployment rename the suffix without rebuilding: the operator's
    /// `NAC_DOMAIN` wins over the file.
    pub fn apply_env_domain(&mut self) {
        if let Ok(value) = std::env::var("NAC_DOMAIN") {
            let value = value.trim();
            if !value.is_empty() {
                self.domain = value.to_string();
            }
        }
    }

    pub fn parse(text: &str) -> Self {
        let mut sites = Vec::new();
        let mut domain = DEFAULT_DOMAIN.to_string();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            // `@domain example.test` — a plain `.local` name collides with
            // mDNS on some networks, so deployments may want their own.
            if let Some(rest) = line.strip_prefix('@') {
                if let Some(value) = rest.split_whitespace().nth(1) {
                    if looks_like_domain(value) {
                        domain = value.to_ascii_lowercase();
                    }
                }
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
            // Remaining tokens are flags, in any order:
            //   tls | plain      how to encrypt
            //   ssh=user@host    the operator build opens a tunnel to it
            let mut tls = TlsMode::Auto;
            let mut ssh = None;
            for flag in parts {
                match flag {
                    "tls" => tls = TlsMode::On,
                    "plain" => tls = TlsMode::Off,
                    _ => {
                        if let Some(dest) = flag.strip_prefix("ssh=") {
                            if !dest.is_empty() {
                                ssh = Some(dest.to_string());
                            }
                        }
                    }
                }
            }
            sites.push(Site {
                name: name.to_string(),
                endpoint,
                default,
                tls,
                ssh,
            });
        }
        if !sites.iter().any(|s| s.default) {
            if let Some(first) = sites.first_mut() {
                first.default = true;
            }
        }
        Self { sites, domain }
    }

    pub fn single(name: &str, endpoint: &str) -> Self {
        Self {
            sites: vec![Site {
                name: name.to_string(),
                endpoint: with_default_port(endpoint.to_string()),
                default: true,
                tls: TlsMode::Auto,
                ssh: None,
            }],
            domain: DEFAULT_DOMAIN.to_string(),
        }
    }

    pub fn embedded() -> Option<Self> {
        if !has_embedded() {
            return None;
        }
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
        let exe = std::env::current_exe().ok();
        candidates(exe.as_deref()).into_iter().find(|p| p.is_file())
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

    /// Adds or replaces a site learned at runtime. A server answering `Resolve`
    /// is the only source of these, so the seeded table stays the trust anchor.
    pub fn learn(&mut self, name: &str, endpoint: &str, tls: TlsMode) {
        let endpoint = with_default_port(endpoint.to_string());
        match self.sites.iter_mut().find(|site| site.name == name) {
            Some(site) => {
                site.endpoint = endpoint;
                site.tls = tls;
            }
            None => self.sites.push(Site {
                name: name.to_string(),
                endpoint,
                default: false,
                tls,
                ssh: None,
            }),
        }
    }
}

/// Files to try, in order, for a viewer running at `exe`.
///
/// A dev build runs straight out of `target/<profile>/`, which never holds the
/// file, so the workspace root above it is searched as well. A distributed
/// build is not under `target/`, and nothing outside its own directory is
/// treated as a workspace.
fn candidates(exe: Option<&Path>) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(dir) = exe.and_then(Path::parent) {
        out.push(dir.join(FILE_NAME));
        out.push(dir.join(EXAMPLE_NAME));
        let target = dir.parent();
        if target.and_then(Path::file_name) == Some(OsStr::new("target")) {
            if let Some(root) = target.and_then(Path::parent) {
                out.push(root.join(FILE_NAME));
                out.push(root.join(EXAMPLE_NAME));
            }
        }
    }
    out.push(PathBuf::from(FILE_NAME));
    out.push(PathBuf::from(EXAMPLE_NAME));
    out
}

pub const DEFAULT_DOMAIN: &str = "nac.local";

/// A suffix has to be something a person could plausibly type as a host: labels
/// of letters, digits and dashes, separated by dots.
fn looks_like_domain(text: &str) -> bool {
    !text.is_empty()
        && text.contains('.')
        && text
            .split('.')
            .all(|label| !label.is_empty() && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
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
    fn dev_build_finds_routes_at_the_workspace_root() {
        let dirs = candidates(Some(Path::new("/repo/target/release/viewer.exe")));
        assert!(dirs.contains(&PathBuf::from("/repo/nac.routes")), "{dirs:?}");
        assert!(dirs.contains(&PathBuf::from("/repo/target/release/nac.routes")));
    }

    #[test]
    fn distributed_build_looks_no_further_than_its_own_directory() {
        let dirs = candidates(Some(Path::new("/opt/nac/viewer.exe")));
        assert!(dirs.contains(&PathBuf::from("/opt/nac/nac.routes")));
        assert!(!dirs.contains(&PathBuf::from("/opt/nac.routes")), "{dirs:?}");
        assert!(!dirs.contains(&PathBuf::from("/nac.routes")));
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
        assert_eq!(routes.sites[0].tls, TlsMode::Auto);
    }

    #[test]
    fn per_site_tls_mode_is_parsed() {
        let text = "\
*home   MTk5LjcuMTQwLjIxNzo3MDAw   tls
console MTI3LjAuMC4xOjcwMDE=        plain
local   MTI3LjAuMC4xOjcwMDA=
";
        let routes = Routes::parse(text);
        assert_eq!(routes.get("home").unwrap().tls, TlsMode::On);
        assert_eq!(routes.get("console").unwrap().tls, TlsMode::Off);
        assert_eq!(routes.get("local").unwrap().tls, TlsMode::Auto);
    }

    #[test]
    fn ssh_flag_is_optional_and_order_independent() {
        let text = "\
console MTI3LjAuMC4xOjcwMDE=  plain ssh=root@example.test
other   MTI3LjAuMC4xOjcwMDE=  ssh=root@example.test plain
plain   MTI3LjAuMC4xOjcwMDE=
";
        let routes = Routes::parse(text);
        let console = routes.get("console").unwrap();
        assert_eq!(console.tls, TlsMode::Off);
        assert_eq!(console.ssh.as_deref(), Some("root@example.test"));
        assert_eq!(routes.get("other").unwrap().ssh.as_deref(), Some("root@example.test"));
        assert_eq!(routes.get("plain").unwrap().ssh, None);
    }

    #[test]
    fn unknown_flag_falls_back_to_auto() {
        let routes = Routes::parse("home  MTI3LjAuMC4xOjcwMDA=  whatever\n");
        assert_eq!(routes.get("home").unwrap().tls, TlsMode::Auto);
    }
}
