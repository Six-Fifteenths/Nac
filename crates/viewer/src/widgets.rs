//! 小零件：标签页的方块、地址栏的同步规则、终端配色、字符串裁剪。
//!
//! 都是与"页面内容"无关的东西，所以单独放一处。

use crate::*;


pub(crate) enum TabHit {
    None,
    Switch,
    Close,
}

/// One tab: a rounded chip with the page title, and a `×` when it can be closed.
///
/// The title is capped so one long page does not push every other tab sideways;
/// middle-click closes it, the way a browser does.
pub(crate) fn tab_chip(ui: &mut egui::Ui, tab: &Tab, active: bool, can_close: bool) -> TabHit {
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
pub(crate) fn should_sync_address(
    focused: bool,
    holding: bool,
    edited: bool,
    shown: &str,
    want: &str,
) -> bool {
    shown != want && !holding && (!focused || !edited)
}

/// One line of markup cannot carry a newline, so `\n` stands for one. A literal
/// backslash is sent as `\\`, which keeps the round trip exact.
pub(crate) fn unescape_lines(text: &str) -> String {
    text.replace("\\\\", "\u{1}")
        .replace("\\n", "\n")
        .replace('\u{1}', "\\")
}

/// Palette of a `[[term]]` panel. These are the WebShell defaults, so a page can
/// reproduce that look without the client knowing what a "terminal site" is.
pub(crate) const TERM_BG: Color32 = Color32::from_rgb(0x0C, 0x06, 0x23);
pub(crate) const TERM_FG: Color32 = Color32::from_rgb(0xF8, 0xDD, 0xE5);
pub(crate) const TERM_BORDER: Color32 = Color32::from_rgb(0xFF, 0xAD, 0xE2);
pub(crate) const TERM_ASCII: Color32 = Color32::from_rgb(0xFF, 0x99, 0x51);
pub(crate) const TERM_USER: Color32 = Color32::from_rgb(0xFE, 0x6B, 0xC9);
pub(crate) const TERM_HOST: Color32 = Color32::from_rgb(0x70, 0xFD, 0xFF);
pub(crate) const TERM_PROMPT: Color32 = Color32::from_rgb(0xA5, 0xA7, 0xA7);
pub(crate) const TERM_CMD: Color32 = Color32::from_rgb(0xFD, 0x9B, 0xDB);
pub(crate) const TERM_LINK: Color32 = Color32::from_rgb(0xB6, 0xAA, 0xEE);
pub(crate) const TERM_INPUT: Color32 = Color32::from_rgb(0xFF, 0x76, 0x85);
/// One terminal line. The original CSS says 22px at 16px text.
pub(crate) const TERM_LINE: f32 = 22.0;

/// Caps a title so a narrow window still fits a few tabs. Counts characters, not
/// bytes, so a Chinese title is cut at a sensible place.
pub(crate) fn shorten(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let mut out: String = text.chars().take(limit.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// A hairline under the tab strip, so the strip reads as one and the toolbar
/// below does not float in the page.
pub(crate) fn strip_line(ui: &egui::Ui) {
    let rect = ui.available_rect_before_wrap();
    ui.painter().hline(
        rect.x_range(),
        rect.top(),
        egui::Stroke::new(1.0, Color32::from_rgb(226, 226, 232)),
    );
}
