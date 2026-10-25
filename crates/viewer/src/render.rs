//! 排版：把块列表变成窗口里的画面。
//!
//! 每种块的具体画法按类别分在 `blocks/` 下：
//! `media`（图片/头像/动画/终端）、`panel`（盒子/表格/提示框/页签）。

use crate::*;

impl Tab {
    pub(crate) fn job(&self, parts: &[Inline], size: f32, color: Color32, family: FontFamily) -> LayoutJob {
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
                        font_id: FontId::new(size, bold_of(&family)),
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

    /// Renders a run of inline content. When it contains links they become real
    /// widgets and the text flows around them; otherwise it is one laid-out job.
    ///
    /// `wrap` is off inside table cells: there the surrounding `Grid` decides
    /// column widths from what each cell asks for, and a wrapping layout asks for
    /// its minimum, which splits labels like "分栏实验" across two lines.
    pub(crate) fn render_flow(
        &mut self,
        ui: &mut egui::Ui,
        parts: &[Inline],
        size: f32,
        color: Color32,
        family: FontFamily,
    ) {
        self.render_flow_with(ui, parts, size, color, family, true);
    }

    pub(crate) fn render_flow_with(
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

    pub(crate) fn render_blocks(&mut self, ui: &mut egui::Ui) {
        let blocks = self.blocks.clone();
        self.sheet.clear();
        collect_sheet(&blocks, &mut self.sheet);
        self.render_list(ui, blocks, Text::default());
    }

    pub(crate) fn render_list(&mut self, ui: &mut egui::Ui, blocks: Vec<Block>, text: Text) {
        let body = text.color;
        let muted = text.color.gamma_multiply(0.62);
        // Links take their colour from the enclosing box, so a dark navbar can
        // say so once instead of on every link.
        ui.visuals_mut().hyperlink_color = text.link;
        let mut saw_input = false;

        for block in blocks {
            match block {
                Block::Heading(level, parts) => {
                    let scale = match level {
                        1 => 1.75,
                        2 => 1.32,
                        _ => 1.1,
                    };
                    ui.add_space(14.0);
                    ui.label(self.job(
                        &parts,
                        text.size * scale,
                        text.color,
                        text.bold_family(),
                    ));
                    ui.add_space(6.0);
                }
                Block::Para(parts) => {
                    self.render_flow(ui, &parts, text.size, body, text.family());
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
                        ui.label(self.job(&parts, 16.0, muted, text.family()));
                    });
                    ui.add_space(8.0);
                }
                Block::List { ordered, items } => {
                    for (index, item) in items.into_iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            let marker = if ordered {
                                format!("{}.", index + 1)
                            } else {
                                "•".to_string()
                            };
                            ui.label(RichText::new(marker).size(16.0).color(muted));
                            ui.label(self.job(&item, 17.0, body, text.family()));
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
                Block::Avatar { name, size } => self.render_avatar(ui, &name, size),
                Block::Prefill { name, value } => {
                    // Only if nobody has typed into it yet: putting the starting
                    // text back every frame would erase what the reader types.
                    if !self.fields.contains_key(&name) {
                        self.fields.insert(name, unescape_lines(&value));
                    }
                }
                Block::Scene {
                    width,
                    height,
                    seconds,
                    bars,
                } => {
                    self.render_scene(ui, width, height, seconds, &bars);
                }
                Block::Box { style, children } => {
                    self.render_box(ui, &style, children, text);
                }
                Block::Sheet(_) => {}
                Block::Term(term) => {
                    if term.target.is_some() {
                        saw_input = true;
                    }
                    self.render_term(ui, &term);
                    ui.add_space(10.0);
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
                Block::Caption(parts) => {
                    ui.label(self.job(&parts, 13.0, muted, text.family()));
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
                    self.render_note(ui, text, kind, &title, inner);
                }
                Block::Details { summary, body } => {
                    let label = markup::plain(&summary);
                    let salt = ui.next_auto_id();
                    egui::CollapsingHeader::new(RichText::new(label).size(16.0))
                        .id_salt(salt)
                        .show(ui, |ui| self.render_list(ui, body, text));
                    ui.add_space(6.0);
                }
                Block::Tabs { titles, bodies } => {
                    self.render_tabs(ui, &titles, bodies, text);
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
                                    self.render_list(ui, part, text);
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

/// The first heading anywhere in the tree, outermost first.
///
/// Nested blocks count: a page that wraps its body in a box still has a title.
pub(crate) fn first_heading(blocks: &[Block]) -> Option<String> {
    for block in blocks {
        match block {
            Block::Heading(_, parts) => return Some(markup::plain(parts)),
            Block::Box { children, .. } => {
                if let Some(found) = first_heading(children) {
                    return Some(found);
                }
            }
            Block::Columns(parts) => {
                for part in parts {
                    if let Some(found) = first_heading(part) {
                        return Some(found);
                    }
                }
            }
            Block::Note { body, .. } | Block::Details { body, .. } => {
                if let Some(found) = first_heading(body) {
                    return Some(found);
                }
            }
            Block::Tabs { bodies, .. } => {
                for body in bodies {
                    if let Some(found) = first_heading(body) {
                        return Some(found);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

/// Walks the whole page for `[[sheet]]` rules before anything is drawn.
pub(crate) fn collect_sheet(blocks: &[Block], out: &mut HashMap<String, Style>) {
    for block in blocks {
        match block {
            Block::Sheet(rules) => {
                for (name, style) in rules {
                    let merged = style.clone().over(out.get(name).unwrap_or(&Style::default()));
                    out.insert(name.clone(), merged);
                }
            }
            Block::Box { children, .. } => collect_sheet(children, out),
            Block::Columns(parts) => {
                for part in parts {
                    collect_sheet(part, out);
                }
            }
            Block::Note { body, .. } | Block::Details { body, .. } => collect_sheet(body, out),
            Block::Tabs { bodies, .. } => {
                for body in bodies {
                    collect_sheet(body, out);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_heading_is_found_inside_boxes() {
        let page = "[[sheet]]\n.c { bg: #fff; }\n[[end]]\n[[box class=page]]\n[[box class=nav]]\n首页\n[[end]]\n# 文章标题\n正文\n[[end]]\n";
        assert_eq!(first_heading(&markup::parse(page)).as_deref(), Some("文章标题"));
    }
}
