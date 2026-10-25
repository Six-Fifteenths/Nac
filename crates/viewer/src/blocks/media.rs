//! 媒体：图片、说明文字、附件、头像、`[[scene]]` 动画、`[[term]]` 终端。

use crate::*;

/// `1024` → `1.0 KB`。附件的大小只用来看个大概，不追求精确。
pub(crate) fn bytes_text(size: u32) -> String {
    let n = size as f32;
    if n >= 1024.0 * 1024.0 {
        format!("{:.1} MB", n / (1024.0 * 1024.0))
    } else if n >= 1024.0 {
        format!("{:.0} KB", n / 1024.0)
    } else {
        format!("{size} B")
    }
}

impl Tab {
    pub(crate) fn render_image(&mut self, ui: &mut egui::Ui, id: u16, pct: f32, ratio: Option<f32>) {
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

    /// `!file`：一行「文件名 · 大小 · 保存」。点了才去取字节，取到就直接落进
    /// 系统下载目录 —— 页面说不出路径，也覆盖不掉已有文件。
    pub(crate) fn render_file(&mut self, ui: &mut egui::Ui, id: u16, name: &str, size: u32) {
        let state = self.saved.get(&id).cloned();
        egui::Frame::new()
            .fill(Color32::from_gray(247))
            .stroke(egui::Stroke::new(1.0, Color32::from_gray(222)))
            .corner_radius(egui::CornerRadius::same(4))
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(name).size(15.0).strong());
                    ui.label(RichText::new(bytes_text(size)).size(13.0).weak());
                    ui.add_space(8.0);
                    // 按钮一直在：想再存一份就再点一次，第二份会自动叫「名字 (2)」。
                    // 只有正在取的那一下没有按钮 —— 那时点了也只会排第二份，没意义。
                    if matches!(state, Some(SaveState::Working)) {
                        ui.label(RichText::new("保存中…").size(13.0).weak());
                    } else if ui.button(RichText::new("保存").size(14.0)).clicked() {
                        self.saved.insert(id, SaveState::Working);
                        self.fetcher.save(id, name.to_string());
                    }
                    match &state {
                        Some(SaveState::Done(path)) => {
                            let short = save::safe_name(name);
                            ui.label(RichText::new(format!("已存到 {short}")).size(13.0).weak())
                                .on_hover_text(path);
                        }
                        Some(SaveState::Failed(why)) => {
                            ui.label(
                                RichText::new(format!("没存成：{why}"))
                                    .size(13.0)
                                    .color(Color32::from_rgb(170, 60, 60)),
                            );
                        }
                        _ => {}
                    }
                });
            });
        ui.add_space(8.0);
    }

    /// Renders a run of inline content. When it contains links they become real
    /// widgets and the text flows around them; otherwise it is one laid-out job.
    ///
    /// `wrap` is off inside table cells: there the surrounding `Grid` decides
    /// column widths from what each cell asks for, and a wrapping layout asks for
    /// its minimum, which splits labels like "分栏实验" across two lines.

    pub(crate) fn render_scene(
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

    /// Draws one `[[box]]`: margins, then a frame with background, border,
    /// shadow and padding, then its children either stacked or in a row.
    ///
    /// Width is never the page's business: the box takes whatever room the
    /// parent gives it.

    pub(crate) fn render_avatar(&mut self, ui: &mut egui::Ui, name: &str, size: f32) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        let painter = ui.painter();
        let seed = fnv1a(name.as_bytes());
        let hue = (seed % 360) as f32 / 360.0;
        let fill = Color32::from(egui::ecolor::Hsva::new(hue, 0.52, 0.72, 1.0));
        let radius = (size * 0.14).clamp(2.0, 10.0) as u8;
        painter.rect_filled(rect, egui::CornerRadius::same(radius), fill);
        let letter = name
            .chars()
            .find(|c| !c.is_whitespace())
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_else(|| "?".to_string());
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            letter,
            FontId::new(size * 0.46, Text::default().bold_family()),
            Color32::WHITE,
        );
    }


    pub(crate) fn render_term(&mut self, ui: &mut egui::Ui, term: &markup::Term) {
        let bg = if term.bare { Color32::BLACK } else { TERM_BG };
        let body = if term.bare { Color32::WHITE } else { TERM_FG };
        let width = ui.available_width().max(280.0);
        if let Some(target) = term.target {
            self.form_target = Some(target);
        }

        let border = if term.bare {
            egui::Stroke::NONE
        } else {
            egui::Stroke::new(2.0, TERM_BORDER)
        };
        egui::Frame::new()
            .fill(bg)
            .stroke(border)
            .corner_radius(egui::CornerRadius::same(3))
            .show(ui, |ui| {
                ui.set_width(width - 4.0);
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                if !term.bare {
                    let bar_w = ui.available_width();
                    let (title_rect, _) =
                        ui.allocate_exact_size(egui::vec2(bar_w, 30.0), egui::Sense::hover());
                    ui.painter().rect_filled(title_rect, 0, TERM_BORDER);
                    ui.painter().text(
                        egui::pos2(title_rect.left() + 10.0, title_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        &term.title,
                        egui::FontId::proportional(17.0),
                        TERM_BG,
                    );
                    // Four hairlines under the title, each one pixel thinner than
                    // the last — the same taper the original uses.
                    for (height, gap) in [(4.0, 1.0), (3.0, 2.0), (2.0, 3.0), (1.0, 4.0)] {
                        ui.add_space(gap);
                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(bar_w, height),
                            egui::Sense::hover(),
                        );
                        ui.painter().rect_filled(rect, 0, TERM_BORDER);
                    }
                }

                ui.add_space(10.0);
                // Left inset matches the original's `#terminal { margin-left: 20px }`.
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 16,
                        right: 8,
                        top: 0,
                        bottom: 10,
                    })
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
                        for line in &term.lines {
                            match line {
                                markup::TermLine::Ascii(text) => {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(text)
                                                .font(egui::FontId::monospace(14.0))
                                                .line_height(Some(16.0))
                                                .color(if term.bare {
                                                    Color32::WHITE
                                                } else {
                                                    TERM_ASCII
                                                }),
                                        )
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                    );
                                }
                                markup::TermLine::Text(runs) => {
                                    let blank = runs.is_empty()
                                        || (runs.len() == 1 && runs[0].1.is_empty());
                                    if blank {
                                        ui.add_space(TERM_LINE);
                                        continue;
                                    }
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 0.0;
                                        for (role, text) in runs {
                                            self.term_run(ui, *role, text, body);
                                        }
                                    });
                                }
                                markup::TermLine::Field {
                                    name,
                                    secret,
                                    lead,
                                } => {
                                    ui.horizontal(|ui| {
                                        ui.spacing_mut().item_spacing.x = 0.0;
                                        for (role, text) in lead {
                                            self.term_run(ui, *role, text, body);
                                        }
                                        let mut value =
                                            self.fields.get(name).cloned().unwrap_or_default();
                                        let ink = if term.bare {
                                            Color32::WHITE
                                        } else {
                                            TERM_INPUT
                                        };
                                        let room = (ui.available_width() - 8.0).max(120.0);
                                        let mut edit = egui::TextEdit::singleline(&mut value)
                                            .font(egui::TextStyle::Monospace)
                                            .text_color(ink)
                                            .desired_width(room)
                                            .frame(egui::Frame::new())
                                            .margin(egui::Margin::symmetric(0, 0));
                                        if *secret {
                                            edit = edit.password(true);
                                        }
                                        ui.visuals_mut().text_cursor.stroke.color = ink;
                                        let resp = ui.add(edit);
                                        if resp.changed() {
                                            self.fields.insert(name.clone(), value.clone());
                                        }
                                        if self.focus_input && !self.address_focused {
                                            resp.request_focus();
                                            self.focus_input = false;
                                        }
                                        if resp.lost_focus()
                                            && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                        {
                                            self.submit();
                                        }
                                    });
                                }
                            }
                        }
                    });
            });
    }

    /// One coloured run inside a `[[term]]`. A run whose text is a URL behaves
    /// like a link, because a terminal that prints an address should be able to
    /// open it.
    pub(crate) fn term_run(&mut self, ui: &mut egui::Ui, role: markup::TermRole, text: &str, body: Color32) {
        use markup::TermRole;
        let color = match role {
            TermRole::User => TERM_USER,
            TermRole::Host => TERM_HOST,
            TermRole::Prompt => TERM_PROMPT,
            TermRole::Cmd => TERM_CMD,
            TermRole::Link => TERM_LINK,
            TermRole::Keys => TERM_ASCII,
            TermRole::Input => TERM_INPUT,
            TermRole::Out => body,
        };
        let rich = egui::RichText::new(text)
            .font(egui::FontId::monospace(15.0))
            .line_height(Some(TERM_LINE))
            .color(color);
        let clickable = matches!(role, TermRole::Link)
            && (text.starts_with("http://") || text.starts_with("https://"));
        if clickable {
            let resp = ui.add(
                egui::Label::new(rich.underline())
                    .wrap_mode(egui::TextWrapMode::Extend)
                    .sense(egui::Sense::click()),
            );
            if resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if resp.clicked() {
                open_in_browser(text);
            }
        } else {
            ui.add(egui::Label::new(rich).wrap_mode(egui::TextWrapMode::Extend));
        }
    }
}
