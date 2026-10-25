//! A very small style system: the subset of CSS this client understands.
//!
//! There is no cascade, no inheritance chains and no selectors beyond a class
//! name. What there is: a box that can be told its background, border, padding,
//! spacing, width and direction, and text that inherits colour, size and family
//! from the box it sits in. That is enough for a real layout — a green navbar, a
//! card with a border, an 8:4 two-column page — without inventing a browser.
//!
//! Values are written either as attributes (`pad=16 bg=#fff`) or as declarations
//! in a stylesheet (`pad: 16; bg: #fff;`). Both go through [`Style::set`], so a
//! class and an attribute can never drift apart.

use eframe::egui::{self, Color32};

/// What a box paints at one instant, after any hover blend.
pub struct Painted {
    pub bg: Option<Color32>,
    pub border: Option<(f32, Color32)>,
}

pub fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        f(a.r(), b.r()),
        f(a.g(), b.g()),
        f(a.b(), b.b()),
        f(a.a(), b.a()),
    )
}

/// `None` on either side means "leave it alone", which is what a background
/// that does not change on hover should do.
fn mix(base: Option<Color32>, hover: Option<Color32>, t: f32) -> Option<Color32> {
    match (base, hover) {
        (_, None) => base,
        (None, Some(h)) => Some(h.gamma_multiply(t)),
        (Some(a), Some(h)) => Some(lerp_color(a, h, t)),
    }
}

fn family(name: &str) -> egui::FontFamily {
    egui::FontFamily::Name(std::sync::Arc::from(name))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
    Normal,
    Bold,
}

/// Which of the bundled faces to draw with. All of them live inside the binary,
/// so a page looks the same on every machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Font {
    Sans,
    Serif,
    Mono,
}

impl Font {
    fn parse(value: &str) -> Option<Font> {
        match value {
            "sans" | "sans-serif" | "proportional" | "ui" => Some(Font::Sans),
            "serif" => Some(Font::Serif),
            "mono" | "monospace" | "code" => Some(Font::Mono),
            _ => None,
        }
    }
}

/// Every property is optional: an unset one inherits from the enclosing box.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    /// Class name looked up in the page stylesheet.
    pub class: Option<String>,
    pub bg: Option<Color32>,
    pub color: Option<Color32>,
    /// Horizontal, vertical.
    pub pad: Option<(f32, f32)>,
    /// Horizontal, vertical.
    pub margin: Option<(f32, f32)>,
    pub gap: Option<f32>,
    pub width: Option<f32>,
    pub max_width: Option<f32>,
    pub min_height: Option<f32>,
    pub radius: Option<f32>,
    /// Width, colour.
    pub border: Option<(f32, Color32)>,
    /// `dir=row`. `None` means column, which is the document default.
    pub row: Option<bool>,
    /// Share of the leftover width when the parent is a row.
    pub grow: Option<f32>,
    pub align: Option<Align>,
    /// Where the text sits inside its box, as opposed to where the box sits
    /// inside its parent (`align`).
    pub text_align: Option<Align>,
    pub size: Option<f32>,
    pub weight: Option<Weight>,
    pub font: Option<Font>,
    pub italic: Option<bool>,
    pub link: Option<Color32>,
    /// What the box looks like while the pointer is over it. A hover only ever
    /// changes paint, never layout, so nothing jumps when the mouse arrives.
    pub hover_bg: Option<Color32>,
    pub hover_color: Option<Color32>,
    pub hover_border: Option<(f32, Color32)>,
    pub hover_link: Option<Color32>,
    /// Fade time in milliseconds; 0 means the change is instant.
    pub transition: Option<f32>,
    /// `nowrap`: do not wrap lines (a navbar, a tag row).
    pub nowrap: Option<bool>,
}

