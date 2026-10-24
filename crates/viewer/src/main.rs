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

use markup::{Block, Inline};

fn main() -> eframe::Result<()> {
    let mut addr: Option<String> = std::env::var("NAC_ADDR").ok();
    let mut routes_path: Option<String> = None;
    let mut site_name: Option<String> = None;
    let mut selftest = false;
    let mut encode: Option<String> = None;
    let mut list_sites = false;
    let mut start_page: Option<u16> = None;

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
            _ => {}
        }
        i += 1;
    }

    if let Some(value) = encode {
        println!("{}", routes::encode(&value));
        return Ok(());
    }

    let routes = build_routes(routes_path.as_deref(), addr);
    if list_sites {
        let embedded = routes::is_embedded();
        for site in &routes.sites {
            if embedded {
                println!("{}{}", if site.default { "*" } else { " " }, site.name);
            } else {
                println!(
                    "{}{}\t{}",
                    if site.default { "*" } else { " " },
                    site.name,
                    site.endpoint
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
        Box::new(move |cc| Ok(Box::new(Viewer::new(cc, routes, site, start_page)))),
    )
}

/// An explicit `--addr` wins (operator escape hatch); then an endpoint embedded
/// at build time via `NAC_EMBED_ROUTES`; otherwise the shipped directory decides.
fn build_routes(explicit: Option<&str>, addr: Option<String>) -> routes::Routes {
    if let Some(fallback) = addr {
        return routes::Routes::single("dev", &fallback);
    }
    if let Some(routes) = routes::Routes::embedded() {
        return routes;
    }
    if let Some(path) = routes::Routes::locate(explicit) {
        if let Some(routes) = routes::Routes::load(&path) {
            return routes;
        }
        eprintln!("无法读取站点目录 {}", path.display());
    }
    routes::Routes::default()
}

fn selftest_run(routes: &routes::Routes, site_name: &str) -> eframe::Result<()> {
    let Some(site) = routes.get(site_name) else {
        println!("site {site_name}: missing");
        return Ok(());
    };
    let mut client = net::Client::new(site.endpoint.clone());
    match client.manifest() {
        Ok(entries) => {
            println!("site {site_name}: {} entries", entries.len());
            for e in &entries {
                println!(
                    "  {}\t{}\t{:016x}\t{}",
                    e.id,
                    if e.dynamic { "dyn " } else { "stat" },
                    e.hash,
                    e.title
                );
            }
            if let Some(id) = entries.iter().map(|e| e.id).min() {
                match client.page(id) {
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
            match client.submit(7, vec![("name".to_string(), "selftest".to_string())]) {
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

/// Reads an address the way a user types it. Returns `(site, page)`.
///
/// Accepted: `/7`, `7`, `/` (home), `home/7`, `nac://home/7`.
/// Endpoints are not accepted here on purpose — they live in the routes file,
/// so nothing the user types or sees has to contain one.
fn parse_address(
    input: &str,
    routes: &routes::Routes,
    current_site: &str,
) -> Option<(String, Option<u16>)> {
    let mut s = input.trim();
    if let Some(rest) = s.strip_prefix("nac://") {
        s = rest;
    }
    let s = s.trim_end_matches('/');
    let bare_number = s.strip_prefix('/').unwrap_or(s);

    if s.is_empty() {
        return Some((current_site.to_string(), None));
    }
    if bare_number.is_empty() {
        return Some((current_site.to_string(), None));
    }
    if !bare_number.contains('/') {
        if let Ok(id) = bare_number.parse::<u16>() {
            return Some((current_site.to_string(), Some(id)));
        }
    }

    let (name, tail) = match s.split_once('/') {
        Some((name, tail)) => (name, tail),
        None => (s, ""),
    };
    if name.is_empty() || routes.get(name).is_none() {
        return None;
    }
    let page = if tail.is_empty() {
        None
    } else {
        Some(tail.parse::<u16>().ok()?)
    };
    Some((name.to_string(), page))
}

// ------------------------------------------------------------------ viewer

struct Viewer {
    fetcher: fetch::Fetcher,
    loading: bool,
    routes: routes::Routes,
    site: String,
    entries: Vec<net::Entry>,
    page_id: Option<u16>,
    title: String,
    blocks: Vec<Block>,
    history: Vec<u16>,
    forward: Vec<u16>,
    home: Option<u16>,
    textures: HashMap<u16, egui::TextureHandle>,
    fields: HashMap<String, String>,
    form_target: Option<u16>,
    failed: bool,
    error: String,
    scroll_top: bool,
    loader: loader::Loader,
    last_title: String,
    focus_input: bool,
    addr_input: String,
    addr_focus: bool,
    addr_typing: bool,
    pending_sync: bool,
    pending_page: Option<u16>,
    saved_fields: HashMap<u16, HashMap<String, String>>,
    show_index: bool,
    target: Option<u16>,
}

impl Viewer {
    fn new(
        cc: &eframe::CreationContext<'_>,
        routes: routes::Routes,
        site: String,
        start_page: Option<u16>,
    ) -> Self {
        install_fonts(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(egui::Visuals::light());

        let endpoint = routes
            .get(&site)
            .map(|s| s.endpoint.clone())
            .unwrap_or_default();

        let mut v = Self {
            fetcher: fetch::Fetcher::new(endpoint.clone()),
            loading: false,
            routes,
            site,
            entries: Vec::new(),
            page_id: None,
            title: String::new(),
            blocks: Vec::new(),
            history: Vec::new(),
            forward: Vec::new(),
            home: None,
            textures: HashMap::new(),
            fields: HashMap::new(),
            form_target: None,
            failed: false,
            error: String::new(),
            scroll_top: true,
            loader: loader::Loader::new(endpoint),
            last_title: String::new(),
            focus_input: false,
            addr_input: String::new(),
            addr_focus: false,
            addr_typing: false,
            pending_sync: false,
            pending_page: None,
            saved_fields: HashMap::new(),
            show_index: false,
            target: start_page,
        };
        v.addr_input = v.address_text();
        v.bootstrap();
        v
    }

    /// What the user sees: a page on a site, never an endpoint.
    fn address_text(&self) -> String {
        let page = match self.page_id {
            Some(id) => id.to_string(),
            None => String::new(),
        };
        if self.routes.sites.len() > 1 {
            format!("{}/{page}", self.site)
        } else {
            format!("/{page}")
        }
    }

    fn bootstrap(&mut self) {
        if self.routes.is_empty() {
            self.fail("找不到站点目录 nac.routes");
            return;
        }
        self.loading = true;
        self.fetcher.manifest();
    }

    /// Applies a manifest that arrived from the worker thread.
    fn accept_manifest(&mut self, entries: Option<Vec<net::Entry>>) {
        match entries {
            Some(entries) => {
                self.entries = entries;
                self.failed = false;
                self.error.clear();
                self.home = self.entries.iter().map(|e| e.id).min();
                match self.target.take().or(self.home) {
                    Some(id) => self.load(id),
                    None => {
                        self.loading = false;
                        self.fail("这个站点还没有任何页面");
                    }
                }
            }
            None => {
                self.loading = false;
                self.entries.clear();
                self.home = None;
                self.page_id = None;
                self.fail("无法连接到服务器");
            }
        }
    }

    fn connect(&mut self, site: String, page: Option<u16>) {
        let endpoint = self
            .routes
            .get(&site)
            .map(|s| s.endpoint.clone())
            .unwrap_or_default();
        self.site = site;
        self.fetcher = fetch::Fetcher::new(endpoint.clone());
        self.loader = loader::Loader::new(endpoint);
        self.textures.clear();
        self.entries.clear();
        self.history.clear();
        self.forward.clear();
        self.saved_fields.clear();
        self.home = None;
        self.page_id = None;
        self.blocks.clear();
        self.title.clear();
        self.fields.clear();
        self.form_target = None;
        self.error.clear();
        self.failed = false;
        self.scroll_top = true;
        self.target = page;
        self.bootstrap();
    }

    fn go(&mut self, input: &str) {
        let Some((site, page)) = parse_address(input, &self.routes, &self.site) else {
            self.failed = true;
            self.error = "地址无效".to_string();
            self.pending_sync = true;
            return;
        };
        self.pending_sync = true;
        if site != self.site {
            self.connect(site, page);
            return;
        }
        match page {
            Some(id) if self.page_id != Some(id) => {
                self.failed = false;
                self.error.clear();
                self.navigate(id);
            }
            Some(_) => {}
            None => self.reload(),
        }
    }

    fn reload(&mut self) {
        self.error.clear();
        self.failed = false;
        match self.page_id {
            Some(id) => self.load(id),
            None => self.bootstrap(),
        }
    }

    fn fail(&mut self, message: impl Into<String>) {
        self.failed = true;
        self.error = message.into();
        self.title.clear();
        self.blocks.clear();
        self.form_target = None;
    }

    fn load(&mut self, id: u16) {
        self.save_fields();
        if !self.entries.iter().any(|e| e.id == id) {
            self.loading = false;
            self.page_id = Some(id);
            self.fail(format!("没有第 {id} 页"));
            return;
        }
        self.loading = true;
        self.pending_page = Some(id);
        self.fetcher.page(id);
    }

    fn accept_page(&mut self, id: u16, text: Option<String>) {
        self.loading = false;
        self.pending_page = None;
        match text {
            Some(text) => self.apply(id, &text),
            None => {
                self.page_id = Some(id);
                self.fail(format!("无法获取第 {id} 页"));
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
        self.fields = self.saved_fields.remove(&id).unwrap_or_default();
        self.form_target = None;
        self.failed = false;
        self.error.clear();
        self.scroll_top = true;
        self.focus_input = true;
    }

    fn navigate(&mut self, target: u16) {
        if let Some(current) = self.page_id {
            if current != target {
                self.history.push(current);
            }
        }
        self.forward.clear();
        self.load(target);
    }

    fn back(&mut self) {
        if let Some(prev) = self.history.pop() {
            if let Some(current) = self.page_id {
                self.forward.push(current);
            }
            self.load(prev);
        }
    }

    fn forward(&mut self) {
        if let Some(next) = self.forward.pop() {
            if let Some(current) = self.page_id {
                self.history.push(current);
            }
            self.load(next);
        }
    }

    fn home_page(&mut self) {
        if let Some(home) = self.home {
            if self.page_id != Some(home) {
                self.navigate(home);
            }
        }
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
        if let Some(current) = self.page_id {
            self.history.push(current);
        }
        self.forward.clear();
        // The submission consumed what was typed; coming back here later should
        // start clean.
        self.saved_fields.remove(&target);
        self.loading = true;
        self.pending_page = Some(target);
        self.fetcher.submit(target, fields);
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
                    ui.label(self.job(&parts, 17.0, body, FontFamily::Proportional));
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
                Block::Pre(text) => {
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
                        self.navigate(target);
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
                        if self.focus_input {
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

impl eframe::App for Viewer {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        for msg in self.fetcher.drain() {
            match msg {
                fetch::Msg::Manifest(entries) => self.accept_manifest(entries),
                fetch::Msg::Page { id, text } => self.accept_page(id, text),
            }
        }
        if self.fetcher.busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        for msg in self.loader.drain() {
            if let loader::Msg::Done(loaded) = msg {
                let color = egui::ColorImage::from_rgba_unmultiplied(
                    [loaded.width as usize, loaded.height as usize],
                    &loaded.rgba,
                );
                let tex = ctx.load_texture(
                    format!("img-{}", loaded.id),
                    color,
                    egui::TextureOptions::LINEAR,
                );
                self.textures.insert(loaded.id, tex);
            }
        }
        if self.loader.busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        if self.title != self.last_title {
            self.last_title = self.title.clone();
            let title = if self.title.is_empty() {
                "nac".to_string()
            } else {
                format!("{} — nac", self.title)
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
        }

        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::L)) {
            self.addr_focus = true;
        }
        let typing = ctx.egui_wants_keyboard_input();
        if ctx.input(|i| i.key_pressed(egui::Key::F5))
            || (!typing && ctx.input(|i| i.key_pressed(egui::Key::R)))
        {
            self.reload();
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Enter)) {
            self.submit();
        }
        if (ctx.input(|i| i.key_pressed(egui::Key::Escape)) && !self.addr_typing)
            || ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowLeft))
        {
            self.back();
        }
        if ctx.input(|i| i.modifiers.alt && i.key_pressed(egui::Key::ArrowRight)) {
            self.forward();
        }

        self.index_window(&ctx);

        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !self.history.is_empty(),
                        egui::Button::new(RichText::new("←").size(18.0)),
                    )
                    .on_hover_text("返回  (Esc / Alt+←)")
                    .clicked()
                {
                    self.back();
                }
                if ui
                    .add_enabled(
                        !self.forward.is_empty(),
                        egui::Button::new(RichText::new("→").size(18.0)),
                    )
                    .on_hover_text("前进  (Alt+→)")
                    .clicked()
                {
                    self.forward();
                }
                if ui
                    .add(egui::Button::new(RichText::new("⟳").size(16.0)))
                    .on_hover_text("重新载入当前页  (F5)")
                    .clicked()
                {
                    self.reload();
                }
                if ui
                    .add(egui::Button::new(RichText::new("⌂").size(16.0)))
                    .on_hover_text("回到首页")
                    .clicked()
                {
                    self.home_page();
                }

                let width = (ui.available_width() - 76.0).max(120.0);
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.addr_input)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(width)
                        .desired_rows(1)
                        .hint_text("/页码"),
                );
                if self.addr_focus {
                    resp.request_focus();
                    self.addr_focus = false;
                }
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let input = self.addr_input.clone();
                    self.go(&input);
                }
                self.addr_typing = resp.has_focus();

                if ui
                    .add(egui::Button::new(RichText::new("索引").size(13.0)))
                    .on_hover_text("列出本站所有页面")
                    .clicked()
                {
                    self.show_index = true;
                }
            });
            ui.add_space(4.0);
        });

        egui::Panel::bottom("bottom").show(ui, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                let bad = self.failed || !self.error.is_empty();
                let color = if bad {
                    Color32::from_rgb(190, 70, 70)
                } else {
                    Color32::from_rgb(90, 160, 90)
                };
                ui.painter().circle_filled(rect.center(), 4.0, color);
                let status = if !self.error.is_empty() {
                    self.error.clone()
                } else if self.loading {
                    match self.pending_page {
                        Some(id) => format!("{} · 正在取第 {id} 页…", self.site),
                        None => format!("{} · 正在连接…", self.site),
                    }
                } else if self.failed {
                    format!("{} · 无法获取内容", self.site)
                } else {
                    format!("{} · 已连接", self.site)
                };
                ui.label(RichText::new(status).weak().size(12.0));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.label(
                        RichText::new("Ctrl+L 地址 · Esc 返回 · F5 刷新")
                            .weak()
                            .size(12.0),
                    );
                });
            });
            ui.add_space(2.0);
        });

        let reset_scroll = self.scroll_top;
        egui::CentralPanel::default().show(ui, |ui| {
            let mut area = egui::ScrollArea::vertical().auto_shrink([false, false]);
            if reset_scroll {
                area = area.vertical_scroll_offset(0.0);
            }
            area.show(ui, |ui| {
                    if self.failed {
                        ui.add_space(140.0);
                        ui.vertical_centered(|ui| {
                            let message = if self.error.is_empty() {
                                "无法获取内容".to_string()
                            } else {
                                self.error.clone()
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
                            self.render_blocks(ui);
                            ui.add_space(60.0);
                        });
                    });
                });
        });
        self.scroll_top = false;

        let want = self.address_text();
        if self.addr_input != want && (!self.addr_typing || self.pending_sync) {
            self.addr_input = want;
            self.pending_sync = false;
        }
    }
}

