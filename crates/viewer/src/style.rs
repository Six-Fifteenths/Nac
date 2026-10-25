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
    pub size: Option<f32>,
    pub weight: Option<Weight>,
    pub mono: Option<bool>,
    pub italic: Option<bool>,
    pub link: Option<Color32>,
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
            size: self.size.or(base.size),
            weight: self.weight.or(base.weight),
            mono: self.mono.or(base.mono),
            italic: self.italic.or(base.italic),
            link: self.link.or(base.link),
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
            "align" | "text-align" => {
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
            "mono" | "font-family" => {
                self.mono = match value {
                    "mono" | "monospace" => Some(true),
                    "sans" | "serif" | "proportional" => Some(false),
                    _ => None,
                }
            }
            "italic" | "font-style" => self.italic = Some(value == "italic"),
            "link" => self.link = color(value),
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
    pub mono: bool,
    pub italic: bool,
    pub link: Color32,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            color: Color32::from_gray(28),
            size: 17.0,
            bold: false,
            mono: false,
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
            mono: style.mono.unwrap_or(self.mono),
            italic: style.italic.unwrap_or(self.italic),
            link: style.link.unwrap_or(self.link),
        }
    }

    pub fn family(&self) -> egui::FontFamily {
        if self.mono {
            egui::FontFamily::Monospace
        } else if self.bold {
            egui::FontFamily::Name(std::sync::Arc::from("bold"))
        } else {
            egui::FontFamily::Proportional
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
}
