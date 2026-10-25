//! 一个标签页的"行为"：地址、连接、取页、前进后退、表单提交。
//!
//! 这里没有一行画东西的代码 —— 那部分在 `render.rs`。

use crate::*;

impl Tab {
    pub(crate) fn new(uid: u64, routes: &routes::Routes, site: String, target: Option<Target>) -> Self {
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
            saved: HashMap::new(),
            address_focused: false,
            focus_input: false,
            saved_fields: HashMap::new(),
            sheet: HashMap::new(),
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
    pub(crate) fn host(&self) -> &str {
        &self.site
    }

    pub(crate) fn path_of(&self, id: u16) -> Option<String> {
        self.entries
            .iter()
            .find(|e| e.id == id)
            .map(|e| e.path.clone())
            .filter(|path| !path.is_empty())
    }

    /// What the user sees: a domain-like origin plus the page path, never an
    /// endpoint or an internal page number.
    pub(crate) fn address_text(&self) -> String {
        let path = self
            .shown_path
            .clone()
            .or_else(|| self.page_id.and_then(|id| self.path_of(id)))
            .unwrap_or_else(|| "/".to_string());
        format!("{}{}", self.host(), path)
    }

    pub(crate) fn bootstrap(&mut self, routes: &routes::Routes) {
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
    pub(crate) fn accept_resolved(
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

    pub(crate) fn accept_manifest(&mut self, entries: Option<Vec<net::Entry>>) {
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

    pub(crate) fn connect(&mut self, routes: &routes::Routes, site: String, target: Option<Target>) {
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

    pub(crate) fn go(&mut self, routes: &mut routes::Routes, input: &str) {
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

    pub(crate) fn reload(&mut self, routes: &routes::Routes) {
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

    pub(crate) fn resolve_path(&self, path: &str) -> Option<Resolved> {
        resolve_address(&self.entries, self.home, path)
    }

    pub(crate) fn open_path(&mut self, path: &str) {
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

    pub(crate) fn fail(&mut self, message: impl Into<String>) {
        self.failed = true;
        self.error = message.into();
        self.title.clear();
        self.blocks.clear();
        self.form_target = None;
    }

    pub(crate) fn load(&mut self, id: u16) {
        self.load_at(id, String::new());
    }

    /// Loads a page and tells it which address was asked for. An empty `path`
    /// means "the page's own address", which is what a plain page-number request
    /// means.
    pub(crate) fn load_at(&mut self, id: u16, path: String) {
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

    pub(crate) fn accept_page(
        &mut self,
        id: u16,
        text: Option<String>,
        reason: Option<protocol::EmptyReason>,
    ) {        self.loading = false;
        match text {
            Some(text) => self.apply(id, &text),
            None => {
                self.page_id = Some(id);
                // 「没有这一页」和「这一页崩了」是两件事，不能共用一句话。
                self.fail(match reason {
                    Some(protocol::EmptyReason::Missing) => "没有这一页",
                    Some(protocol::EmptyReason::Refused) => "服务器不肯给这一页",
                    Some(protocol::EmptyReason::Failed) => "页面出错了",
                    // 服务器没说是哪一种：跟以前的文案一致，不猜。
                    _ => "页面加载失败",
                });
            }
        }
    }

    /// 附件存好了（或者没存成）。只有点过「保存」的那一个 id 会变。
    pub(crate) fn accept_saved(&mut self, id: u16, result: Result<String, String>) {
        let state = match result {
            Ok(path) => SaveState::Done(path),
            Err(why) => SaveState::Failed(why),
        };
        self.saved.insert(id, state);
    }

    pub(crate) fn apply(&mut self, id: u16, text: &str) {
        self.blocks = markup::parse(text);
        // 上一页的附件进度不该跟着这一页走：id 是页内的，换了页就不作数了。
        self.saved.clear();
        // 窗口标题 = 正文里第一个标题。必须往嵌套里找 —— 页面常常把整个正文包在
        // `[[box class=page]]` 里，只看顶层就一个标题都找不到（文章页曾经因此
        // 一律显示发布时的固定标题）。
        self.title = crate::render::first_heading(&self.blocks)
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
    pub(crate) fn place(&self) -> Option<Place> {
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
    pub(crate) fn go_place(&mut self, routes: &routes::Routes, place: Place) {
        if place.site == self.site {
            self.open_path(&place.path);
        } else {
            self.connect(routes, place.site, Some(Target::Path(place.path)));
        }
    }

    pub(crate) fn navigate(&mut self, target: u16) {
        self.navigate_at(target, String::new());
    }

    pub(crate) fn navigate_at(&mut self, target: u16, path: String) {
        if let Some(current) = self.page_id {
            if current != target || !path.is_empty() {
                let from = self.place();
                self.trail.record(from);
            }
        }
        self.load_at(target, path);
    }

    pub(crate) fn back(&mut self, routes: &routes::Routes) {
        let current = self.place();
        if let Some(prev) = self.trail.go_back(current) {
            self.go_place(routes, prev);
        }
    }

    pub(crate) fn forward(&mut self, routes: &routes::Routes) {
        let current = self.place();
        if let Some(next) = self.trail.go_forward(current) {
            self.go_place(routes, next);
        }
    }

    pub(crate) fn home_page(&mut self) {
        self.open_path("/");
    }

    pub(crate) fn save_fields(&mut self) {
        stash_fields(&mut self.saved_fields, self.page_id, &self.fields);
    }

    pub(crate) fn submit(&mut self) {
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
}
