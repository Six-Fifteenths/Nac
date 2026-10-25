#![cfg_attr(
    all(target_os = "windows", feature = "windows_gui"),
    windows_subsystem = "windows"
)]

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Color32, FontFamily, FontId, Layout, RichText};

mod fetch;
mod loader;
mod markup;
mod net;
mod routes;
mod tls;
#[cfg(feature = "admin")]
mod tunnel;

use markup::{Block, Inline, LinkTarget, NoteKind};

/// A windowed build has no console, so a panic at startup used to look exactly
/// like "double-clicking does nothing": the process vanished and said nothing.
/// Write the reason next to the crash and put it on screen once.
fn report_panics() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static SHOWN: AtomicBool = AtomicBool::new(false);

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let text = format!("{info}");
        eprintln!("{text}");
        if SHOWN.swap(true, Ordering::SeqCst) {
            previous(info);
            return;
        }
        let path = std::env::temp_dir().join("nac-viewer-crash.txt");
        let _ = std::fs::write(&path, &text);
        let message = format!(
            "Nac 客户端遇到了内部错误，无法继续。\n\n{text}\n\n详细信息已写入：\n{}",
            path.display()
        );
        alert(&message);
        previous(info);
    }));
}

/// A modal box, so the message is visible even with no console attached.
#[cfg(target_os = "windows")]
fn alert(text: &str) {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            owner: *mut std::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            kind: u32,
        ) -> i32;
    }

    let wide = |s: &str| -> Vec<u16> {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    };
    let body = wide(text);
    let title = wide("Nac");
    const MB_ICONERROR: u32 = 0x10;
    // SAFETY: both strings are NUL-terminated and outlive the call.
    unsafe {
        MessageBoxW(
            ptr::null_mut(),
            body.as_ptr(),
            title.as_ptr(),
            MB_ICONERROR,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn alert(text: &str) {
    eprintln!("{text}");
}

/// What clicking a tab did.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TabHit {
    None,
    Switch,
    Close,
}

/// One tab: a rounded chip with the page title, and a `×` when it can be closed.
///
/// The title is capped so one long page does not push every other tab sideways;
/// middle-click closes it, the way a browser does.
fn tab_chip(ui: &mut egui::Ui, tab: &Tab, active: bool, can_close: bool) -> TabHit {
    let title = if tab.title.is_empty() {
        tab.address_text()
    } else {
        tab.title.clone()
    };
    let label = shorten(&title, 20);
    let mut hit = TabHit::None;

    let fill = if active {
        Color32::from_rgb(233, 238, 250)
    } else {
        Color32::from_rgb(245, 245, 248)
    };
    let stroke = if active {
        egui::Stroke::new(1.0, Color32::from_rgb(190, 202, 232))
    } else {
        egui::Stroke::new(1.0, Color32::from_rgb(232, 232, 236))
    };

    egui::Frame::new()
        .fill(fill)
        .stroke(stroke)
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(9, 4))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let name = ui.add(
                    egui::Label::new(
                        RichText::new(label)
                            .size(13.5)
                            .color(if active {
                                Color32::from_rgb(30, 40, 70)
                            } else {
                                Color32::from_rgb(90, 95, 105)
                            }),
                    )
                    .sense(egui::Sense::click()),
                );
                if name.clicked() {
                    hit = TabHit::Switch;
                }
                if name.middle_clicked() && can_close {
                    hit = TabHit::Close;
                }
                name.on_hover_text(title);

                if can_close {
                    let close = ui.add(
                        egui::Label::new(
                            RichText::new("×")
                                .size(13.0)
                                .color(Color32::from_rgb(150, 155, 165)),
                        )
                        .sense(egui::Sense::click()),
                    );
                    if close.clicked() {
                        hit = TabHit::Close;
                    }
                    close.on_hover_text("关闭标签页  (Ctrl+W)");
                }
            });
        });
    hit
}

/// Whether the address bar should be rewritten from the tab's own address.
///
/// The rule that used to be missing: text the reader typed is never thrown away.
/// While an address committed from the bar is still loading, the bar keeps what
/// was typed; once it lands, the bar shows where it actually ended up.
fn should_sync_address(
    focused: bool,
    holding: bool,
    edited: bool,
    shown: &str,
    want: &str,
) -> bool {
    shown != want && !holding && (!focused || !edited)
}

/// Caps a title so a narrow window still fits a few tabs. Counts characters, not
/// bytes, so a Chinese title is cut at a sensible place.
fn shorten(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut out: String = text.chars().take(limit.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// A hairline under the tab strip, so the strip reads as one and the toolbar
/// below does not float in the page.
fn strip_line(ui: &egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        rect.x_range(),
        rect.top(),
        egui::Stroke::new(1.0, Color32::from_rgb(226, 226, 232)),
    );
}

fn main() -> eframe::Result<()> {
    report_panics();

    let mut addr: Option<String> = std::env::var("NAC_ADDR").ok();
    let mut routes_path: Option<String> = None;
    let mut site_name: Option<String> = None;
    let mut selftest = false;
    let mut encode: Option<String> = None;
    let mut list_sites = false;
    let mut start_page: Option<u16> = None;
    let mut start_path: Option<String> = None;
    let mut plaintext = false;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--addr" => {
                if let Some(v) = args.get(i + 1) {
                    addr = Some(v.clone());
                    i += 1;
                }
            }
            "--routes" => {
                if let Some(v) = args.get(i + 1) {
                    routes_path = Some(v.clone());
                    i += 1;
                }
            }
            "--site" => {
                if let Some(v) = args.get(i + 1) {
                    site_name = Some(v.clone());
                    i += 1;
                }
            }
            "--routes-encode" => {
                if let Some(v) = args.get(i + 1) {
                    encode = Some(v.clone());
                    i += 1;
                }
            }
            "--routes-check" => list_sites = true,
            "--page" => {
                if let Some(v) = args.get(i + 1) {
                    start_page = v.parse::<u16>().ok();
                    i += 1;
                }
            }
            "--selftest" => selftest = true,
            // `--at /hn/item/12`: open a deep address, which a page number alone
            // cannot express once one page answers a whole family of addresses.
            "--at" => {
                if let Some(v) = args.get(i + 1) {
                    start_path = Some(v.clone());
                    i += 1;
                }
            }
            "--plaintext" => plaintext = true,
            _ => {}
        }
        i += 1;
    }

    if plaintext {
        tls::disable_plaintext();
    }

    if let Some(value) = encode {
        println!("{}", routes::encode(&value));
        return Ok(());
    }

    let (routes, from_embedded) = build_routes(routes_path.as_deref(), addr);
    // The SNI name is cosmetic here (the certificate is pinned, not name-checked),
    // so the default site's name is good enough.
    tls::set_server_name(
        routes
            .default_site()
            .map(|site| site.name.as_str())
            .unwrap_or(routes::DEFAULT_DOMAIN),
    );
    // A site marked `tls` cannot work without a pin, and the failure would
    // otherwise show up only as "cannot connect". Say it once, up front.
    if !plaintext && !tls::has_pin() {
        let forced: Vec<&str> = routes
            .sites
            .iter()
            .filter(|s| s.tls == routes::TlsMode::On)
            .map(|s| s.name.as_str())
            .collect();
        if !forced.is_empty() {
            eprintln!(
                "注意：站点 {} 要求 TLS，但这份构建没有证书 pin（NAC_TLS_PIN）——这些站点会连不上。",
                forced.join(", ")
            );
        }
    }

    if list_sites {
        for site in &routes.sites {
            let mode = match site.tls {
                routes::TlsMode::Auto => "auto",
                routes::TlsMode::On => "tls",
                routes::TlsMode::Off => "plain",
            };
            let tunnel = match &site.ssh {
                Some(dest) if cfg!(feature = "admin") => format!("\tssh={dest}"),
                Some(_) => "\tssh（本构建不支持）".to_string(),
                None => String::new(),
            };
            if from_embedded {
                println!(
                    "{}{}\t{}{}",
                    if site.default { "*" } else { " " },
                    site.name,
                    mode,
                    tunnel
                );
            } else {
                println!(
                    "{}{}\t{}\t{}{}",
                    if site.default { "*" } else { " " },
                    site.name,
                    site.endpoint,
                    mode,
                    tunnel
                );
            }
        }
        return Ok(());
    }

    let site = match site_name {
        Some(name) if routes.get(&name).is_some() => name,
        Some(name) => {
            eprintln!("没有名为 {name} 的站点");
            return Ok(());
        }
        None => routes
            .default_site()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "dev".to_string()),
    };

    if selftest {
        return selftest_run(&routes, &site);
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 760.0])
            .with_min_inner_size([560.0, 420.0])
            .with_title("nac"),
        ..Default::default()
    };

    eframe::run_native(
        "nac",
        options,
        Box::new(move |cc| {
            Ok(Box::new(Viewer::new(
                cc,
                routes,
                site,
                start_path.map(Target::Path).or(start_page.map(Target::Page)),
            )))
        }),
    )
}

/// Resolution order: `--addr`, then an explicit `--routes <path>`, then an
/// endpoint embedded at build time via `NAC_EMBED_ROUTES`, then whatever
/// directory the usual lookup finds. The explicit flag has to win over the
/// embedded copy, otherwise a standalone build silently ignores it.
///
/// The second value says whether the result came from the embedded copy, which
/// is what decides if endpoints are printed: a standalone build should not
/// print them, but an operator who passed `--routes` asked for the file.
fn build_routes(explicit: Option<&str>, addr: Option<String>) -> (routes::Routes, bool) {
    if let Some(fallback) = addr {
        return (routes::Routes::single("dev", &fallback), false);
    }
    if let Some(path) = explicit {
        match routes::Routes::load(std::path::Path::new(path)) {
            Some(routes) => return (routes, false),
            None => eprintln!("无法读取站点目录 {path}"),
        }
    }
    if let Some(routes) = routes::Routes::embedded() {
        return (routes, true);
    }
    if let Some(path) = routes::Routes::locate(None) {
        if let Some(routes) = routes::Routes::load(&path) {
            return (routes, false);
        }
        eprintln!("无法读取站点目录 {}", path.display());
    }
    (routes::Routes::default(), false)
}

