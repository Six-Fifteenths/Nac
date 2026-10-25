//! 地址：把一个地址栏里的字符串变成一个可以去取的地方。
//!
//! 三种输入 —— `名字`、`名字/路径`、`/路径` —— 都归到这里。规则见
//! PROTOCOL-DESIGN §3/§4：名字是一整串字符，没有层级、没有共用后缀。

use crate::*;


pub(crate) fn stash_fields(
    saved: &mut HashMap<u16, HashMap<String, String>>,
    page: Option<u16>,
    fields: &HashMap<String, String>,
) {
    if let Some(id) = page {
        if !fields.is_empty() {
            saved.insert(id, fields.clone());
        }
    }
}

/// Reads an address the way a browser does.
///
/// Accepted: `<domain>/`, `<domain>/about`, `about`, `/about`,
/// `home.<domain>/about`, and the legacy `home/7` / `nac://home/7`. The suffix
/// is `routes.domain()`, so a deployment can use a name of its own.
/// Endpoints are not accepted here on purpose — they live in the routes file,
/// so nothing the user types or sees has to contain one.
/// A typed address, before anyone has checked that the site is reachable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Address {
    /// Already in the table (seeded or learned earlier).
    Known { site: String, path: String },
    /// Written with our own suffix but not in the table yet. The caller asks a
    /// server where it lives instead of calling the address invalid.
    Unknown { site: String, path: String },
}

/// Where the reader was: a page on a site. Page numbers alone are not enough,
/// because they only mean something inside one site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub site: String,
    pub path: String,
}

/// The back/forward stacks. Kept apart from the UI so the rules — including the
/// ones that cross a site boundary — can be tested without opening a window.
#[derive(Default)]
pub struct Trail {
    back: Vec<Place>,
    ahead: Vec<Place>,
}

impl Trail {
    /// Remembers leaving `from`. Anything that was reachable with "forward" is
    /// dropped, the same way a browser starts a new branch.
    pub(crate) fn record(&mut self, from: Option<Place>) {
        if let Some(place) = from {
            self.back.push(place);
        }
        self.ahead.clear();
    }

    pub(crate) fn go_back(&mut self, current: Option<Place>) -> Option<Place> {
        let place = self.back.pop()?;
        if let Some(here) = current {
            self.ahead.push(here);
        }
        Some(place)
    }

    pub(crate) fn go_forward(&mut self, current: Option<Place>) -> Option<Place> {
        let place = self.ahead.pop()?;
        if let Some(here) = current {
            self.back.push(here);
        }
        Some(place)
    }

    pub(crate) fn can_back(&self) -> bool {
        !self.back.is_empty()
    }

    pub(crate) fn can_forward(&self) -> bool {
        !self.ahead.is_empty()
    }
}

/// Reads an address the way a browser does.
///
/// Accepted: `<host>/`, `<host>/about`, `about`, `/about`, and the legacy
/// `nac://<host>/7`. A **host is just a site's name** — there is no shared
/// suffix and no sub-domain rule, so `nac.local`, `iris.out`, `shop.nac.local`
/// and `EX_MYTH` are all equally valid and are matched by exact name (ignoring
/// case). Anything not in the table is treated as a name to look up, not as a
/// typo.
/// Endpoints are not accepted here on purpose — they live in the routes file,
/// so nothing the user types or sees has to contain one.
pub(crate) fn parse_address(input: &str, routes: &routes::Routes, current_site: &str) -> Option<Address> {
    let mut s = input.trim();
    if let Some((_, rest)) = s.split_once("://") {
        s = rest;
    }
    if let Some(rest) = s.strip_prefix("//") {
        s = rest;
    }
    let s = s.split(|c| c == '?' || c == '#').next().unwrap_or(s).trim();
    if s.is_empty() {
        return Some(Address::Known {
            site: current_site.to_string(),
            path: "/".to_string(),
        });
    }

    if !s.starts_with('/') {
        let (host, path) = match s.split_once('/') {
            Some((host, tail)) => (host, normalize_path(&format!("/{tail}"))?),
            None => (s, "/".to_string()),
        };
        if let Some(site) = routes.get(host) {
            return Some(Address::Known {
                site: site.name.clone(),
                path,
            });
        }
        // Pasting an endpoint is a mistake, not a path: refuse it rather than
        // turning the whole thing into a 404 on the current site.
        if host.contains(':') || host.contains('[') {
            return None;
        }
        if looks_like_host(host) {
            return Some(Address::Unknown {
                site: host.to_string(),
                path,
            });
        }
        return Some(Address::Known {
            site: current_site.to_string(),
            path: normalize_path(&format!("/{s}"))?,
        });
    }

    Some(Address::Known {
        site: current_site.to_string(),
        path: normalize_path(s)?,
    })
}