impl Style {
    /// Fills unset properties from `base`. Used for class → attribute overrides.
    pub fn over(&self, base: &Style) -> Style {
        Style {
            class: self.class.clone().or_else(|| base.class.clone()),
            bg: self.bg.or(base.bg),
            color: self.color.or(base.color),
            pad: self.pad.or(base.pad),
            margin: self.margin.or(base.margin),
            gap: self.gap.or(base.gap),
            width: self.width.or(base.width),
            max_width: self.max_width.or(base.max_width),
            min_height: self.min_height.or(base.min_height),
            radius: self.radius.or(base.radius),
            border: self.border.or(base.border),
            row: self.row.or(base.row),
            grow: self.grow.or(base.grow),
            align: self.align.or(base.align),
            text_align: self.text_align.or(base.text_align),
            size: self.size.or(base.size),
            weight: self.weight.or(base.weight),
            font: self.font.or(base.font),
            italic: self.italic.or(base.italic),
            link: self.link.or(base.link),
            hover_bg: self.hover_bg.or(base.hover_bg),
            hover_color: self.hover_color.or(base.hover_color),
            hover_border: self.hover_border.or(base.hover_border),
            hover_link: self.hover_link.or(base.hover_link),
            transition: self.transition.or(base.transition),
            nowrap: self.nowrap.or(base.nowrap),
        }
    }

    /// Sets one property. Unknown names are ignored rather than fatal: a typo
    /// should cost a colour, not the whole page.
    pub fn set(&mut self, key: &str, value: &str) {
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        match key.as_str() {
            "bg" | "background" => self.bg = color(value),
            "color" | "fg" => self.color = color(value),
            "pad" | "padding" => self.pad = pair(value),
            "margin" => self.margin = pair(value),
            "gap" => self.gap = number(value),
            "w" | "width" => self.width = number(value),
            "maxw" | "max-width" => self.max_width = number(value),
            "minh" | "min-height" => self.min_height = number(value),
            "radius" | "border-radius" => self.radius = number(value),
            "border" => self.border = border(value),
            "dir" | "display" => {
                self.row = match value {
                    "row" | "flex" | "horizontal" => Some(true),
                    "col" | "column" | "vertical" | "block" => Some(false),
                    _ => None,
                }
            }
            "grow" | "flex" => self.grow = number(value),
            "ta" | "text-align" | "halign" => {
                self.text_align = match value {
                    "center" => Some(Align::Center),
                    "right" | "end" => Some(Align::Right),
                    "left" | "start" => Some(Align::Left),
                    _ => None,
                }
            }
            "align" => {
                self.align = match value {
                    "center" => Some(Align::Center),
                    "right" | "end" => Some(Align::Right),
                    "left" | "start" => Some(Align::Left),
                    _ => None,
                }
            }
            "size" | "font-size" => self.size = number(value),
            "weight" | "font-weight" => {
                self.weight = match value {
                    "bold" | "600" | "700" | "800" => Some(Weight::Bold),
                    "normal" | "400" | "300" => Some(Weight::Normal),
                    _ => None,
                }
            }
            "font" | "mono" | "font-family" => self.font = Font::parse(value),
            "italic" | "font-style" => self.italic = Some(value == "italic"),
            "link" => self.link = color(value),
            "hover-bg" | "hover-background" => self.hover_bg = color(value),
            "hover-color" | "hover-fg" => self.hover_color = color(value),
            "hover-border" => self.hover_border = border(value),
            "hover-link" => self.hover_link = color(value),
            "transition" => {
                self.transition = number(&value.replace("ms", "")).map(|ms| ms / 1000.0)
            }
            "nowrap" | "white-space" => self.nowrap = Some(value == "nowrap"),
            _ => {}
        }
    }

    /// Parses `k=v k=v` into a style.
    pub fn from_attrs(attrs: &str) -> Style {
        let mut style = Style::default();
        for token in attrs.split_whitespace() {
            let Some((key, value)) = token.split_once('=') else {
                continue;
            };
            if key.eq_ignore_ascii_case("class") {
                style.class = Some(value.trim_matches('"').to_string());
                continue;
            }
            style.set(key, value.trim_matches('"'));
        }
        style
    }

    /// Parses `k: v; k: v` from inside a `[[sheet]]` rule.
    pub fn from_declarations(text: &str) -> Style {
        let mut style = Style::default();
        for part in text.split(';') {
            let Some((key, value)) = part.split_once(':') else {
                continue;
            };
            style.set(key, value);
        }
        style
    }

