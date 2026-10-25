//! 窗口：标签栏、地址栏、状态栏、内容区，以及每个标签页的消息泵。

use crate::*;

impl eframe::App for Viewer {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // Every tab keeps working while it is in the background: a page still
        // loading in one tab does not stall because another tab is on screen.
        for index in 0..self.tabs.len() {
            for msg in self.tabs[index].fetcher.drain() {
                match msg {
                    fetch::Msg::Manifest(entries) => self.tabs[index].accept_manifest(entries),
                    fetch::Msg::Page { id, text, reason } => {
                        self.tabs[index].accept_page(id, text, reason)
                    }
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