fn selftest_run(routes: &routes::Routes, site_name: &str) -> eframe::Result<()> {
    let Some(site) = routes.get(site_name) else {
        println!("site {site_name}: missing");
        return Ok(());
    };
    // Keep the tunnel alive for the whole run; dropping it kills `ssh`.
    #[cfg(feature = "admin")]
    let _tunnel = match tunnel::Tunnel::ensure(site) {
        Ok(tunnel) => tunnel,
        Err(message) => {
            println!("tunnel: {message}");
            return Ok(());
        }
    };
    let mut client = net::Client::with_tls(site.endpoint.clone(), site.tls);
    match client.hello() {
        Ok((version, name)) => println!("protocol: {version}  site: {name}"),
        Err(()) => println!("protocol: legacy"),
    }
    match client.manifest() {
        Ok(entries) => {
            println!("site {site_name}: {} entries", entries.len());
            for e in &entries {
                println!(
                    "  {}\t{}\t{:016x}\t{}\t{}",
                    e.id,
                    if e.dynamic { "dyn " } else { "stat" },
                    e.hash,
                    e.path,
                    e.title
                );
            }
            if let Some(id) = entries.iter().map(|e| e.id).min() {
                match client.page(id, "") {
                    Ok(text) => {
                        let blocks = markup::parse(&text);
                        println!("page {id}: {} bytes, {} blocks", text.len(), blocks.len());
                        print!("{text}");
                    }
                    Err(_) => println!("page {id}: EMPTY"),
                }
            }
            match client.image(1) {
                Ok(bytes) => println!("image 1: {} bytes", bytes.len()),
                Err(_) => println!("image 1: EMPTY"),
            }
            match client.submit(7, vec![("name".to_string(), "selftest".to_string())], "") {
                Ok((id, text)) => println!("submit 7 -> page {id}: {} bytes", text.len()),
                Err(_) => println!("submit 7: EMPTY"),
            }
        }
        Err(_) => println!("site {site_name}: EMPTY"),
    }
    Ok(())
}