    /// The style a pointer-over box actually paints with, `t` of the way there.
    pub fn blend(&self, t: f32) -> Painted {
        let t = t.clamp(0.0, 1.0);
        Painted {
            bg: mix(self.bg, self.hover_bg, t),
            border: match (self.border, self.hover_border) {
                (None, Some(h)) => Some((h.0, h.1.gamma_multiply(t))),
                (Some(a), Some(h)) => Some((h.0, lerp_color(a.1, h.1, t))),
                (b, None) => b,
            },
        }
    }

}

/// Text properties that a child block inherits from its box.
#[derive(Clone, Copy, Debug)]
pub struct Text {
    pub color: Color32,
    pub size: f32,
    pub bold: bool,
    pub font: Font,
    pub align: Option<Align>,
    pub italic: bool,
    pub link: Color32,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            color: Color32::from_gray(28),
            size: 17.0,
            bold: false,
            font: Font::Sans,
            align: None,
            italic: false,
            link: Color32::from_rgb(40, 100, 190),
        }
    }
}

impl Text {
    /// Applies a box's text properties on top of what it inherited.
    pub fn inherit(self, style: &Style) -> Text {
        Text {
            color: style.color.unwrap_or(self.color),
            size: style.size.unwrap_or(self.size),
            bold: matches!(style.weight, Some(Weight::Bold)) || (style.weight.is_none() && self.bold),
            font: style.font.unwrap_or(self.font),
            align: style.text_align.or(self.align),
            italic: style.italic.unwrap_or(self.italic),
            link: style.link.unwrap_or(self.link),
        }
    }

    pub fn family(&self) -> egui::FontFamily {
        match (self.font, self.bold) {
            (Font::Mono, _) => egui::FontFamily::Monospace,
            (Font::Serif, true) => family("serif-bold"),
            (Font::Serif, false) => family("serif"),
            (Font::Sans, true) => family("sans-bold"),
            (Font::Sans, false) => family("sans"),
        }
    }

    /// The bold cut of whichever family this text uses.
    pub fn bold_family(&self) -> egui::FontFamily {
        match self.font {
            Font::Mono => egui::FontFamily::Monospace,
            Font::Serif => family("serif-bold"),
            Font::Sans => family("sans-bold"),
        }
    }
}

fn number(value: &str) -> Option<f32> {
    value
        .trim()
        .trim_end_matches("px")
        .trim_end_matches('%')
        .parse::<f32>()
        .ok()
}

/// `16` → (16,16); `16,24` → (16,24).
fn pair(value: &str) -> Option<(f32, f32)> {
    let mut parts = value.split(',');
    let first = number(parts.next()?)?;
    let second = parts.next().and_then(number).unwrap_or(first);
    Some((first, second))
}

/// `#rrggbb`, `#rgb`, or a few names.
pub fn color(value: &str) -> Option<Color32> {
    let value = value.trim();
    let named = match value.to_ascii_lowercase().as_str() {
        "white" => Some((255, 255, 255)),
        "black" => Some((0, 0, 0)),
        "none" | "transparent" => return None,
        _ => None,
    };
    if let Some((r, g, b)) = named {
        return Some(Color32::from_rgb(r, g, b));
    }
    let hex = value.strip_prefix('#')?;
    let expand = |c: &str| u8::from_str_radix(c, 16).ok();
    match hex.len() {
        3 => {
            let r = expand(&hex[0..1])?;
            let g = expand(&hex[1..2])?;
            let b = expand(&hex[2..3])?;
            Some(Color32::from_rgb(r * 17, g * 17, b * 17))
        }
        6 => {
            let r = expand(&hex[0..2])?;
            let g = expand(&hex[2..4])?;
            let b = expand(&hex[4..6])?;
            Some(Color32::from_rgb(r, g, b))
        }
        _ => None,
    }
}