impl Viewer {
    fn index_window(&mut self, ctx: &egui::Context) {
        if !self.show_index {
            return;
        }
        let entries = self.entries.clone();
        let current = self.page_id;
        let mut open = true;
        let mut go_to = None;

        egui::Window::new("站点索引")
            .open(&mut open)
            .resizable(false)
            .default_width(380.0)
            .show(ctx, |ui| {
                if entries.is_empty() {
                    ui.label(RichText::new("没有可显示的页面").weak());
                    return;
                }
                for e in &entries {
                    let mark = if Some(e.id) == current { "▸" } else { " " };
                    let title = if e.title.is_empty() {
                        "(无标题)"
                    } else {
                        e.title.as_str()
                    };
                    let kind = if e.dynamic { "  动态" } else { "" };
                    let text = format!("{mark} {:<4}{title}{kind}", e.id);
                    if ui
                        .selectable_label(Some(e.id) == current, RichText::new(text).size(15.0))
                        .clicked()
                    {
                        go_to = Some(e.id);
                    }
                }
            });

        if let Some(id) = go_to {
            self.show_index = false;
            self.navigate(id);
        } else if !open {
            self.show_index = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_address, routes, stash_fields};
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
        routes::Routes::parse("home 127.0.0.1:7000\narchive 127.0.0.1:7001\n")
    }