// ------------------------------------------------------------------- fonts

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let regular = load_first(&[
        "C:/Windows/Fonts/Deng.ttf",
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/simkai.ttf",
        "/System/Library/Fonts/PingFang.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    ]);
    let bold = load_first(&[
        "C:/Windows/Fonts/Dengb.ttf",
        "C:/Windows/Fonts/msyhbd.ttc",
        "C:/Windows/Fonts/simhei.ttf",
    ]);

    let mut proportional = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mut monospace = fonts
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    let mut regular_name = None;
    if let Some(bytes) = regular {
        let name = "cjk".to_string();
        fonts
            .font_data
            .insert(name.clone(), Arc::new(egui::FontData::from_owned(bytes)));
        proportional.insert(0, name.clone());
        monospace.push(name.clone());
        regular_name = Some(name);
    }

    fonts
        .families
        .insert(FontFamily::Proportional, proportional.clone());
    fonts.families.insert(FontFamily::Monospace, monospace);

    let mut bold_family = Vec::new();
    if let Some(bytes) = bold {
        let name = "cjk-bold".to_string();
        fonts
            .font_data
            .insert(name.clone(), Arc::new(egui::FontData::from_owned(bytes)));
        bold_family.push(name);
    }
    if let Some(name) = regular_name {
        bold_family.push(name);
    }
    if bold_family.is_empty() {
        bold_family = proportional;
    }
    fonts
        .families
        .insert(FontFamily::Name(Arc::from("bold")), bold_family);

    ctx.set_fonts(fonts);
}

fn load_first(paths: &[&str]) -> Option<Vec<u8>> {
    paths
        .iter()
        .find_map(|p| std::fs::read(p).ok())
        .filter(|b| !b.is_empty())
}

/// Keeps what the user typed so going back to a page restores its form.
fn stash_fields(
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
    fn record(&mut self, from: Option<Place>) {
        if let Some(place) = from {
            self.back.push(place);
        }
        self.ahead.clear();
    }

    fn go_back(&mut self, current: Option<Place>) -> Option<Place> {
        let place = self.back.pop()?;
        if let Some(here) = current {
            self.ahead.push(here);
        }
        Some(place)
    }

    fn go_forward(&mut self, current: Option<Place>) -> Option<Place> {
        let place = self.ahead.pop()?;
        if let Some(here) = current {
            self.back.push(here);
        }
        Some(place)
    }

    fn can_back(&self) -> bool {
        !self.back.is_empty()
    }

    fn can_forward(&self) -> bool {
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
fn parse_address(input: &str, routes: &routes::Routes, current_site: &str) -> Option<Address> {
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
fn looks_like_host(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 64
        && !text.contains(':')
        && !text.chars().any(|c| c.is_whitespace() || c.is_control())
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        && (text.contains('.') || text.contains('_'))
}

fn normalize_path(input: &str) -> Option<String> {
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
fn open_in_browser(url: &str) {
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

fn resolve_address(entries: &[net::Entry], home: Option<u16>, path: &str) -> Option<Resolved> {
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
fn resolve_path(entries: &[net::Entry], home: Option<u16>, path: &str) -> Option<u16> {
    resolve_address(entries, home, path).map(|r| r.id)
}



#[derive(Clone, Debug, PartialEq)]
enum Target {
    Page(u16),
    Path(String),
}

// ------------------------------------------------------------------ viewer

/// Everything one tab owns. A tab is the unit a browser calls a tab: its own
/// site, connection, image cache, history and form contents. Two tabs can show
/// two different sites at once without sharing anything.
struct Tab {
    /// Stable across tab reordering, so per-tab widget state (scroll offset,
    /// texture names) survives closing another tab.
    uid: u64,
    fetcher: fetch::Fetcher,
    loading: bool,
    site: String,
    #[cfg(feature = "admin")]
    tunnel: Option<tunnel::Tunnel>,
    entries: Vec<net::Entry>,
    page_id: Option<u16>,
    title: String,
    blocks: Vec<Block>,
    trail: Trail,
    home: Option<u16>,
    textures: HashMap<u16, egui::TextureHandle>,
    fields: HashMap<String, String>,
    form_target: Option<u16>,
    failed: bool,
    error: String,
    scroll_top: bool,
    loader: loader::Loader,
    /// Whether the address bar has the caret. A page must not steal focus from
    /// it, or keystrokes meant for the address vanish into a form field.
    address_focused: bool,
    focus_input: bool,
    saved_fields: HashMap<u16, HashMap<String, String>>,
    shown_path: Option<String>,
    /// When the current page's `[[scene]]` timelines started. Reset on every
    /// page load, so a reload replays the animation instead of resuming it.
    scene_clock: Option<std::time::Instant>,
    target: Option<Target>,
    /// Set while a site name is being looked up: the path to open once it arrives.
    pending_path: Option<String>,
    /// The name currently being looked up, for the status line.
    resolving: Option<String>,
}

/// The window: a list of tabs plus the address-bar state, which belongs to the
/// window rather than to any one tab.
struct Viewer {
    tabs: Vec<Tab>,
    active: usize,
    routes: routes::Routes,
    addr_input: String,
    addr_focus: bool,
    addr_typing: bool,
    addr_hold: bool,
    /// Set the moment the reader edits the bar, cleared when the text is
    /// accepted or dropped. Keeps the page from rewriting what is being typed.
    addr_edited: bool,
    last_title: String,
}

/// Which tab should be active after `index` is removed.
///
/// Separate from the viewer so the arithmetic is testable: closing a tab to the
/// left of the active one must not silently select somebody else.
fn active_after_close(len: usize, active: usize, index: usize) -> usize {
    if len <= 1 {
        return active;
    }
    let remaining = len - 1;
    if index < active {
        active - 1
    } else {
        active.min(remaining - 1)
    }
}

/// Source of `Tab::uid`. Only needs to be unique within one run.
static NEXT_TAB_UID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl Viewer {
    fn new(
        cc: &eframe::CreationContext<'_>,
        routes: routes::Routes,
        site: String,
        start_page: Option<Target>,
    ) -> Self {
        install_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(egui::Visuals::light());

        let mut viewer = Self {
            tabs: Vec::new(),
            active: 0,
            routes,
            addr_input: String::new(),
            addr_focus: false,
            addr_typing: false,
            addr_hold: false,
            addr_edited: false,
            last_title: String::new(),
        };
        viewer.open_tab(site, start_page);
        viewer.addr_input = viewer.tab().address_text();
        viewer
    }

    fn tab(&self) -> &Tab {
        &self.tabs[self.active.min(self.tabs.len() - 1)]
    }

    fn tab_mut(&mut self) -> &mut Tab {
        let index = self.active.min(self.tabs.len() - 1);
        &mut self.tabs[index]
    }

    fn open_tab(&mut self, site: String, target: Option<Target>) {
        let uid = NEXT_TAB_UID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.tabs.push(Tab::new(uid, &self.routes, site, target));
        self.active = self.tabs.len() - 1;
        self.addr_hold = false;
        self.addr_edited = false;
    }

    /// Opens the default site in a new tab. Used by the `+` button and `Ctrl+T`.
    fn new_tab(&mut self) {
        let site = self
            .routes
            .default_site()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "dev".to_string());
        self.open_tab(site, None);
    }

    /// Closes one tab. The last tab is never closed, so the window always has
    /// somewhere to draw.
    fn close_tab(&mut self, index: usize) {
        if self.tabs.len() <= 1 || index >= self.tabs.len() {
            return;
        }
        let next = active_after_close(self.tabs.len(), self.active, index);
        self.tabs.remove(index);
        self.active = next;
        self.addr_hold = false;
        self.addr_edited = false;
    }
}

impl Tab {
    fn new(uid: u64, routes: &routes::Routes, site: String, target: Option<Target>) -> Self {
        let endpoint = routes
            .get(&site)
            .map(|s| s.endpoint.clone())
            .unwrap_or_default();
        let tls = routes.get(&site).map(|s| s.tls).unwrap_or_default();

        let mut tab = Self {
            uid,
            fetcher: fetch::Fetcher::new(endpoint.clone(), tls),
            loading: false,
            site,
            #[cfg(feature = "admin")]
            tunnel: None,
            entries: Vec::new(),
            page_id: None,
            title: String::new(),
            blocks: Vec::new(),
            trail: Trail::default(),
            home: None,
            textures: HashMap::new(),
            fields: HashMap::new(),
            form_target: None,
            failed: false,
            error: String::new(),
            scroll_top: true,
            loader: loader::Loader::new(endpoint, tls),
            address_focused: false,
            focus_input: false,
            saved_fields: HashMap::new(),
            shown_path: None,
            scene_clock: None,
            target,
            pending_path: None,
            resolving: None,
        };
        #[cfg(feature = "admin")]
        {
            // Bring up the forward for this site before the first fetch.
            if let Some(spec) = routes.get(&tab.site).cloned() {
                if spec.ssh.is_some() {
                    match tunnel::Tunnel::ensure(&spec) {
                        Ok(t) => tab.tunnel = t,
                        Err(message) => tab.fail(message),
                    }
                }
            }
        }
        tab.bootstrap(routes);
        tab
    }

    /// What the user sees: a page on a site, never an endpoint.
    /// What the user sees: the site's own name plus the path. A host is just a
    /// name, so `nac.local/hanoi/4` and `iris.out/about` are the same shape.
    fn host(&self) -> &str {
        &self.site
    }

    fn path_of(&self, id: u16) -> Option<String> {
        self.entries
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.path.clone())
            .filter(|path| !path.is_empty())
    }

    /// What the user sees: a domain-like origin plus the page path, never an
    /// endpoint or an internal page number.
    fn address_text(&self) -> String {
        let path = self
            .shown_path
            .clone()
            .or_else(|| self.page_id.and_then(|id| self.path_of(id)))
            .unwrap_or_else(|| "/".to_string());
        format!("{}{}", self.host(), path)
    }

    fn bootstrap(&mut self, routes: &routes::Routes) {
        if routes.is_empty() {
            self.fail("找不到站点目录 nac.routes");
            return;
        }
        self.loading = true;
        self.fetcher.manifest();
    }

    /// Applies a manifest that arrived from the worker thread.
    /// A `Resolve` came back. Either we learned where the site is, or nobody on
    /// this connection knows that name.
    fn accept_resolved(
        &mut self,
        routes: &mut routes::Routes,
        name: String,
        found: Option<net::Found>,
    ) {
        self.resolving = None;
        self.loading = false;
        let path = self.pending_path.take().unwrap_or_else(|| "/".to_string());
        match found {
            Some(found) => {
                let tls = if found.tls {
                    routes::TlsMode::On
                } else {
                    routes::TlsMode::Off
                };
                routes.learn(&name, &found.endpoint, tls);
                let from = self.place();
                self.trail.record(from);
                self.connect(routes, name, Some(Target::Path(path)));
                // Until the first page arrives, the site's own name is the most
                // useful thing the window can say.
                if self.title.is_empty() {
                    self.title = found.title;
                }
            }
            None => {
                self.failed = true;
                self.error = format!("找不到站点 {name}");
            }
        }
    }

    fn accept_manifest(&mut self, entries: Option<Vec<net::Entry>>) {
        match entries {
            Some(entries) => {
                self.entries = entries;
                self.failed = false;
                self.error.clear();
                self.home = self
                    .entries
                    .iter()
                    .find(|e| e.path == "/")
                    .map(|e| e.id)
                    .or_else(|| self.entries.iter().map(|e| e.id).min());
                if self.entries.is_empty() {
                    self.loading = false;
                    self.fail("这个站点还没有任何页面");
                    return;
                }
                match self.target.take() {
                    Some(Target::Page(id)) => self.load(id),
                    Some(Target::Path(path)) => self.open_path(&path),
                    None => self.open_path("/"),
                }
            }
            None => {
                self.loading = false;
                self.entries.clear();
                self.home = None;
                self.page_id = None;
                self.shown_path = None;
                self.fail("无法连接到服务器");
            }
        }
    }

    fn connect(&mut self, routes: &routes::Routes, site: String, target: Option<Target>) {
        #[cfg(feature = "admin")]
        {
            self.tunnel = None;
            if let Some(spec) = routes.get(&site).cloned() {
                if spec.ssh.is_some() {
                    match tunnel::Tunnel::ensure(&spec) {
                        Ok(t) => self.tunnel = t,
                        Err(message) => {
                            self.site = site;
                            self.fail(message);
                            return;
                        }
                    }
                }
            }
        }
        let endpoint = routes
            .get(&site)
            .map(|s| s.endpoint.clone())
            .unwrap_or_default();
        let tls = routes.get(&site).map(|s| s.tls).unwrap_or_default();
        self.site = site;
        self.fetcher = fetch::Fetcher::new(endpoint.clone(), tls);
        self.loader = loader::Loader::new(endpoint, tls);
        self.textures.clear();
        self.entries.clear();
        self.saved_fields.clear();
        self.home = None;
        self.page_id = None;
        self.shown_path = None;
        self.blocks.clear();
        self.title.clear();
        self.fields.clear();
        self.form_target = None;
        self.error.clear();
        self.failed = false;
        self.scroll_top = true;
        self.target = target;
        self.bootstrap(routes);
    }

    fn go(&mut self, routes: &mut routes::Routes, input: &str) {
        let Some(address) = parse_address(input, routes, &self.site) else {
            self.failed = true;
            self.error = "地址无效".to_string();
            return;
        };
        self.failed = false;
        self.error.clear();
        match address {
            Address::Known { site, path } => {
                if site != self.site {
                    let from = self.place();
                    self.trail.record(from);
                    self.connect(routes, site, Some(Target::Path(path)));
                } else {
                    self.open_path(&path);
                }
            }
            // A name with our own suffix that the table has never heard of: ask
            // the server we are on. This is what lets a site be added without
            // reissuing the client.
            Address::Unknown { site, path } => {
                if self.resolving.as_deref() == Some(site.as_str()) {
                    return;
                }
                self.pending_path = Some(path);
                self.resolving = Some(site.clone());
                self.loading = true;
                self.fetcher.resolve(site);
            }
        }
    }

    fn reload(&mut self, routes: &routes::Routes) {
        self.error.clear();
        self.failed = false;
        let path = self
            .shown_path
            .clone()
            .or_else(|| self.page_id.and_then(|id| self.path_of(id)));
        match path {
            Some(path) => {
                // Refresh the manifest first: static pages are cached by its
                // hash, so F5 must see a republish before reusing the cache.
                self.target = Some(Target::Path(path));
                self.loading = true;
                self.fetcher.manifest();
            }
            None => self.bootstrap(routes),
        }
    }

    fn resolve_path(&self, path: &str) -> Option<Resolved> {
        resolve_address(&self.entries, self.home, path)
    }

    fn open_path(&mut self, path: &str) {
        let path = normalize_path(path).unwrap_or_else(|| path.to_string());
        self.shown_path = Some(path.clone());
        if let Some(found) = self.resolve_path(&path) {
            self.failed = false;
            self.error.clear();
            if self.page_id == Some(found.id) {
                self.load_at(found.id, found.path);
            } else {
                self.navigate_at(found.id, found.path);
            }
            return;
        }

        // Old links and bookmarks may still name a page by number.
        let bare = path.trim_start_matches('/');
        if !bare.contains('/') {
            if let Ok(id) = bare.parse::<u16>() {
                if self.entries.iter().any(|e| e.id == id) {
                    self.failed = false;
                    self.error.clear();
                    let shown = format!("/{id}");
                    if self.page_id == Some(id) {
                        self.load_at(id, shown);
                    } else {
                        self.navigate_at(id, shown);
                    }
                    return;
                }
            }
        }

        self.page_id = None;
        self.loading = false;
        self.fail(format!("404 · 没有 {path}"));
    }

    fn fail(&mut self, message: impl Into<String>) {
        self.failed = true;
        self.error = message.into();
        self.title.clear();
        self.blocks.clear();
        self.form_target = None;
    }

    fn load(&mut self, id: u16) {
        self.load_at(id, String::new());
    }

    /// Loads a page and tells it which address was asked for. An empty `path`
    /// means "the page's own address", which is what a plain page-number request
    /// means.
    fn load_at(&mut self, id: u16, path: String) {
        self.save_fields();
        let path = if path.is_empty() {
            self.path_of(id).unwrap_or_else(|| format!("/{id}"))
        } else {
            path
        };
        self.shown_path = Some(path.clone());
        if !self.entries.iter().any(|e| e.id == id) {
            self.loading = false;
            self.page_id = Some(id);
            self.fail("页面不存在");
            return;
        }
        self.loading = true;
        self.fetcher.page(id, path);
    }

    fn accept_page(&mut self, id: u16, text: Option<String>) {
        self.loading = false;
        match text {
            Some(text) => self.apply(id, &text),
            None => {
                self.page_id = Some(id);
                self.fail("页面加载失败");
            }
        }
    }

    fn apply(&mut self, id: u16, text: &str) {
        self.blocks = markup::parse(text);
        self.title = self
            .blocks
            .iter()
            .find_map(|b| match b {
                Block::Heading(_, parts) => Some(markup::plain(parts)),
                _ => None,
            })
            .or_else(|| {
                self.entries
                    .iter()
                    .find(|e| e.id == id)
                    .map(|e| e.title.clone())
            })
            .unwrap_or_default();
        self.page_id = Some(id);
        // A redirect can land on the page that already answers the address we
        // were on — a prefix page answering `/hn/item/12`. Keep that address in
        // that case, or it would collapse to the page's own `/hn/item` and the
        // next reload would have nothing to look up.
        let address_still_fits = self
            .shown_path
            .as_deref()
            .and_then(|path| self.resolve_path(path))
            .map(|found| found.id == id)
            .unwrap_or(false);
        if !address_still_fits {
            self.shown_path = self.path_of(id).or_else(|| Some(format!("/{id}")));
        }
        self.fields = self.saved_fields.remove(&id).unwrap_or_default();
        self.form_target = None;
        self.failed = false;
        self.error.clear();
        self.scroll_top = true;
        self.focus_input = true;
        // A new page starts its animations from zero.
        self.scene_clock = None;
    }

    /// The current address, preferring what was actually asked for over the page's
    /// own path — a page answering `/item/` was still reached at `/item/123`.
    fn place(&self) -> Option<Place> {
        let path = self
            .shown_path
            .clone()
            .or_else(|| self.page_id.and_then(|id| self.path_of(id)))?;
        Some(Place {
            site: self.site.clone(),
            path,
        })
    }

    /// Goes to a remembered location, following the site boundary if needed.
    /// History is preserved: this is what makes the back button work across sites.
    fn go_place(&mut self, routes: &routes::Routes, place: Place) {
        if place.site == self.site {
            self.open_path(&place.path);
        } else {
            self.connect(routes, place.site, Some(Target::Path(place.path)));
        }
    }

    fn navigate(&mut self, target: u16) {
        self.navigate_at(target, String::new());
    }

    fn navigate_at(&mut self, target: u16, path: String) {
        if let Some(current) = self.page_id {
            if current != target || !path.is_empty() {
                let from = self.place();
                self.trail.record(from);
            }
        }
        self.load_at(target, path);
    }

    fn back(&mut self, routes: &routes::Routes) {
        let current = self.place();
        if let Some(prev) = self.trail.go_back(current) {
            self.go_place(routes, prev);
        }
    }

    fn forward(&mut self, routes: &routes::Routes) {
        let current = self.place();
        if let Some(next) = self.trail.go_forward(current) {
            self.go_place(routes, next);
        }
    }

    fn home_page(&mut self) {
        self.open_path("/");
    }

    fn save_fields(&mut self) {
        stash_fields(&mut self.saved_fields, self.page_id, &self.fields);
    }

    fn submit(&mut self) {
        let Some(target) = self.form_target else {
            return;
        };
        let fields: Vec<(String, String)> = self
            .fields
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if let Some(place) = self.place() {
            self.trail.record(Some(place));
        }
        // The submission consumed what was typed; coming back here later should
        // start clean.
        self.saved_fields.remove(&target);
        self.loading = true;
        // A form on a page that answers a prefix has to come back to the same
        // address, or the handler will not know which item was submitted for.
        let path = self
            .shown_path
            .clone()
            .or_else(|| self.path_of(target))
            .unwrap_or_default();
        self.fetcher.submit(target, path, fields);
    }

    fn job(&self, parts: &[Inline], size: f32, color: Color32, family: FontFamily) -> LayoutJob {
        let mut job = LayoutJob::default();
        for p in parts {
            let (text, fmt) = match p {
                Inline::Text(t) => (
                    t.clone(),
                    TextFormat {
                        font_id: FontId::new(size, family.clone()),
                        color,
                        ..Default::default()
                    },
                ),
                Inline::Bold(t) => (
                    t.clone(),
                    TextFormat {
                        font_id: FontId::new(
                            size,
                            FontFamily::Name(Arc::from("bold")),
                        ),
                        color,
                        ..Default::default()
                    },
                ),
                Inline::Italic(t) => (
                    t.clone(),
                    TextFormat {
                        font_id: FontId::new(size, family.clone()),
                        color,
                        italics: true,
                        ..Default::default()
                    },
                ),
                Inline::Code(t) => (
                    format!(" {t} "),
                    TextFormat {
                        font_id: FontId::new((size - 1.0).max(10.0), FontFamily::Monospace),
                        color,
                        background: Color32::from_gray(236),
                        ..Default::default()
                    },
                ),
                Inline::Link { text, .. } => (
                    text.clone(),
                    TextFormat {
                        font_id: FontId::new(size, family.clone()),
                        color: Color32::from_rgb(40, 100, 190),
                        ..Default::default()
                    },
                ),
            };
            job.append(&text, 0.0, fmt);
        }
        job
    }

    fn render_image(&mut self, ui: &mut egui::Ui, id: u16, pct: f32, ratio: Option<f32>) {
        if let Some(tex) = self.textures.get(&id) {
            let px = tex.size_vec2();
            let w = (ui.available_width() * pct).min(px.x).max(1.0);
            let h = if px.x > 0.0 { w * px.y / px.x } else { w };
            ui.add(egui::Image::new(egui::load::SizedTexture::new(
                tex.id(),
                egui::vec2(w, h),
            )));
            return;
        }

        if self.loader.failed.contains(&id) {
            ui.label(RichText::new("[图片无法加载]").weak());
            return;
        }

        self.loader.request(id);
        let w = (ui.available_width() * pct).max(1.0);
        // Reserve the real box when publish told us the aspect ratio, so the
        // page does not jump when the image lands.
        let h = match ratio {
            Some(r) if r > 0.0 => (w / r).clamp(20.0, 2000.0),
            _ => 120.0,
        };
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 4.0, Color32::from_gray(238));
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "加载中…",
            FontId::proportional(13.0),
            Color32::from_gray(150),
        );
    }

    /// Renders a run of inline content. When it contains links they become real
    /// widgets and the text flows around them; otherwise it is one laid-out job.
    ///
    /// `wrap` is off inside table cells: there the surrounding `Grid` decides
    /// column widths from what each cell asks for, and a wrapping layout asks for
    /// its minimum, which splits labels like "分栏实验" across two lines.
    fn render_flow(
        &mut self,
        ui: &mut egui::Ui,
        parts: &[Inline],
        size: f32,
        color: Color32,
        family: FontFamily,
    ) {
        self.render_flow_with(ui, parts, size, color, family, true);
    }

    fn render_flow_with(
        &mut self,
        ui: &mut egui::Ui,
        parts: &[Inline],
        size: f32,
        color: Color32,
        family: FontFamily,
        wrap: bool,
    ) {
        if !parts.iter().any(|p| matches!(p, Inline::Link { .. })) {
            ui.label(self.job(parts, size, color, family));
            return;
        }

        let mut run: Vec<Inline> = Vec::new();
        let mut clicked: Option<LinkTarget> = None;
        let body = |ui: &mut egui::Ui, run: &mut Vec<Inline>, clicked: &mut Option<LinkTarget>| {
            // The gap between a label and a link would otherwise be an extra
            // space in the middle of a sentence.
            ui.spacing_mut().item_spacing.x = 0.0;
            for part in parts {
                match part {
                    Inline::Link { target, text } => {
                        if !run.is_empty() {
                            let job = self.job(run, size, color, family.clone());
                            ui.label(job);
                            run.clear();
                        }
                        if ui.link(RichText::new(text).size(size)).clicked() {
                            *clicked = Some(target.clone());
                        }
                    }
                    other => run.push(other.clone()),
                }
            }
            if !run.is_empty() {
                let job = self.job(run, size, color, family.clone());
                ui.label(job);
            }
        };
        if wrap {
            ui.horizontal_wrapped(|ui| body(ui, &mut run, &mut clicked));
        } else {
            ui.horizontal(|ui| body(ui, &mut run, &mut clicked));
        }

        if let Some(target) = clicked {
            match target {
                LinkTarget::Page(id) => self.navigate(id),
                LinkTarget::Path(path) => self.open_path(&path),
                LinkTarget::Url(url) => open_in_browser(&url),
            }
        }
    }

    /// Draws a `[[scene]]` and keeps the clock running while it has time left.
    ///
    /// The page lays out rectangles in its own coordinates; this only scales the
    /// canvas to the window, walks each bar's keyframes with a smoothstep so
    /// motion starts and stops gently, and asks for repaints until the timeline
    /// is over. Clicking the canvas replays it.
    fn render_scene(
        &mut self,
        ui: &mut egui::Ui,
        width: f32,
        height: f32,
        seconds: f32,
        bars: &[markup::SceneBar],
    ) {
        let available = ui.available_width().max(120.0);
        let scale = (available / width).min(1.6).max(0.2);
        let size = egui::vec2(width * scale, height * scale);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
        if response.clicked() {
            self.scene_clock = Some(std::time::Instant::now());
        }
        let started = *self.scene_clock.get_or_insert_with(std::time::Instant::now);
        let elapsed = started.elapsed().as_secs_f32();

        let painter = ui.painter_at(rect);
        painter.rect_filled(
            rect,
            egui::CornerRadius::same(8),
            egui::Color32::from_rgb(248, 248, 250),
        );
        painter.rect_stroke(
            rect,
            egui::CornerRadius::same(8),
            egui::Stroke::new(1.0, egui::Color32::from_rgb(228, 228, 234)),
            egui::StrokeKind::Inside,
        );

        let look = |bar: &markup::SceneBar, t: f32| -> Option<(f32, f32)> {
            let keys = &bar.keys;
            if keys.is_empty() {
                return None;
            }
            if t <= keys[0].0 {
                return Some((keys[0].1, keys[0].2));
            }
            let last = keys[keys.len() - 1];
            if t >= last.0 {
                return Some((last.1, last.2));
            }
            for pair in keys.windows(2) {
                let (t0, x0, y0) = pair[0];
                let (t1, x1, y1) = pair[1];
                if t >= t0 && t <= t1 {
                    let span = (t1 - t0).max(f32::EPSILON);
                    let u = (t - t0) / span;
                    let eased = u * u * (3.0 - 2.0 * u);
                    return Some((x0 + (x1 - x0) * eased, y0 + (y1 - y0) * eased));
                }
            }
            Some((last.1, last.2))
        };

        for bar in bars {
            let Some((x, y)) = look(bar, elapsed) else {
                continue;
            };
            let bar_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left() + x * scale, rect.top() + y * scale),
                egui::vec2(bar.width * scale, bar.height * scale),
            );
            painter.rect_filled(
                bar_rect,
                egui::CornerRadius::same((bar.height * scale * 0.35) as u8),
                egui::Color32::from_rgb(bar.color[0], bar.color[1], bar.color[2]),
            );
        }

        if elapsed < seconds.max(0.0) {
            ui.ctx().request_repaint();
        }
    }

    fn render_table(
        &mut self,
        ui: &mut egui::Ui,
        header: &[Vec<Inline>],
        rows: &[Vec<Vec<Inline>>],
    ) {
        let columns = header.len().max(1);
        let id = ui.next_auto_id();
        egui::Grid::new(id)
            .num_columns(columns)
            .spacing([18.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                for cell in header {
                    ui.label(self.job(
                        cell,
                        15.5,
                        Color32::from_gray(15),
                        FontFamily::Name(Arc::from("bold")),
                    ));
                }
                ui.end_row();
                for row in rows {
                    for index in 0..columns {
                        match row.get(index) {
                            Some(cell) => self.render_flow_with(
                                ui,
                                cell,
                                16.0,
                                Color32::from_gray(28),
                                FontFamily::Proportional,
                                false,
                            ),
                            None => {
                                ui.label("");
                            }
                        };
                    }
                    ui.end_row();
                }
            });
        ui.add_space(8.0);
    }

    fn render_note(
        &mut self,
        ui: &mut egui::Ui,
        kind: NoteKind,
        title: &[Inline],
        body: Vec<Block>,
    ) {
        let (bar, fill) = match kind {
            NoteKind::Info => (Color32::from_rgb(74, 132, 199), Color32::from_rgb(241, 246, 253)),
            NoteKind::Ok => (Color32::from_rgb(64, 152, 101), Color32::from_rgb(241, 250, 244)),
            NoteKind::Warn => (Color32::from_rgb(206, 156, 60), Color32::from_rgb(254, 250, 240)),
            NoteKind::Danger => (Color32::from_rgb(198, 82, 82), Color32::from_rgb(253, 243, 243)),
        };
        let frame = egui::Frame::new()
            .fill(fill)
            .inner_margin(egui::Margin::symmetric(10, 8));
        let full_width = ui.available_width();
        frame.show(ui, |ui| {
            // Without this the box shrinks to whatever the text happens to need,
            // so a page with several notes gets boxes of different widths.
            ui.set_min_width((full_width - 20.0).max(60.0));
            if !title.is_empty() {
                ui.horizontal(|ui| {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(4.0, 15.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, bar);
                    ui.add_space(6.0);
                    ui.label(self.job(
                        title,
                        16.0,
                        bar,
                        FontFamily::Name(Arc::from("bold")),
                    ));
                });
                ui.add_space(4.0);
            }
            self.render_list(ui, body);
        });
        ui.add_space(8.0);
    }

    fn render_tabs(&mut self, ui: &mut egui::Ui, titles: &[String], bodies: Vec<Vec<Block>>) {
        let id = ui.next_auto_id();
        let mut selected = ui.data_mut(|d| *d.get_temp_mut_or_default::<usize>(id));
        if selected >= titles.len() {
            selected = 0;
        }
        ui.horizontal_wrapped(|ui| {
            for (index, title) in titles.iter().enumerate() {
                if ui.selectable_label(index == selected, title.as_str()).clicked() {
                    selected = index;
                }
            }
        });
        ui.data_mut(|d| d.insert_temp(id, selected));
        ui.add_space(6.0);
        if let Some(body) = bodies.into_iter().nth(selected) {
            self.render_list(ui, body);
        }
        ui.add_space(6.0);
    }

    fn render_blocks(&mut self, ui: &mut egui::Ui) {
        let blocks = self.blocks.clone();
        self.render_list(ui, blocks);
    }

    fn render_list(&mut self, ui: &mut egui::Ui, blocks: Vec<Block>) {
        let body = Color32::from_gray(28);
        let muted = Color32::from_gray(110);
        let mut saw_input = false;

        for block in blocks {
            match block {
                Block::Heading(level, parts) => {
                    let size = match level {
                        1 => 30.0,
                        2 => 23.0,
                        _ => 19.0,
                    };
                    ui.add_space(14.0);
                    ui.label(self.job(
                        &parts,
                        size,
                        Color32::from_gray(15),
                        FontFamily::Name(Arc::from("bold")),
                    ));
                    ui.add_space(6.0);
                }
                Block::Para(parts) => {
                    self.render_flow(ui, &parts, 17.0, body, FontFamily::Proportional);
                    ui.add_space(8.0);
                }
                Block::Quote(parts) => {
                    ui.horizontal(|ui| {
                        ui.add_space(6.0);
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(3.0, 20.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, 0.0, Color32::from_gray(200));
                        ui.add_space(8.0);
                        ui.label(self.job(&parts, 16.0, muted, FontFamily::Proportional));
                    });
                    ui.add_space(8.0);
                }
                Block::List(items) => {
                    for item in items {
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(RichText::new("•").color(muted));
                            ui.label(self.job(&item, 17.0, body, FontFamily::Proportional));
                        });
                    }
                    ui.add_space(8.0);
                }
                Block::Pre { lang, text } => {
                    if let Some(lang) = lang {
                        ui.label(RichText::new(lang).size(11.0).color(muted));
                    }
                    ui.label(
                        RichText::new(text)
                            .monospace()
                            .size(15.0)
                            .color(Color32::from_gray(40)),
                    );
                    ui.add_space(8.0);
                }
                Block::Image { id, pct, ratio } => {
                    self.render_image(ui, id, pct, ratio);
                    ui.add_space(8.0);
                }
                Block::Link { target, text } => {
                    if ui
                        .link(RichText::new(text).size(17.0))
                        .clicked()
                    {
                        match target {
                            LinkTarget::Page(id) => self.navigate(id),
                            LinkTarget::Path(path) => self.open_path(&path),
                            LinkTarget::Url(url) => open_in_browser(&url),
                        }
                    }
                    ui.add_space(4.0);
                }
                Block::Form { target } => {
                    self.form_target = Some(target);
                    ui.add_space(4.0);
                }
                Block::Input {
                    name,
                    label,
                    secret,
                } => {
                    saw_input = true;
                    let mut value = self.fields.get(&name).cloned().unwrap_or_default();
                    let mut resp_opt = None;
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut value)
                                .password(secret)
                                .hint_text(label)
                                .desired_width(320.0),
                        );
                        if resp.changed() {
                            self.fields.insert(name.clone(), value.clone());
                        }
                        resp_opt = Some(resp);
                    });
                    if let Some(resp) = resp_opt {
                        // Never steal the caret from the address bar: a page that
                        // finishes loading while the reader is typing an address
                        // would otherwise eat every keystroke after that point.
                        if self.focus_input && !self.address_focused {
                            resp.request_focus();
                            self.focus_input = false;
                        }
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.submit();
                            return;
                        }
                    }
                    ui.add_space(4.0);
                }
                Block::Hidden { name, value } => {
                    self.fields.insert(name.clone(), value.clone());
                }
                Block::Scene {
                    width,
                    height,
                    seconds,
                    bars,
                } => {
                    self.render_scene(ui, width, height, seconds, &bars);
                }
                Block::Button { label } => {
                    if ui
                        .add(egui::Button::new(RichText::new(label).size(16.0)))
                        .clicked()
                    {
                        self.submit();
                        return;
                    }
                    ui.add_space(8.0);
                }
                Block::Gap => ui.add_space(6.0),
                Block::Hr => {
                    ui.add_space(6.0);
                    let width = ui.available_width();
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 0.0, Color32::from_gray(212));
                    ui.add_space(8.0);
                }
                Block::Ordered(items) => {
                    for (index, item) in items.into_iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(
                                RichText::new(format!("{}.", index + 1))
                                    .size(16.0)
                                    .color(muted),
                            );
                            ui.add_space(2.0);
                            ui.label(self.job(&item, 17.0, body, FontFamily::Proportional));
                        });
                    }
                    ui.add_space(8.0);
                }
                Block::Caption(parts) => {
                    ui.label(self.job(&parts, 13.0, muted, FontFamily::Proportional));
                    ui.add_space(8.0);
                }
                Block::Table { header, rows } => {
                    self.render_table(ui, &header, &rows);
                }
                Block::Note {
                    kind,
                    title,
                    body: inner,
                } => {
                    self.render_note(ui, kind, &title, inner);
                }
                Block::Details { summary, body } => {
                    let label = markup::plain(&summary);
                    let salt = ui.next_auto_id();
                    egui::CollapsingHeader::new(RichText::new(label).size(16.0))
                        .id_salt(salt)
                        .show(ui, |ui| self.render_list(ui, body));
                    ui.add_space(6.0);
                }
                Block::Tabs { titles, bodies } => {
                    self.render_tabs(ui, &titles, bodies);
                }
                Block::Textarea { name, label } => {
                    saw_input = true;
                    ui.label(RichText::new(label).size(14.0).color(muted));
                    let mut value = self.fields.get(&name).cloned().unwrap_or_default();
                    let width = ui.available_width();
                    let resp = ui.add(
                        egui::TextEdit::multiline(&mut value)
                            .desired_rows(3)
                            .desired_width(width),
                    );
                    if resp.changed() {
                        self.fields.insert(name, value);
                    }
                    ui.add_space(4.0);
                }
                Block::Select {
                    name,
                    label,
                    options,
                } => {
                    saw_input = true;
                    let current = self
                        .fields
                        .get(&name)
                        .cloned()
                        .filter(|v| options.contains(v))
                        .unwrap_or_else(|| options.first().cloned().unwrap_or_default());
                    let mut chosen = current.clone();
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.label(RichText::new(label).size(15.0).color(muted));
                        egui::ComboBox::from_id_salt(("nac-select", name.as_str()))
                            .selected_text(current.clone())
                            .show_ui(ui, |ui| {
                                for option in &options {
                                    ui.selectable_value(&mut chosen, option.clone(), option.as_str());
                                }
                            });
                    });
                    if chosen != current {
                        self.fields.insert(name, chosen);
                    }
                    ui.add_space(4.0);
                }
                Block::Checkbox { name, label } => {
                    saw_input = true;
                    let id = ui.scope_id().with(("nac-check", name.as_str()));
                    let mut checked = ui.data_mut(|d| *d.get_temp_mut_or_default::<bool>(id));
                    if ui.checkbox(&mut checked, label).changed() {
                        self.fields.insert(
                            name,
                            if checked { "1".to_string() } else { String::new() },
                        );
                    }
                    ui.data_mut(|d| d.insert_temp(id, checked));
                    ui.add_space(4.0);
                }
                Block::Columns(parts) => {
                    let n = parts.len().clamp(1, 4);
                    let gap = ui.spacing().item_spacing.x;
                    let total = ui.available_width();
                    let col_w = ((total - gap * (n as f32 - 1.0)) / n as f32).max(80.0);
                    // 手工摆放列，而不是 ui.columns：后者会把列宽收缩到内容宽度，
                    // 三列以上时会挤在左边。
                    ui.horizontal_top(|ui| {
                        for (idx, part) in parts.into_iter().enumerate() {
                            if idx > 0 {
                                ui.add_space(gap);
                            }
                            ui.allocate_ui_with_layout(
                                egui::vec2(col_w, 0.0),
                                Layout::top_down(Align::LEFT),
                                |ui| {
                                    ui.set_width(col_w);
                                    self.render_list(ui, part);
                                },
                            );
                        }
                    });
                    ui.add_space(6.0);
                }
            }
        }

        if !saw_input {
            self.focus_input = false;
        }
    }
}

