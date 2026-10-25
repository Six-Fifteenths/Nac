//! A very small style system: the part of CSS this client is willing to read.
//!
//! No cascade, no selectors beyond a class name, and — deliberately — no way to
//! say where something goes or how wide it is. A page describes *what a thing
//! is* (a card, a navbar, an aside); the client decides the layout. That is why
//! there is no `width`, no `position`, no percentage and no media query here:
//! the same page has to work in a 700px window and a 2000px one.
//!
//! Values are written either as attributes (`pad=16 bg=#fff`) or as declarations
//! in a stylesheet (`pad: 16; bg: #fff;`). Both go through [`Style::set`], so a
//! class and an attribute can never drift apart.

use eframe::egui::{self, Color32};

/// A drop shadow: blur radius, vertical offset, colour.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub blur: f32,
    pub offset: f32,
    pub color: Color32,
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
    pub radius: Option<f32>,
    /// Width, colour.
    pub border: Option<(f32, Color32)>,
    /// `dir=row`. `None` means column, which is the document default. This is
    /// the only layout attribute there is: the client, not the page, decides
    /// how wide the columns turn out.
    pub row: Option<bool>,
    /// Where the text sits inside its box.
    pub text_align: Option<Align>,
    pub size: Option<f32>,
    pub weight: Option<Weight>,
    pub font: Option<Font>,
    pub italic: Option<bool>,
    pub link: Option<Color32>,
    pub shadow: Option<Shadow>,
    /// 0..1. `None` means fully opaque.
    pub opacity: Option<f32>,
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
            radius: self.radius.or(base.radius),
            border: self.border.or(base.border),
            row: self.row.or(base.row),
            text_align: self.text_align.or(base.text_align),
            size: self.size.or(base.size),
            weight: self.weight.or(base.weight),
            font: self.font.or(base.font),
            italic: self.italic.or(base.italic),
            link: self.link.or(base.link),
            shadow: self.shadow.or(base.shadow),
            opacity: self.opacity.or(base.opacity),
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
            "radius" | "border-radius" => self.radius = number(value),
            "border" => self.border = border(value),
            "dir" | "display" => {
                self.row = match value {
                    "row" | "flex" | "horizontal" => Some(true),
                    "col" | "column" | "vertical" | "block" => Some(false),
                    _ => None,
                }
            }
            "ta" | "text-align" | "halign" => {
                self.text_align = match value {
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
            "shadow" => self.shadow = shadow(value),
            "opacity" => self.opacity = opacity(value),
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

/// `16` or `16px`. Nothing else: no `%`, no `calc()`, no `em`.
///
/// This is the third rule of the style layer, held in code rather than in a
/// comment: a page may say how large something is, it may not ask the client to
/// work out a size. `50%` used to be read as 50 pixels, which is worse than
/// refusing it.
fn number(value: &str) -> Option<f32> {
    let value = value.trim();
    let digits = value.strip_suffix("px").unwrap_or(value);
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit() || c == '.' || c == '-') {
        return None;
    }
    digits.parse::<f32>().ok()
}

/// `8` = blur 8, offset 0; `8,3` adds a downward offset; `8,3:#00000033` a colour.
fn shadow(value: &str) -> Option<Shadow> {
    let (sizes, color) = match value.split_once(':') {
        Some((sizes, colour)) => (sizes, color(colour).unwrap_or(Color32::from_black_alpha(40))),
        None => (value, Color32::from_black_alpha(40)),
    };
    let mut parts = sizes.split(',');
    let blur = number(parts.next()?)?;
    // No offset means a shadow straight behind the box, not one 8px lower.
    let offset = parts.next().and_then(number).unwrap_or(0.0);
    if blur <= 0.0 {
        return None;
    }
    Some(Shadow {
        blur,
        offset,
        color,
    })
}

/// `0.6` and `60` both mean 60%.
fn opacity(value: &str) -> Option<f32> {
    let amount = number(value)?;
    let fraction = if amount > 1.0 { amount / 100.0 } else { amount };
    Some(fraction.clamp(0.0, 1.0))
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
        // `#rrggbbaa`, which is how a shadow says how soft it is.
        8 => {
            let r = expand(&hex[0..2])?;
            let g = expand(&hex[2..4])?;
            let b = expand(&hex[4..6])?;
            let a = expand(&hex[6..8])?;
            Some(Color32::from_rgba_unmultiplied(r, g, b, a))
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
    fn shadow_and_opacity_take_sizes_not_expressions() {
        let plain = Style::from_attrs("shadow=8");
        let shadow = plain.shadow.expect("shadow parses");
        assert_eq!(shadow.blur, 8.0);
        assert_eq!(shadow.offset, 0.0);
        assert!(shadow.color.a() > 0, "a shadow is visible by default");

        let deep = Style::from_attrs("shadow=12,4:#000000").shadow.unwrap();
        assert_eq!((deep.blur, deep.offset), (12.0, 4.0));

        assert_eq!(Style::from_attrs("shadow=0").shadow, None);
        assert_eq!(Style::from_attrs("opacity=0.6").opacity, Some(0.6));
        assert_eq!(Style::from_attrs("opacity=60").opacity, Some(0.6));
        assert_eq!(Style::from_attrs("opacity=200").opacity, Some(1.0));
        assert_eq!(Style::from_attrs("opacity=20%").opacity, None);
    }

    /// The rules the style layer is not allowed to break. They are tests rather
    /// than a paragraph in a document, so that adding `position`, a width or a
    /// selector later fails here first.
    #[test]
    fn the_style_layer_refuses_to_become_css() {
        // 1. Numbers are numbers: no percentages, no expressions.
        assert_eq!(number("16"), Some(16.0));
        assert_eq!(number("16px"), Some(16.0));
        assert_eq!(number("50%"), None);
        assert_eq!(number("calc(50% - 4px)"), None);
        assert_eq!(number("2em"), None);
        // 2. No geometry. A page describes structure, never a size or a place.
        let placed = Style::from_attrs(
            "position=absolute x=10 y=20 top=4 left=4 float=left w=200 maxw=980 grow=2 \
             minh=40 align=center",
        );
        assert_eq!(placed.pad, None);
        assert_eq!(placed.margin, None);
        assert_eq!(placed.row, None);
        assert_eq!(placed.text_align, None);

        // 3. No media queries, no animation beyond the hover fade.
        let media = Style::from_declarations("@media (max-width: 600px): pad: 4;");
        assert_eq!(media.pad, None);
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
