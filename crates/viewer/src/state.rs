//! 状态：`Tab` 是一个标签页的全部状态，`Viewer` 是窗口（标签页 + 地址栏）。

use crate::*;


pub(crate) struct Tab {
    /// Stable across tab reordering, so per-tab widget state (scroll offset,
    /// texture names) survives closing another tab.
    pub(crate) uid: u64,
    pub(crate) fetcher: fetch::Fetcher,
    pub(crate) loading: bool,
    pub(crate) site: String,
    #[cfg(feature = "admin")]
    pub(crate) tunnel: Option<tunnel::Tunnel>,
    pub(crate) entries: Vec<net::Entry>,
    pub(crate) page_id: Option<u16>,
    pub(crate) title: String,
    pub(crate) blocks: Vec<Block>,
    pub(crate) trail: Trail,
    pub(crate) home: Option<u16>,
    pub(crate) textures: HashMap<u16, egui::TextureHandle>,
    pub(crate) fields: HashMap<String, String>,
    pub(crate) form_target: Option<u16>,
    pub(crate) failed: bool,
    pub(crate) error: String,
    pub(crate) scroll_top: bool,
    pub(crate) loader: loader::Loader,
    /// Whether the address bar has the caret. A page must not steal focus from
    /// it, or keystrokes meant for the address vanish into a form field.
    pub(crate) address_focused: bool,
    pub(crate) focus_input: bool,
    pub(crate) saved_fields: HashMap<u16, HashMap<String, String>>,
    /// `[[sheet]]` rules, gathered before the first box is drawn.
    pub(crate) sheet: HashMap<String, Style>,
    pub(crate) shown_path: Option<String>,
    /// When the current page's `[[scene]]` timelines started. Reset on every
    /// page load, so a reload replays the animation instead of resuming it.
    pub(crate) scene_clock: Option<std::time::Instant>,
    pub(crate) target: Option<Target>,
    /// Set while a site name is being looked up: the path to open once it arrives.
    pub(crate) pending_path: Option<String>,
    /// The name currently being looked up, for the status line.
    pub(crate) resolving: Option<String>,
}

/// The window: a list of tabs plus the address-bar state, which belongs to the
/// window rather than to any one tab.
pub(crate) struct Viewer {
    pub(crate) tabs: Vec<Tab>,
    pub(crate) active: usize,
    pub(crate) routes: routes::Routes,
    pub(crate) addr_input: String,
    pub(crate) addr_focus: bool,
    pub(crate) addr_typing: bool,
    pub(crate) addr_hold: bool,
    /// Set the moment the reader edits the bar, cleared when the text is
    /// accepted or dropped. Keeps the page from rewriting what is being typed.
    pub(crate) addr_edited: bool,
    pub(crate) last_title: String,
}

/// Which tab should be active after `index` is removed.
///
/// Separate from the viewer so the arithmetic is testable: closing a tab to the
/// left of the active one must not silently select somebody else.
pub(crate) fn active_after_close(len: usize, active: usize, index: usize) -> usize {
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


pub(crate) static NEXT_TAB_UID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl Viewer {
    pub(crate) fn new(
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

    pub(crate) fn tab(&self) -> &Tab {
        &self.tabs[self.active.min(self.tabs.len() - 1)]
    }

    pub(crate) fn tab_mut(&mut self) -> &mut Tab {
        let index = self.active.min(self.tabs.len() - 1);
        &mut self.tabs[index]
    }

    pub(crate) fn open_tab(&mut self, site: String, target: Option<Target>) {
        let uid = NEXT_TAB_UID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.tabs.push(Tab::new(uid, &self.routes, site, target));
        self.active = self.tabs.len() - 1;
        self.addr_hold = false;
        self.addr_edited = false;
    }

    /// Opens the default site in a new tab. Used by the `+` button and `Ctrl+T`.
    pub(crate) fn new_tab(&mut self) {
        let site = self
            .routes
            .default_site()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "dev".to_string());
        self.open_tab(site, None);
    }

    /// Closes one tab. The last tab is never closed, so the window always has
    /// somewhere to draw.
    pub(crate) fn close_tab(&mut self, index: usize) {
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

impl Viewer {
    /// Runs `f` against the active tab while handing it the site table. The two
    /// live in separate fields, so the borrow checker can see they do not alias.
    pub(crate) fn with_tab<F: FnOnce(&mut Tab, &routes::Routes)>(&mut self, f: F) {
        let index = self.active.min(self.tabs.len() - 1);
        let routes = &self.routes;
        f(&mut self.tabs[index], routes);
    }

    pub(crate) fn with_tab_routes<F: FnOnce(&mut Tab, &mut routes::Routes)>(&mut self, f: F) {
        let index = self.active.min(self.tabs.len() - 1);
        let routes = &mut self.routes;
        f(&mut self.tabs[index], routes);
    }
}