impl Viewer {
    /// Runs `f` against the active tab while handing it the site table. The two
    /// live in separate fields, so the borrow checker can see they do not alias.
    fn with_tab<F: FnOnce(&mut Tab, &routes::Routes)>(&mut self, f: F) {
        let index = self.active.min(self.tabs.len() - 1);
        let routes = &self.routes;
        f(&mut self.tabs[index], routes);
    }

    fn with_tab_routes<F: FnOnce(&mut Tab, &mut routes::Routes)>(&mut self, f: F) {
        let index = self.active.min(self.tabs.len() - 1);
        let routes = &mut self.routes;
        f(&mut self.tabs[index], routes);
    }
}

impl eframe::App for Viewer {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Every tab keeps working while it is in the background: a page still
        // loading in one tab does not stall because another tab is on screen.
        for index in 0..self.tabs.len() {
            for msg in self.tabs[index].fetcher.drain() {
                match msg {
                    fetch::Msg::Manifest(entries) => self.tabs[index].accept_manifest(entries),
                    fetch::Msg::Page { id, text } => self.tabs[index].accept_page(id, text),
                    fetch::Msg::Resolved { name, found } => {
                        self.tabs[index].accept_resolved(&mut self.routes, name, found)
                    }
                }
            }
            for msg in self.tabs[index].loader.drain() {
                if let loader::Msg::Done(loaded) = msg {
                    let color = egui::ColorImage::from_rgba_unmultiplied(
                        [loaded.width as usize, loaded.height as usize],
                        &loaded.rgba,
                    );
                    // Texture names are per tab: two tabs may show the same page
                    // number with different images. `uid` rather than the index,
                    // because indices shift when a tab is closed.
                    let tex = ctx.load_texture(
                        format!("img-{}-{}", self.tabs[index].uid, loaded.id),
                        color,
                        egui::TextureOptions::LINEAR,
                    );
                    self.tabs[index].textures.insert(loaded.id, tex);
                }
            }
            if self.tabs[index].fetcher.busy() {
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
            if self.tabs[index].loader.busy() {
                ctx.request_repaint_after(std::time::Duration::from_millis(50));
            }
        }

        let title = self.tab().title.clone();
        if title != self.last_title {
            self.last_title = title.clone();
            let text = if title.is_empty() {
                "nac".to_string()
            } else {
                format!("{title} — nac")
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(text));
        }

        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::L)) {
            self.addr_focus = true;
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::T)) {
            self.new_tab();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::W)) {
            let index = self.active;
            self.close_tab(index);
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Tab)) {
            self.active = (self.active + 1) % self.tabs.len();
            self.addr_hold = false;
        }

        let typing = ctx.egui_wants_keyboard_input();
        if ctx.input(|i| i.key_pressed(egui::Key::F5))
            || (!typing && ctx.input(|i| i.key_pressed(egui::Key::R)))
        {
            self.with_tab(|tab, routes| tab.reload(routes));
            self.addr_hold = false;
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Enter)) {
            self.tab_mut().submit();
        }
        if (ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !self.addr_typing)
            || ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft))
        {
            self.with_tab(|tab, routes| tab.back(routes));
            self.addr_hold = false;
            self.addr_edited = false;
        }
        if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight)) {
            self.with_tab(|tab, routes| tab.forward(routes));
            self.addr_hold = false;
        }

        // ------------------------------------------------------------ tab bar
        let mut switch_to: Option<usize> = None;
        let mut close: Option<usize> = None;
        let mut add_tab = false;
        let can_close = self.tabs.len() > 1;
        egui::Panel::top("tabs").show(ui, |ui| {
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                for (index, tab) in self.tabs.iter().enumerate() {
                    match tab_chip(ui, tab, index == self.active, can_close) {
                        TabHit::Switch => switch_to = Some(index),
                        TabHit::Close => close = Some(index),
                        TabHit::None => {}
                    }
                    ui.add_space(4.0);
                }
                if ui
                    .add(egui::Button::new(RichText::new("+").size(15.0)).frame(false))
                    .on_hover_text("新标签页  (Ctrl+T)")
                    .clicked()
                {
                    add_tab = true;
                }
            });
            ui.add_space(5.0);
            strip_line(ui);
        });
        if let Some(index) = switch_to {
            if index != self.active {
                self.active = index;
                self.addr_hold = false;
                self.addr_edited = false;
            }
        }
        if let Some(index) = close {
            self.close_tab(index);
        }
        if add_tab {
            self.new_tab();
        }

        // ------------------------------------------------------------ toolbar
        let mut want_back = false;
        let mut want_forward = false;
        let mut want_reload = false;
        let mut want_home = false;
        let mut goto: Option<String> = None;
        let back_ok = self.tab().trail.can_back();
        let forward_ok = self.tab().trail.can_forward();

        // One row, one height: every control in the toolbar is the same size, so
        // the bar does not reflow when a glyph changes width.
        const BUTTON: egui::Vec2 = egui::vec2(30.0, 30.0);
        // Flat until touched: a row of grey slabs reads as a dialog, not as a
        // browser. The button only lights up under the pointer.
        let nav = |ui: &mut egui::Ui, glyph: &str, enabled: bool, tip: &str| -> bool {
            let (rect, resp) = ui.allocate_exact_size(BUTTON, egui::Sense::click());
            let resp = resp.on_hover_text(tip);
            let down = enabled && resp.is_pointer_button_down_on();
            let hot = enabled && resp.hovered();
            if hot {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                let bg = if down {
                    Color32::from_rgb(211, 218, 230)
                } else {
                    Color32::from_rgb(234, 237, 243)
                };
                ui.painter()
                    .rect_filled(rect, egui::CornerRadius::same(7), bg);
            }
            let ink = if enabled {
                Color32::from_rgb(48, 54, 64)
            } else {
                Color32::from_rgb(183, 187, 195)
            };
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                glyph,
                egui::FontId::proportional(16.0),
                ink,
            );
            enabled && resp.clicked()
        };

        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.add_space(6.0);
                if nav(ui, "←", back_ok, "后退  (Alt+←)") {
                    want_back = true;
                }
                if nav(ui, "→", forward_ok, "前进  (Alt+→)") {
                    want_forward = true;
                }
                if nav(ui, "⟳", true, "刷新  (F5)") {
                    want_reload = true;
                }
                if nav(ui, "⌂", true, "回到首页") {
                    want_home = true;
                }
                ui.add_space(4.0);

                let width = (ui.available_width() - 14.0).max(120.0);
                let hint = self
                    .routes
                    .default_site()
                    .map(|site| format!("{}/路径", site.name))
                    .unwrap_or_else(|| "名字/路径".to_string());
                // A visible box, so the bar reads as "type here" instead of as a
                // line of text floating in the toolbar. Focused, it gets a blue ring.
                let ring = if self.addr_typing {
                    egui::Stroke::new(1.5, Color32::from_rgb(74, 132, 199))
                } else {
                    egui::Stroke::new(1.0, Color32::from_rgb(208, 211, 220))
                };
                let resp = egui::Frame::new()
                    .fill(Color32::from_rgb(255, 255, 255))
                    .stroke(ring)
                    .corner_radius(egui::CornerRadius::same(7))
                    .inner_margin(egui::Margin::symmetric(2, 2))
                    .show(ui, |ui| {
                        ui.add_sized(
                            [width - 4.0, 26.0],
                            egui::TextEdit::singleline(&mut self.addr_input)
                                .font(egui::TextStyle::Monospace)
                                .margin(egui::Margin::symmetric(7, 5))
                                .hint_text(hint),
                        )
                    })
                    .inner;
                if self.addr_focus {
                    resp.request_focus();
                    // Ctrl+L is meant to be "replace this", not "append to it".
                    if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), resp.id) {
                        state
                            .cursor
                            .set_char_range(Some(egui::text::CCursorRange::two(
                                egui::text::CCursor::new(0),
                                egui::text::CCursor::new(self.addr_input.chars().count()),
                            )));
                        state.store(ui.ctx(), resp.id);
                    }
                    self.addr_focus = false;
                }
                if resp.changed() {
                    self.addr_edited = true;
                }
                // A single-line `TextEdit` gives up focus the moment Enter is
                // pressed, so `has_focus` alone would never see it -- that is
                // why Enter in the address bar used to do nothing at all.
                let mine = resp.has_focus() || resp.lost_focus();
                if mine && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    goto = Some(self.addr_input.clone());
                    // Hold the typed text until the page it leads to lands,
                    // then let the bar show where we actually ended up. Focus
                    // stays put, so a second address can be typed now.
                    self.addr_hold = true;
                    self.addr_edited = false;
                    resp.request_focus();
                } else if mine && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    // Put back what the tab is actually showing and leave.
                    self.addr_hold = false;
                    self.addr_edited = false;
                    self.addr_input = self.tab().address_text();
                    resp.surrender_focus();
                }
                self.addr_typing = resp.has_focus();
            });
            ui.add_space(6.0);
        });

        if want_back {
            self.with_tab(|tab, routes| tab.back(routes));
            self.addr_hold = false;
        }
        if want_forward {
            self.with_tab(|tab, routes| tab.forward(routes));
            self.addr_hold = false;
        }
        if want_reload {
            self.with_tab(|tab, routes| tab.reload(routes));
            self.addr_hold = false;
        }
        if want_home {
            self.tab_mut().home_page();
            self.addr_hold = false;
        }
        if let Some(input) = goto {
            self.with_tab_routes(|tab, routes| tab.go(routes, &input));
            // If that address could not even start loading (bad syntax), do not
            // sit on the typed text: let the bar fall back to the real address.
            if !self.tab().loading {
                self.addr_hold = false;
            }
        }

        // --------------------------------------------------------- status bar
        let address = self.tab().address_text();
        let status = {
            let tab = self.tab();
            if !tab.error.is_empty() {
                tab.error.clone()
            } else if tab.loading {
                format!("{address} · 正在加载…")
            } else if tab.failed {
                format!("{address} · 无法获取内容")
            } else {
                format!("{address} · 已连接")
            }
        };
        let bad = {
            let tab = self.tab();
            tab.failed || !tab.error.is_empty()
        };
        let tab_count = self.tabs.len();
        egui::Panel::bottom("bottom").show(ui, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                let color = if bad {
                    Color32::from_rgb(190, 70, 70)
                } else {
                    Color32::from_rgb(90, 160, 90)
                };
                ui.painter().circle_filled(rect.center(), 4.0, color);
                ui.label(RichText::new(status).weak().size(12.0));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{tab_count} 个标签页 · Ctrl+T 新标签 · Ctrl+L 地址 · F5 刷新"
                        ))
                        .weak()
                        .size(12.0),
                    );
                });
            });
            ui.add_space(2.0);
        });

        // ------------------------------------------------------------- content
        let reset_scroll = self.tab().scroll_top;
        let failed = self.tab().failed;
        let error = self.tab().error.clone();
        let uid = self.tab().uid;
        egui::CentralPanel::default().show(ui, |ui| {
            // Salted by tab: without it every tab would share one scroll offset,
            // so switching tabs would land you mid-page somewhere else.
            let mut area = egui::ScrollArea::vertical()
                .id_salt(("page", uid))
                .auto_shrink([false, false]);
            if reset_scroll {
                area = area.vertical_scroll_offset(0.0);
            }
            area.show(ui, |ui| {
                if failed {
                    ui.add_space(140.0);
                    ui.vertical_centered(|ui| {
                        let message = if error.is_empty() {
                            "无法获取内容".to_string()
                        } else {
                            error.clone()
                        };
                        ui.label(
                            RichText::new(message)
                                .size(20.0)
                                .color(Color32::from_gray(120)),
                        );
                        ui.add_space(10.0);
                        ui.label(RichText::new("按 F5 重试，或在地址栏里换个地址").weak());
                    });
                    return;
                }

                let width = ui.available_width();
                let pad = ((width - 860.0) * 0.5).max(20.0);
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    ui.vertical(|ui| {
                        ui.set_max_width((width - pad * 2.0).max(320.0));
                        ui.add_space(18.0);
                        let typing = self.addr_typing;
                        self.tab_mut().address_focused = typing;
                        self.tab_mut().render_blocks(ui);
                        ui.add_space(60.0);
                    });
                });
            });
        });
        self.tab_mut().scroll_top = false;

        // The bar mirrors the tab's address -- but never over text the reader
        // typed. Rewriting the buffer mid-keystroke is what made typing feel
        // like it was being eaten and the caret jump back.
        let want = self.tab().address_text();
        if self.addr_hold && !self.tab().loading {
            self.addr_hold = false;
        }
        let focused = self.addr_typing;
        if should_sync_address(focused, self.addr_hold, self.addr_edited, &self.addr_input, &want) {
            self.addr_input = want;
            self.addr_edited = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        active_after_close, looks_like_host, net, normalize_path, parse_address,
        resolve_address, resolve_path, routes, shorten, should_sync_address, stash_fields,
        Address, Place, Trail,
    };
    use std::collections::HashMap;

    #[test]
    fn fields_survive_a_page_switch() {
        let mut saved: HashMap<u16, HashMap<String, String>> = HashMap::new();
        let mut typed = HashMap::new();
        typed.insert("name".to_string(), "阿离".to_string());

        stash_fields(&mut saved, Some(7), &typed);
        assert_eq!(saved.len(), 1);
        assert_eq!(
            saved.remove(&7).unwrap().get("name").map(String::as_str),
            Some("阿离")
        );
        // Nothing to stash when the form is empty or there is no page yet.
        stash_fields(&mut saved, Some(7), &HashMap::new());
        stash_fields(&mut saved, None, &typed);
        assert!(saved.is_empty());
    }

    fn sites() -> routes::Routes {
        routes::Routes::parse(
            "*nac.local 127.0.0.1:7000\niris.out 127.0.0.1:7002\nEX_MYTH 127.0.0.1:7003\n",
        )
    }

    #[test]
    fn page_paths_need_no_host() {
        let r = sites();
        assert_eq!(parse_address("/7", &r, "home"), known("home", "/7"));
        assert_eq!(parse_address("7", &r, "home"), known("home", "/7"));
        assert_eq!(parse_address("/", &r, "home"), known("home", "/"));
        assert_eq!(
            parse_address("  /about/  ", &r, "home"),
            known("home", "/about/")
        );
        assert_eq!(parse_address("about", &r, "home"), known("home", "/about"));
        assert_eq!(parse_address("/a//b/.", &r, "home"), known("home", "/a/b"));
    }

    fn entry(id: u16, path: &str) -> net::Entry {
        net::Entry {
            id,
            dynamic: false,
            hash: 0,
            title: String::new(),
            path: path.to_string(),
            matches: None,
        }
    }

    fn dynamic_entry(id: u16, path: &str, matches: &str) -> net::Entry {
        net::Entry {
            id,
            dynamic: true,
            hash: 0,
            title: String::new(),
            path: path.to_string(),
            matches: Some(matches.to_string()),
        }
    }

    #[test]
    fn paths_resolve_directories_and_404() {
        let entries = vec![entry(1, "/"), entry(2, "/notes/"), entry(3, "/notes/a")];
        assert_eq!(resolve_path(&entries, Some(1), "/"), Some(1));
        assert_eq!(resolve_path(&entries, Some(1), "/notes"), Some(2));
        assert_eq!(resolve_path(&entries, Some(1), "/notes/"), Some(2));
        assert_eq!(resolve_path(&entries, Some(1), "/notes/a"), Some(3));
        assert_eq!(resolve_path(&entries, Some(1), "/missing"), None);
        assert_eq!(normalize_path("//a/./b/").as_deref(), Some("/a/b/"));
    }

    fn known(site: &str, path: &str) -> Option<Address> {
        Some(Address::Known {
            site: site.to_string(),
            path: path.to_string(),
        })
    }

    fn unknown(site: &str, path: &str) -> Option<Address> {
        Some(Address::Unknown {
            site: site.to_string(),
            path: path.to_string(),
        })
    }

    #[test]
    fn hosts_select_the_site_and_path() {
        let r = sites();
        assert_eq!(
            parse_address("nac.local/", &r, "nac.local"),
            known("nac.local", "/")
        );
        assert_eq!(
            parse_address("nac.local/about", &r, "nac.local"),
            known("nac.local", "/about")
        );
        assert_eq!(
            parse_address("iris.out/notes", &r, "nac.local"),
            known("iris.out", "/notes")
        );
        assert_eq!(
            parse_address("EX_MYTH/3", &r, "nac.local"),
            known("EX_MYTH", "/3")
        );
    }

    #[test]
    fn named_sites_can_be_switched_by_name() {
        let r = sites();
        assert_eq!(
            parse_address("iris.out/3", &r, "nac.local"),
            known("iris.out", "/3")
        );
        assert_eq!(
            parse_address("nac://EX_MYTH/3", &r, "nac.local"),
            known("EX_MYTH", "/3")
        );
        assert_eq!(
            parse_address("iris.out", &r, "nac.local"),
            known("iris.out", "/")
        );
    }

    #[test]
    fn a_name_the_table_lacks_is_resolved_not_rejected() {
        let r = sites();
        // The whole name is kept: there is no suffix to strip.
        assert_eq!(
            parse_address("shop.nac.local/hats", &r, "nac.local"),
            unknown("shop.nac.local", "/hats")
        );
        assert_eq!(
            parse_address("NOWHERE.out/", &r, "nac.local"),
            unknown("NOWHERE.out", "/")
        );
    }

    #[test]
    fn a_learned_site_becomes_a_known_one() {
        let mut r = sites();
        assert_eq!(
            parse_address("iris.out/", &r, "nac.local"),
            known("iris.out", "/")
        );
        r.learn("nowhere.out", "10.0.0.5:7100", routes::TlsMode::On);
        assert_eq!(
            parse_address("nowhere.out/notes", &r, "nac.local"),
            known("nowhere.out", "/notes")
        );
        assert_eq!(
            parse_address("NOWHERE.OUT/notes", &r, "nac.local"),
            known("nowhere.out", "/notes")
        );
        assert_eq!(
            r.get("nowhere.out").map(|s| s.endpoint.as_str()),
            Some("10.0.0.5:7100")
        );
        assert_eq!(
            r.get("nowhere.out").map(|s| s.tls),
            Some(routes::TlsMode::On)
        );
    }

    #[test]
    fn back_crosses_a_site_boundary() {
        let mut trail = Trail::default();
        let home = Place {
            site: "home".into(),
            path: "/".into(),
        };
        let shop = Place {
            site: "shop".into(),
            path: "/hats".into(),
        };
        assert!(!trail.can_back());

        // Reading home, then following a link that lands on another site.
        trail.record(Some(home.clone()));
        assert!(trail.can_back());
        assert!(!trail.can_forward());

        assert_eq!(trail.go_back(Some(shop.clone())), Some(home.clone()));
        assert!(!trail.can_back());
        assert!(trail.can_forward());

        assert_eq!(trail.go_forward(Some(home.clone())), Some(shop));
    }

    #[test]
    fn a_new_step_drops_the_forward_branch() {
        let mut trail = Trail::default();
        let one = Place {
            site: "home".into(),
            path: "/one".into(),
        };
        let two = Place {
            site: "home".into(),
            path: "/two".into(),
        };
        let other = Place {
            site: "notes".into(),
            path: "/".into(),
        };
        trail.record(Some(one.clone()));
        assert_eq!(trail.go_back(Some(two.clone())), Some(one.clone()));
        assert!(trail.can_forward());

        // Going somewhere new makes the old "forward" unreachable.
        trail.record(Some(two.clone()));
        assert!(!trail.can_forward());

        // and the stack now holds `one` then `two`.
        assert_eq!(trail.go_back(Some(other)), Some(two));
        assert!(!trail.can_back());
    }

    #[test]
    fn a_name_is_whatever_the_file_calls_it() {
        // No suffix and no sub-domain rule: `iris.out` and `EX_MYTH` are just
        // names, the same way `nac.local` is.
        let r = sites();
        assert_eq!(parse_address("nac.local/", &r, "nac.local"), known("nac.local", "/"));
        assert_eq!(
            parse_address("iris.out/notes", &r, "nac.local"),
            known("iris.out", "/notes")
        );
        assert_eq!(
            parse_address("EX_MYTH/", &r, "nac.local"),
            known("EX_MYTH", "/")
        );
        // Case is not something a reader should have to remember.
        assert_eq!(
            parse_address("ex_myth/", &r, "nac.local"),
            known("EX_MYTH", "/")
        );
        // A name the table has not seen is looked up, not rejected.
        assert_eq!(
            parse_address("somewhere.else/", &r, "nac.local"),
            unknown("somewhere.else", "/")
        );
    }

    #[test]
    fn a_bare_word_is_a_path_and_a_host_shaped_one_is_not() {
        let r = sites();
        // No dot and no underscore: a path on the current site.
        assert_eq!(
            parse_address("about", &r, "nac.local"),
            known("nac.local", "/about")
        );
        assert_eq!(parse_address("7", &r, "nac.local"), known("nac.local", "/7"));
        // Names may use dots and underscores; spaces and colons are not names.
        assert!(looks_like_host("EX_MYTH"));
        assert!(looks_like_host("iris.out"));
        assert!(!looks_like_host("two words"));
        assert!(!looks_like_host(""));
        assert!(!looks_like_host("about"));
        assert!(!looks_like_host("127.0.0.1:7000"));
    }

    #[test]
    fn typing_is_never_overwritten_by_the_tab() {
        // The bug this guards: every frame the model won and the caret jumped
        // back, so keystrokes appeared to vanish.
        assert!(!should_sync_address(true, false, true, "nac.l", "nac.local/"));
        assert!(!should_sync_address(true, true, false, "", "nac.local/"));
        // A resolved address changes the tab under the reader's cursor; if they
        // have not touched the bar, it should follow.
        assert!(should_sync_address(true, false, false, "nac.local/", "iris.out/"));
        // Just committed an address: keep showing it while it loads, focused or
        // not -- otherwise the bar snaps back to the old address.
        assert!(!should_sync_address(false, true, false, "iris.out", "nac.local/"));
        assert!(!should_sync_address(true, true, true, "iris.out", "nac.local/"));
        // Idle and out of date: follow the tab.
        assert!(should_sync_address(false, false, false, "nac.local/", "iris.out/"));
        // Idle and already right: do nothing (no needless caret resets).
        assert!(!should_sync_address(false, false, false, "iris.out/", "iris.out/"));
    }

    #[test]
    fn a_long_title_is_cut_by_characters_not_bytes() {
        assert_eq!(shorten("short", 20), "short");
        assert_eq!(shorten("a".repeat(20).as_str(), 20), "a".repeat(20));
        assert_eq!(shorten(&"a".repeat(21), 20).chars().count(), 20);
        assert!(shorten(&"a".repeat(21), 20).ends_with('…'));
        // 20 Chinese characters are 60 bytes; cutting by bytes would panic.
        let chinese = "汉".repeat(30);
        let cut = shorten(&chinese, 20);
        assert_eq!(cut.chars().count(), 20);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn closing_a_tab_keeps_the_same_one_active() {
        // Three tabs, looking at the middle one, close the first: the tab that
        // was in the middle is now index 0 and must stay selected.
        assert_eq!(active_after_close(3, 1, 0), 0);
        // Close the active one: fall to whatever took its place.
        assert_eq!(active_after_close(3, 1, 1), 1);
        // Close the last while standing on it: step back.
        assert_eq!(active_after_close(3, 2, 2), 1);
        // Close something to the right: no change.
        assert_eq!(active_after_close(3, 0, 2), 0);
        // Two tabs, close the other one.
        assert_eq!(active_after_close(2, 1, 0), 0);
    }

    #[test]
    fn a_prefix_page_answers_the_whole_family() {
        let entries = vec![
            entry(1, "/"),
            dynamic_entry(2, "/item/", "/item/"),
            dynamic_entry(3, "/item/special/", "/item/special/"),
        ];
        // Exact match keeps naming the page's own address.
        assert_eq!(
            resolve_address(&entries, Some(1), "/item/").map(|r| (r.id, r.path)),
            Some((2, "/item/".to_string()))
        );
        // Anything under the prefix goes to the same page, and the page is told
        // which address was asked for.
        assert_eq!(
            resolve_address(&entries, Some(1), "/item/123").map(|r| (r.id, r.path)),
            Some((2, "/item/123".to_string()))
        );
        // The longest declared prefix wins, so a more specific page can exist.
        assert_eq!(
            resolve_address(&entries, Some(1), "/item/special/9").map(|r| r.id),
            Some(3)
        );
        // No prefix matches: still a 404.
        assert_eq!(resolve_address(&entries, Some(1), "/nope"), None);
    }

    #[test]
    fn endpoints_are_not_addresses() {
        let r = sites();
        // An endpoint is not something a reader should ever paste, so it is
        // refused instead of becoming a 404 on the current site.
        assert_eq!(parse_address("127.0.0.1:7000/1", &r, "nac.local"), None);
        assert_eq!(parse_address("[::1]:7000/", &r, "nac.local"), None);
        // A dot does not make a name wrong — it makes it worth looking up.
        assert_eq!(
            parse_address("example.test", &r, "nac.local"),
            unknown("example.test", "/")
        );
        assert_eq!(parse_address("/../secret", &r, "nac.local"), None);
    }
}