/// `1:#dddddd` or `1px:#ddd`.
fn border(value: &str) -> Option<(f32, Color32)> {
    let (width, colour) = value.rsplit_once(':')?;
    Some((number(width)?, color(colour)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_and_declarations_agree() {
        let a = Style::from_attrs("pad=12 bg=#ffffff border=1:#dddddd");
        let b = Style::from_declarations("pad: 12; bg: #fff; border: 1:#ddd;");
        assert_eq!(a.pad, b.pad);
        assert_eq!(a.bg, b.bg);
        assert_eq!(a.border, b.border);
        assert_eq!(a.pad, Some((12.0, 12.0)));
    }

    #[test]
    fn a_class_is_overridden_by_an_attribute() {
        let base = Style::from_declarations("bg: #fff; pad: 16; radius: 4;");
        let over = Style::from_attrs("pad=4");
        let merged = over.over(&base);
        assert_eq!(merged.bg, base.bg, "class keeps what the attribute omits");
        assert_eq!(merged.pad, Some((4.0, 4.0)), "attribute wins");
        assert_eq!(merged.radius, Some(4.0));
    }

    #[test]
    fn sizes_accept_px_and_hex_accepts_short_form() {
        assert_eq!(number("16px"), Some(16.0));
        assert_eq!(color("#fff"), Some(Color32::from_rgb(255, 255, 255)));
        assert_eq!(color("#5cb85c"), Some(Color32::from_rgb(0x5c, 0xb8, 0x5c)));
        assert_eq!(border("2:#5cb85c"), Some((2.0, Color32::from_rgb(0x5c, 0xb8, 0x5c))));
    }

    #[test]
    fn text_inherits_and_can_be_overridden() {
        let base = Text::default();
        let white = base.inherit(&Style::from_attrs("color=#fff weight=bold size=15"));
        assert_eq!(white.color, Color32::from_rgb(255, 255, 255));
        assert!(white.bold);
        assert_eq!(white.size, 15.0);
        // A child box that sets nothing keeps the white bold of its parent.
        let inner = white.inherit(&Style::default());
        assert_eq!(inner.color, white.color);
        assert!(inner.bold);
    }

    #[test]
    fn a_hover_never_changes_the_box_only_its_paint() {
        let quiet = Style::from_attrs("bg=#ffffff border=1:#dddddd");
        let loud = Style::from_attrs(
            "bg=#ffffff border=1:#dddddd hover-bg=#ffe9e9 hover-border=3:#cc0000 transition=200",
        );
        // Nothing set: the box looks the same at every point of the hover.
        let still = quiet.blend(1.0);
        assert_eq!(still.bg, quiet.bg);
        assert_eq!(still.border, quiet.border);
        // Halfway: the colours are between the two ends, not either end.
        let half = loud.blend(0.5);
        assert_eq!(half.bg, Some(Color32::from_rgb(0xff, 0xf4, 0xf4)));
        assert_eq!(half.border.map(|b| b.0), Some(3.0));
        // Fully hovered: exactly the hover colour.
        assert_eq!(loud.blend(1.0).bg, Some(Color32::from_rgb(0xff, 0xe9, 0xe9)));
        assert_eq!(loud.blend(1.0).border, Some((3.0, Color32::from_rgb(0xcc, 0, 0))));
        // A box with no background fades one in from nothing rather than jumping.
        let bare = Style::from_attrs("hover-bg=#ffe9e9");
        assert_eq!(bare.blend(0.0).bg.map(|c| c.a()), Some(0));
        let mid = bare.blend(0.5).bg.unwrap().a();
        assert!(mid > 0 && mid < 255, "halfway there, got alpha {mid}");
    }

    #[test]
    fn a_font_is_picked_by_name_and_decides_the_family() {
        let serif = Text::default().inherit(&Style::from_attrs("font=serif"));
        assert_eq!(serif.font, Font::Serif);
        let mono = Text::default().inherit(&Style::from_attrs("font=monospace"));
        assert_eq!(mono.font, Font::Mono);
        assert_eq!(mono.family(), egui::FontFamily::Monospace);
        assert_eq!(
            serif.bold_family(),
            egui::FontFamily::Name(std::sync::Arc::from("serif-bold"))
        );
    }
}