    #[test]
    fn page_paths_need_no_host() {
        let r = sites();
        assert_eq!(
            parse_address("/7", &r, "home"),
            Some(("home".to_string(), Some(7)))
        );
        assert_eq!(
            parse_address("7", &r, "home"),
            Some(("home".to_string(), Some(7)))
        );
        assert_eq!(
            parse_address("/", &r, "home"),
            Some(("home".to_string(), None))
        );
        assert_eq!(
            parse_address("  /12  ", &r, "home"),
            Some(("home".to_string(), Some(12)))
        );
    }

    #[test]
    fn named_sites_can_be_switched_by_name() {
        let r = sites();
        assert_eq!(
            parse_address("archive/3", &r, "home"),
            Some(("archive".to_string(), Some(3)))
        );
        assert_eq!(
            parse_address("nac://archive/3", &r, "home"),
            Some(("archive".to_string(), Some(3)))
        );
        assert_eq!(
            parse_address("archive", &r, "home"),
            Some(("archive".to_string(), None))
        );
    }

    #[test]
    fn endpoints_are_not_addresses() {
        let r = sites();
        assert_eq!(parse_address("127.0.0.1:7000/1", &r, "home"), None);
        assert_eq!(parse_address("example.test", &r, "home"), None);
        assert_eq!(parse_address("/abc", &r, "home"), None);
        assert_eq!(parse_address("/70000", &r, "home"), None);
    }
}