/// Whether a typed word should be looked up as a site name rather than read as a
/// path on the current site.
///
/// A name has to *look* like one: a dot or an underscore, the way `iris.out` and
/// `EX_MYTH` do. That keeps `about` a path, keeps `7` a path, and still lets a
/// site be called whatever the operator wants — including things no DNS would
/// accept. A colon means someone pasted an endpoint, which is never an address.
pub(crate) fn looks_like_host(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && !text.contains(':')
        && !text.chars().any(|c| c.is_whitespace() || c.is_control())
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        && (text.contains('.') || text.contains('_'))
}

pub(crate) fn normalize_path(input: &str) -> Option<String> {
    let mut parts = Vec::new();
    for part in input.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return None;
        }
        parts.push(part);
    }
    let mut out = format!("/{}", parts.join("/"));
    if input.ends_with('/') && out != "/" {
        out.push('/');
    }
    Some(out)
}

/// Hands an external address to the system browser.
///
/// The URL comes from a page, so it is treated as untrusted: the markup layer
/// already refused anything that is not http(s), and no shell is involved
/// (`rundll32` / `xdg-open` / `open` receive the URL as a single argument), so
/// there is nothing for a `&`, `|` or quote inside it to break out of.
pub(crate) fn open_in_browser(url: &str) {
    #[cfg(target_os = "windows")]
    let launched = std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn();
    #[cfg(target_os = "macos")]
    let launched = std::process::Command::new("open").arg(url).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let launched = std::process::Command::new("xdg-open").arg(url).spawn();
    if let Err(err) = launched {
        eprintln!("could not open {url}: {err}");
    }
}

/// Finds the page that answers `path`.
///
/// Exact addresses win. Failing that, a page may declare a prefix
/// (`# nac:match /item/`) and answer everything under it; the longest declared
/// prefix wins, so `/item/` and `/item/special/` can coexist.
#[derive(Debug, PartialEq, Eq)]
pub struct Resolved {
    pub id: u16,
    /// The address the page should be told about. For an exact match this is the
    /// entry's own path; for a prefix match it is the address actually asked for.
    pub path: String,
}

pub(crate) fn resolve_address(entries: &[net::Entry], home: Option<u16>, path: &str) -> Option<Resolved> {
    if let Some(entry) = entries.iter().find(|e| e.path == path) {
        return Some(Resolved {
            id: entry.id,
            path: entry.path.clone(),
        });
    }
    // A trailing slash and its absence should name the same directory page.
    let alt = if path.ends_with('/') {
        path.trim_end_matches('/').to_string()
    } else {
        format!("{path}/")
    };
    if !alt.is_empty() && alt != "/" {
        if let Some(entry) = entries.iter().find(|e| e.path == alt) {
            return Some(Resolved {
                id: entry.id,
                path: path.to_string(),
            });
        }
    }
    let best = entries
        .iter()
        .filter_map(|e| e.matches.as_deref().map(|prefix| (e, prefix)))
        .filter(|(_, prefix)| path.starts_with(prefix))
        .max_by_key(|(_, prefix)| prefix.len());
    if let Some((entry, _)) = best {
        return Some(Resolved {
            id: entry.id,
            path: path.to_string(),
        });
    }
    if path == "/" {
        return home.map(|id| Resolved {
            id,
            path: "/".to_string(),
        });
    }
    None
}

#[cfg(test)]
pub(crate) fn resolve_path(entries: &[net::Entry], home: Option<u16>, path: &str) -> Option<u16> {
    resolve_address(entries, home, path).map(|r| r.id)
}




pub(crate) enum Target {
    Page(u16),
    Path(String),
}
