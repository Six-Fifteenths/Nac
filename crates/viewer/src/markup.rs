//! Line-oriented markup parser.
//!
//! The vocabulary is small on purpose, but the blocks nest: a column can hold a
//! table, a note can hold a list. Parsing is recursive — every container
//! (`[[cols]]`, `[[note]]`, `[[details]]`, `[[tabs]]`) opens a new sequence and
//! returns at its own `[[end]]`, so nesting depth is bounded by the input rather
//! than by bespoke depth counters.

use crate::style::Style;

#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
    /// `[[path:/x|文字]]` or `[[12|文字]]` written inside a run of text.
    Link { target: LinkTarget, text: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum LinkTarget {
    Page(u16),
    Path(String),
    /// `[[url:https://…|文字]]` opens in the system browser. Only http(s) is
    /// accepted, so a page cannot hand the shell a `file:` or custom scheme.
    Url(String),
}

/// One rectangle in a `[[scene]]`, with its keyframes in scene coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct SceneBar {
    pub color: [u8; 3],
    pub width: f32,
    pub height: f32,
    /// `(t, x, y)`, sorted by `t`. Positions are the rectangle's top-left corner.
    pub keys: Vec<(f32, f32, f32)>,
}

/// Severity of a `[[note]]` box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteKind {
    Info,
    Ok,
    Warn,
    Danger,
}

/// Colour role of one run of text inside a `[[term]]`.
///
/// The codes are short because pages generate them: `pu` user, `ph` host,
/// `pd` prompt default, `out` plain output, `cmd` a command name, `link` an
/// address, `keys` a key hint, `in` what the reader typed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TermRole {
    User,
    Host,
    Prompt,
    Out,
    Cmd,
    Link,
    Keys,
    Input,
}

impl TermRole {
    fn parse(code: &str) -> Self {
        match code {
            "pu" => Self::User,
            "ph" => Self::Host,
            "pd" => Self::Prompt,
            "cmd" => Self::Cmd,
            "link" => Self::Link,
            "keys" => Self::Keys,
            "in" => Self::Input,
            _ => Self::Out,
        }
    }
}

/// One line of a `[[term]]` panel.
#[derive(Clone, Debug, PartialEq)]
pub enum TermLine {
    /// Banner art: monospace, fixed leading, no spanning.
    Ascii(String),
    /// Coloured runs, concatenated without spacing.
    Text(Vec<(TermRole, String)>),
    /// Coloured runs, then a text field. `secret` draws dots instead of glyphs.
    Field {
        name: String,
        secret: bool,
        lead: Vec<(TermRole, String)>,
    },
}

/// A dark terminal panel: `[[term panel|标题|页号]]` … `[[end]]`.
///
/// `target` makes the panel's own text field submit to that page, so the input
/// line can sit *inside* the frame instead of below it — which is the whole
/// point of a terminal.
#[derive(Clone, Debug, PartialEq)]
pub struct Term {
    pub title: String,
    pub target: Option<u16>,
    /// Without the header bars, the panel is the "everything is ruined" look:
    /// plain black, no title, no bars.
    pub bare: bool,
    pub lines: Vec<TermLine>,
}

impl NoteKind {
    fn parse(text: &str) -> Option<Self> {
        Some(match text.trim() {
            "info" => Self::Info,
            "ok" | "success" => Self::Ok,
            "warn" | "warning" => Self::Warn,
            "danger" | "error" => Self::Danger,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Heading(u8, Vec<Inline>),
    Para(Vec<Inline>),
    Quote(Vec<Inline>),
    /// `- item` / `* item` / `+ item`, or `1. item` with `ordered`.
    ///
    /// The numbers in an ordered list come from the client, not from the source:
    /// you may write `1.` on every line and still get 1, 2, 3.
    List {
        ordered: bool,
        items: Vec<Vec<Inline>>,
    },
    /// `| line` runs, or `[[code lang]] … [[end]]`.
    Pre {
        lang: Option<String>,
        text: String,
    },
    /// `---`
    Hr,
    Image {
        id: u16,
        pct: f32,
        ratio: Option<f32>,
    },
    /// `!cap 文字` — small muted line under a figure.
    Caption(Vec<Inline>),
    Link {
        target: LinkTarget,
        text: String,
    },
    Form {
        target: u16,
    },
    Input {
        name: String,
        label: String,
        secret: bool,
    },
    Textarea {
        name: String,
        label: String,
    },
    Checkbox {
        name: String,
        label: String,
    },
    /// `[[prefill 名|值]]`: give a field its starting text, once. A later frame
    /// must not put it back, or every keystroke would be undone.
    Prefill {
        name: String,
        value: String,
    },
    /// `[[avatar 名字 32]]`: a square avatar drawn from the name alone. Nothing
    /// is fetched, so a page cannot make the client talk to the network.
    Avatar {
        name: String,
        size: f32,
    },
    /// `[[scene WxH seconds]] … [[end]]`: a timeline of rectangles the client
    /// interpolates. The page decides what moves and when; nothing here is
    /// specific to any one demo.
    Scene {
        width: f32,
        height: f32,
        seconds: f32,
        bars: Vec<SceneBar>,
    },
    /// `[[hidden name|value]]`: carries state the reader never sees.
    Hidden {
        name: String,
        value: String,
    },
    Button {
        label: String,
    },
    Gap,
    /// `[[cols N]] … [[col]] … [[end]]`
    Columns(Vec<Vec<Block>>),
    /// `[[table]] | 表头 | … | [[end]]` — the first row is the header.
    Table {
        header: Vec<Vec<Inline>>,
        rows: Vec<Vec<Vec<Inline>>>,
    },
    /// `[[note kind|标题]] … [[end]]`
    Note {
        kind: NoteKind,
        title: Vec<Inline>,
        body: Vec<Block>,
    },
    /// `[[details 标题]] … [[end]]`
    Details {
        summary: Vec<Inline>,
        body: Vec<Block>,
    },
    /// `[[tabs 甲;乙]] … [[tab]] … [[end]]`
    Tabs {
        titles: Vec<String>,
        bodies: Vec<Vec<Block>>,
    },
    /// `[[term panel|标题|页号]] … [[end]]`
    Term(Term),
    /// `[[sheet]] .name { bg: #fff; } [[end]]` — class name → style. Collected
    /// before rendering, never drawn itself.
    Sheet(Vec<(String, Style)>),
    /// `[[box class=card pad=16 bg=#fff]] … [[end]]` — the layout primitive.
    Box {
        style: Style,
        children: Vec<Block>,
    },
}

pub fn parse(text: &str) -> Vec<Block> {
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;
    parse_seq(&lines, &mut i, &[]).0
}

/// Parses blocks until one of `stops` is reached, or until the input ends.
/// Returns the blocks and the stop marker that ended them (`None` at EOF).
fn parse_seq(lines: &[&str], i: &mut usize, stops: &[&str]) -> (Vec<Block>, Option<String>) {
    let mut out = Vec::new();
    while *i < lines.len() {
        let head = lines[*i].trim();
        if stops.iter().any(|stop| *stop == head) {
            let stop = head.to_string();
            *i += 1;
            return (out, Some(stop));
        }
        if head.is_empty() {
            push_gap(&mut out);
            *i += 1;
            continue;
        }
        match parse_one(lines, i) {
            Some(block) => out.push(block),
            None => *i += 1,
        }
    }
    (out, None)
}

fn parse_one(lines: &[&str], i: &mut usize) -> Option<Block> {
    let t = lines[*i].trim_start();
    let head = t.trim_end();

    if let Some((level, rest)) = heading(t) {
        *i += 1;
        return Some(Block::Heading(level, inlines(rest)));
    }

    if head == "---" || head == "***" || head == "___" {
        *i += 1;
        return Some(Block::Hr);
    }

    // ```lang … ``` —— Markdown 的围栏代码块，比 [[code]] 更该是默认写法。
    if let Some(rest) = t.strip_prefix("```") {
        let lang = rest.trim().to_string();
        *i += 1;
        let mut body: Vec<String> = Vec::new();
        while *i < lines.len() {
            let line = lines[*i];
            if line.trim_start().starts_with("```") {
                *i += 1;
                break;
            }
            body.push(line.trim_end().to_string());
            *i += 1;
        }
        return Some(Block::Pre {
            lang: (!lang.is_empty()).then_some(lang),
            text: body.join("\n"),
        });
    }

    // Stray markers outside their container are dropped rather than shown.
    if head == "[[end]]" || head == "[[col]]" || head == "[[tab]]" {
        return None;
    }

    if t.starts_with('>') {
        let mut parts = Vec::new();
        while *i < lines.len() {
            let q = lines[*i].trim_start();
            if let Some(rest) = q.strip_prefix('>') {
                parts.push(rest.trim_start().to_string());
                *i += 1;
            } else {
                break;
            }
        }
        return Some(Block::Quote(inlines(&parts.join(" "))));
    }

    if let Some(first) = bullet_item(t) {
        let mut items = vec![inlines(first)];
        *i += 1;
        while *i < lines.len() {
            let q = lines[*i].trim_start();
            if let Some(rest) = bullet_item(q) {
                items.push(inlines(rest));
                *i += 1;
            } else {
                break;
            }
        }
        return Some(Block::List {
            ordered: false,
            items,
        });
    }

    if ordered_item(t).is_some() {
        let mut items = Vec::new();
        while *i < lines.len() {
            let q = lines[*i].trim_start();
            match ordered_item(q) {
                Some(rest) => {
                    items.push(inlines(rest));
                    *i += 1;
                }
                None => break,
            }
        }
        return Some(Block::List {
            ordered: true,
            items,
        });
    }

    if t.starts_with('|') {
        let mut pre = Vec::new();
        while *i < lines.len() {
            let q = lines[*i].trim_start();
            if let Some(rest) = q.strip_prefix('|') {
                pre.push(rest.strip_prefix(' ').unwrap_or(rest).to_string());
                *i += 1;
            } else {
                break;
            }
        }
        return Some(Block::Pre {
            lang: None,
            text: pre.join("\n"),
        });
    }

    if let Some(rest) = t.strip_prefix("!img ") {
        let mut it = rest.split_whitespace();
        if let Some(id) = it.next().and_then(|v| v.parse::<u16>().ok()) {
            let pct = it
                .next()
                .and_then(|v| v.trim_end_matches('%').parse::<f32>().ok())
                .map(|v| (v / 100.0).clamp(0.05, 1.0))
                .unwrap_or(1.0);
            // Optional `WxH` from publish: lets the client reserve the right box
            // before the image arrives.
            let ratio = it
                .next()
                .and_then(|v| v.split_once('x'))
                .and_then(|(w, h)| {
                    let w = w.parse::<f32>().ok()?;
                    let h = h.parse::<f32>().ok()?;
                    (w > 0.0 && h > 0.0).then(|| w / h)
                });
            *i += 1;
            return Some(Block::Image { id, pct, ratio });
        }
        *i += 1;
        return None;
    }

    if let Some(rest) = t.strip_prefix("!cap ") {
        *i += 1;
        return Some(Block::Caption(inlines(rest)));
    }

    if let Some(rest) = t.strip_prefix("[[cols ") {
        return parse_columns(lines, i, inner_spec(rest));
    }

    if head == "[[table]]" {
        return parse_table(lines, i);
    }

    if let Some(rest) = t.strip_prefix("[[note") {
        return parse_note(lines, i, inner_spec(rest));
    }

    if let Some(rest) = t.strip_prefix("[[details") {
        return parse_details(lines, i, inner_spec(rest));
    }

    if let Some(rest) = t.strip_prefix("[[tabs") {
        return parse_tabs(lines, i, inner_spec(rest));
    }

    if let Some(rest) = t.strip_prefix("[[scene") {
        return parse_scene(lines, i, inner_spec(rest));
    }

    if let Some(rest) = t.strip_prefix("[[term") {
        return parse_term(lines, i, inner_spec(rest));
    }

    if head == "[[sheet]]" {
        *i += 1;
        return Some(Block::Sheet(parse_sheet(lines, i)));
    }

    if let Some(rest) = t.strip_prefix("[[box") {
        let attrs = inner_spec(rest);
        *i += 1;
        let (mut children, _) = parse_seq(lines, i, &["[[end]]"]);
        trim_gaps(&mut children);
        return Some(Block::Box {
            style: Style::from_attrs(attrs),
            children,
        });
    }

    if let Some(rest) = t.strip_prefix("[[code") {
        let lang = inner_spec(rest);
        *i += 1;
        let text = raw_until_end(lines, i).join("\n");
        return Some(Block::Pre {
            lang: (!lang.is_empty()).then(|| lang.to_string()),
            text,
        });
    }

    if let Some(rest) = t.strip_prefix("[[form ") {
        *i += 1;
        if let Some(id) = rest.trim_end_matches("]]").trim().parse::<u16>().ok() {
            return Some(Block::Form { target: id });
        }
        return None;
    }

    if let Some(rest) = t.strip_prefix("[[input ") {
        *i += 1;
        return named_field(rest).map(|(name, label)| Block::Input {
            name,
            label,
            secret: false,
        });
    }

    if let Some(rest) = t.strip_prefix("[[secret ") {
        *i += 1;
        return named_field(rest).map(|(name, label)| Block::Input {
            name,
            label,
            secret: true,
        });
    }

    if let Some(rest) = t.strip_prefix("[[avatar ") {
        *i += 1;
        let mut parts = inner_spec(rest).split_whitespace();
        let name = parts.next().unwrap_or_default().to_string();
        let size = parts
            .next()
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(40.0)
            .clamp(12.0, 160.0);
        return Some(Block::Avatar { name, size });
    }

    if let Some(rest) = t.strip_prefix("[[prefill ") {
        *i += 1;
        let rest = inner_spec(rest);
        let (name, value) = rest.split_once('|').unwrap_or((rest, ""));
        return Some(Block::Prefill {
            name: name.trim().to_string(),
            value: value.to_string(),
        });
    }

    if let Some(rest) = t.strip_prefix("[[hidden ") {
        *i += 1;
        let body = rest.trim_end_matches("]]");
        return split_pipe(body).and_then(|(name, value)| {
            let name = name.trim().to_string();
            if name.is_empty() {
                return None;
            }
            Some(Block::Hidden {
                name,
                value: value.trim().to_string(),
            })
        });
    }

    if let Some(rest) = t.strip_prefix("[[textarea ") {
        *i += 1;
        return named_field(rest).map(|(name, label)| Block::Textarea { name, label });
    }

    if let Some(rest) = t.strip_prefix("[[check ") {
        *i += 1;
        return named_field(rest).map(|(name, label)| Block::Checkbox { name, label });
    }


    if let Some(rest) = t.strip_prefix("[[button ") {
        *i += 1;
        return Some(Block::Button {
            label: rest.trim_end_matches("]]").trim().to_string(),
        });
    }

    if let Some((target, text)) = page_link(head) {
        *i += 1;
        return Some(Block::Link { target, text });
    }

    // Unknown `[[...]]` directive: show it as text instead of dropping it.
    if t.starts_with("[[") {
        *i += 1;
        return Some(Block::Para(inlines(head)));
    }

    // Paragraph: consume until a blank line or a block-starting line.
    let mut parts = Vec::new();
    while *i < lines.len() {
        let q = lines[*i].trim_start();
        if q.trim().is_empty() || starts_block(q) {
            break;
        }
        parts.push(q.to_string());
        *i += 1;
    }
    if parts.is_empty() {
        *i += 1;
        return None;
    }
    Some(Block::Para(inlines(&parts.join(" "))))
}

fn parse_columns(lines: &[&str], i: &mut usize, spec: &str) -> Option<Block> {
    // 客户端不设上限：页面要几栏就几栏，窄窗口里自然变挤，这是页面自己的选择。
    let wanted = spec.parse::<u16>().unwrap_or(2).clamp(1, 12) as usize;
    *i += 1;
    let mut segments = Vec::new();
    loop {
        let (mut blocks, stop) = parse_seq(lines, i, &["[[col]]", "[[end]]"]);
        trim_gaps(&mut blocks);
        segments.push(blocks);
        if stop.as_deref() != Some("[[col]]") {
            break;
        }
    }

    let mut parts: Vec<Vec<Block>> = segments.into_iter().filter(|p| !p.is_empty()).collect();
    // No explicit breaks: split the blocks into equal-ish columns.
    if parts.len() == 1 && wanted > 1 {
        let all = parts.pop().unwrap_or_default();
        let per = all.len().div_ceil(wanted).max(1);
        parts = all.chunks(per).map(|c| c.to_vec()).collect();
    }
    if parts.is_empty() {
        return None;
    }
    Some(Block::Columns(parts))
}

fn parse_table(lines: &[&str], i: &mut usize) -> Option<Block> {
    *i += 1;
    let raw = raw_until_end(lines, i);
    let mut parsed: Vec<Vec<Vec<Inline>>> = raw
        .iter()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .map(cells)
        .collect();
    if parsed.is_empty() {
        return None;
    }
    let header = parsed.remove(0);
    Some(Block::Table {
        header,
        rows: parsed,
    })
}

fn parse_note(lines: &[&str], i: &mut usize, spec: &str) -> Option<Block> {
    let (kind, title) = match spec.split_once('|') {
        Some((kind, title)) => (
            NoteKind::parse(kind).unwrap_or(NoteKind::Info),
            title.trim().to_string(),
        ),
        None => (
            NoteKind::parse(spec).unwrap_or(NoteKind::Info),
            String::new(),
        ),
    };
    *i += 1;
    let (mut body, _) = parse_seq(lines, i, &["[[end]]"]);
    trim_gaps(&mut body);
    Some(Block::Note {
        kind,
        title: inlines(&title),
        body,
    })
}

fn parse_details(lines: &[&str], i: &mut usize, spec: &str) -> Option<Block> {
    *i += 1;
    let (mut body, _) = parse_seq(lines, i, &["[[end]]"]);
    trim_gaps(&mut body);
    Some(Block::Details {
        summary: inlines(spec),
        body,
    })
}

fn parse_tabs(lines: &[&str], i: &mut usize, spec: &str) -> Option<Block> {
    let titles: Vec<String> = spec
        .split(';')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    if titles.is_empty() {
        return None;
    }
    *i += 1;
    let mut bodies = Vec::new();
    loop {
        let (mut blocks, stop) = parse_seq(lines, i, &["[[tab]]", "[[end]]"]);
        trim_gaps(&mut blocks);
        bodies.push(blocks);
        if stop.as_deref() != Some("[[tab]]") {
            break;
        }
    }
    Some(Block::Tabs { titles, bodies })
}

/// `.name { key: value; key: value }` … up to `[[end]]`.
///
/// One class per rule, no nesting, no selectors. A second rule for the same name
/// wins, which is the only cascade there is.
fn parse_sheet(lines: &[&str], i: &mut usize) -> Vec<(String, Style)> {
    let mut out: Vec<(String, Style)> = Vec::new();
    while *i < lines.len() {
        let line = lines[*i].trim();
        *i += 1;
        if line == "[[end]]" {
            break;
        }
        if line.is_empty() {
            continue;
        }
        let Some((selector, body)) = line.split_once('{') else {
            continue;
        };
        // One class name, nothing else. `.a .b`, `.a > .b`, `@media`, `a:hover`
        // and `[data-x]` are all dropped here, so they can never quietly start
        // working: the style layer is not allowed to grow selectors.
        let name = selector.trim().trim_start_matches('.');
        let plain = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !plain {
            continue;
        }
        let name = name.to_string();
        let body = body.trim_end_matches('}');
        let style = Style::from_declarations(body);
        if let Some(slot) = out.iter_mut().find(|(existing, _)| *existing == name) {
            slot.1 = style.over(&slot.1);
        } else {
            out.push((name, style));
        }
    }
    out
}

/// `[[term 版式|标题|页号]]`, then one directive per line, then `[[end]]`.
///
/// Line directives:
///   `[[ascii 文字]]`      banner art — monospace, fixed leading
///   `[[line r|文字|r|文字]]`  runs of coloured text
///   `[[cursor 名|r|文字]]`    runs of coloured text, then a text field
///   `[[passwd 名|r|文字]]`    same, but the field hides what is typed
///
/// `|` is the separator, so pages replace any literal `|` before emitting.
fn parse_term(lines: &[&str], i: &mut usize, spec: &str) -> Option<Block> {
    let mut parts = spec.split('|').map(str::trim);
    let flavor = parts.next().unwrap_or("");
    let title = parts.next().unwrap_or("").to_string();
    let target = parts
        .next()
        .and_then(|v| v.parse::<u16>().ok())
        .filter(|v| *v != 0);
    if flavor != "panel" && flavor != "bare" {
        return None;
    }
    let bare = flavor == "bare";

    *i += 1;
    let mut out = Vec::new();
    while *i < lines.len() {
        let head = lines[*i].trim();
        *i += 1;
        if head == "[[end]]" {
            break;
        }
        if head.is_empty() {
            out.push(TermLine::Text(Vec::new()));
            continue;
        }
        if let Some(rest) = head.strip_prefix("[[ascii ") {
            out.push(TermLine::Ascii(
                rest.trim_end_matches(']').trim_end().to_string(),
            ));
            continue;
        }
        if let Some(rest) = head.strip_prefix("[[line ") {
            out.push(TermLine::Text(term_runs(unclose(rest))));
            continue;
        }
        let field = head
            .strip_prefix("[[cursor ")
            .map(|rest| (rest, false))
            .or_else(|| head.strip_prefix("[[passwd ").map(|rest| (rest, true)));
        if let Some((rest, secret)) = field {
            let rest = unclose(rest).trim_end();
            let (name, tail) = rest.split_once('|').unwrap_or((rest, ""));
            out.push(TermLine::Field {
                name: name.trim().to_string(),
                secret,
                lead: term_runs(tail),
            });
            continue;
        }
        // Unknown directive inside a panel: keep it as plain text rather than
        // dropping it, so a typo shows up instead of silently vanishing.
        out.push(TermLine::Text(vec![(TermRole::Out, head.to_string())]));
    }

    Some(Block::Term(Term {
        title,
        target,
        bare,
        lines: out,
    }))
}

/// Drops the closing `]]` only, so text that legitimately ends in `]` survives.
fn unclose(text: &str) -> &str {
    text.strip_suffix("]]").unwrap_or(text)
}

/// `r|文字|r|文字` → pairs. A trailing orphan run is treated as plain output.
fn term_runs(text: &str) -> Vec<(TermRole, String)> {
    let parts: Vec<&str> = text.split('|').collect();
    let mut out = Vec::new();
    let mut index = 0;
    while index < parts.len() {
        if index + 1 < parts.len() {
            out.push((
                TermRole::parse(parts[index].trim()),
                parts[index + 1].to_string(),
            ));
            index += 2;
        } else {
            out.push((TermRole::Out, parts[index].to_string()));
            index += 1;
        }
    }
    out
}

/// Reads raw lines up to and including `[[end]]`. Used where the body is not
/// markup at all, so nothing inside should be intepreted.
fn raw_until_end(lines: &[&str], i: &mut usize) -> Vec<String> {
    let mut out = Vec::new();
    while *i < lines.len() {
        let line = lines[*i];
        *i += 1;
        if line.trim() == "[[end]]" {
            break;
        }
        out.push(line.to_string());
    }
    out
}

/// Strips the trailing `]]` from the tail of an opening directive.
fn inner_spec(rest: &str) -> &str {
    rest.trim_end_matches(']').trim()
}

/// `[[scene 600x220 6.0]]`, then `[[bar rrggbb w h]]` and its `[[at t x y]]`
/// lines, then `[[end]]`.
///
/// The page is free to emit as many bars and keys as fit; the client only reads
/// times and positions, so the same block animates anything — a chart, a
/// progress bar, falling blocks.
fn parse_scene(lines: &[&str], i: &mut usize, spec: &str) -> Option<Block> {
    let mut parts = spec.split_whitespace();
    let size = parts.next()?;
    let (w, h) = size.split_once('x')?;
    let width = w.trim().parse::<f32>().ok()?;
    let height = h.trim().parse::<f32>().ok()?;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let seconds = parts
        .next()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(1.0)
        .max(0.0);

    *i += 1;
    let mut bars: Vec<SceneBar> = Vec::new();
    while *i < lines.len() {
        let line = lines[*i].trim();
        *i += 1;
        if line == "[[end]]" {
            break;
        }
        if let Some(rest) = line.strip_prefix("[[bar ") {
            let body = inner_spec(rest);
            let mut f = body.split_whitespace();
            let Some(color) = f.next().and_then(parse_color) else {
                continue;
            };
            let bw = f.next().and_then(|v| v.parse::<f32>().ok()).unwrap_or(10.0);
            let bh = f.next().and_then(|v| v.parse::<f32>().ok()).unwrap_or(10.0);
            bars.push(SceneBar {
                color,
                width: bw,
                height: bh,
                keys: Vec::new(),
            });
            continue;
        }
        if let Some(rest) = line.strip_prefix("[[at ") {
            let body = inner_spec(rest);
            let mut f = body.split_whitespace();
            let numbers: Vec<f32> = (0..3)
                .map(|_| f.next().and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0))
                .collect();
            if let Some(bar) = bars.last_mut() {
                bar.keys.push((numbers[0], numbers[1], numbers[2]));
            }
        }
    }

    for bar in &mut bars {
        bar.keys.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    }
    if bars.is_empty() {
        return None;
    }
    Some(Block::Scene {
        width,
        height,
        seconds,
        bars,
    })
}

/// `c0392b` or `#c0392b`.
fn parse_color(text: &str) -> Option<[u8; 3]> {
    let hex = text.trim_start_matches('#');
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

fn cells(line: &str) -> Vec<Vec<Inline>> {
    let inner = line.trim();
    let inner = inner.strip_prefix('|').unwrap_or(inner);
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut depth = 0usize;
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '[' && chars.peek() == Some(&'[') {
            depth += 1;
            buf.push('[');
            buf.push(chars.next().unwrap_or('['));
            continue;
        }
        if c == ']' && chars.peek() == Some(&']') && depth > 0 {
            depth -= 1;
            buf.push(']');
            buf.push(chars.next().unwrap_or(']'));
            continue;
        }
        if c == '|' && depth == 0 {
            out.push(inlines(buf.trim()));
            buf.clear();
            continue;
        }
        buf.push(c);
    }
    out.push(inlines(buf.trim()));
    out
}

/// `[[input name|标签]]` and friends: the name defaults to the label when the
/// label is left empty.
fn named_field(rest: &str) -> Option<(String, String)> {
    let (name, label) = split_pipe(rest.trim_end_matches("]]"))?;
    let name = name.trim().to_string();
    if name.is_empty() {
        return None;
    }
    let label = label.trim();
    Some((
        name.clone(),
        if label.is_empty() {
            name
        } else {
            label.to_string()
        },
    ))
}

pub fn plain(parts: &[Inline]) -> String {
    let mut s = String::new();
    for p in parts {
        match p {
            Inline::Text(t) | Inline::Bold(t) | Inline::Italic(t) | Inline::Code(t) => {
                s.push_str(t)
            }
            Inline::Link { text, .. } => s.push_str(text),
        }
    }
    s
}

fn trim_gaps(blocks: &mut Vec<Block>) {
    while matches!(blocks.first(), Some(Block::Gap)) {
        blocks.remove(0);
    }
    while matches!(blocks.last(), Some(Block::Gap)) {
        blocks.pop();
    }
}

fn push_gap(out: &mut Vec<Block>) {
    if !matches!(out.last(), Some(Block::Gap)) {
        out.push(Block::Gap);
    }
}

fn heading(line: &str) -> Option<(u8, &str)> {
    // 一到六级都认（Markdown 的写法），样式只分三级：一级标题、二级标题、
    // 其余一律按三级画 —— 文档里明说了这一点。
    for (prefix, level) in [
        ("###### ", 6u8),
        ("##### ", 5),
        ("#### ", 4),
        ("### ", 3),
        ("## ", 2),
        ("# ", 1),
    ] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return Some((level, rest.trim()));
        }
    }
    None
}

/// `- 内容` / `* 内容` / `+ 内容` — Markdown 允许三种圆点，都当成无序列表。
fn bullet_item(line: &str) -> Option<&str> {
    for marker in ["- ", "* ", "+ "] {
        if let Some(rest) = line.strip_prefix(marker) {
            return Some(rest);
        }
    }
    None
}

/// `1. 内容` → `内容`. The number itself is not used.
fn ordered_item(line: &str) -> Option<&str> {
    let (number, rest) = line.split_once(". ")?;
    if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(rest)
}

fn starts_block(line: &str) -> bool {
    line.trim_end() == "---"
        || line.starts_with('>')
        || line.starts_with("- ")
        || ordered_item(line).is_some()
        || line.starts_with('|')
        || line.starts_with("!img ")
        || line.starts_with("!cap ")
        || line.starts_with("[[")
        || heading(line).is_some()
}

fn split_pipe(s: &str) -> Option<(&str, &str)> {
    s.split_once('|')
}

fn page_link(line: &str) -> Option<(LinkTarget, String)> {
    let rest = line.strip_prefix("[[")?;
    let end = rest.find("]]")?;
    // A whole-line link has to *be* the whole line. Anything after the closing
    // bracket means this is a paragraph that happens to start with a link, and
    // those used to lose every word after it.
    if !rest[end + 2..].trim().is_empty() {
        return None;
    }
    bracket_link(&rest[..end])
}

/// `[文字](/path)` / `[文字](https://…)` / `[文字](31345)` —— Markdown 的链接。
///
/// 目标是 `/` 开头就按站内路径，`http(s)://` 开头交给系统浏览器，纯数字当页号。
/// 其它（`file:`、`javascript:`、自定义协议）一律不认，链接就退化成普通文字。
fn markdown_link(target: &str) -> Option<LinkTarget> {
    let raw = target.trim();
    if raw.is_empty() {
        return None;
    }
    if let Some(rest) = raw.strip_prefix('/') {
        let path = format!("/{rest}");
        return Some(LinkTarget::Path(path));
    }
    let lower = raw.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("http://") {
        return Some(LinkTarget::Url(raw.to_string()));
    }
    raw.parse::<u16>().ok().map(LinkTarget::Page)
}

fn bracket_link(inner: &str) -> Option<(LinkTarget, String)> {
    let (raw, text) = inner.split_once('|')?;
    let raw = raw.trim();
    let target = if let Some(path) = raw.strip_prefix("path:") {
        let path = path.trim();
        if path.is_empty() {
            return None;
        }
        LinkTarget::Path(if path.starts_with('/') {
            path.to_string()
        } else {
            format!("/{path}")
        })
    } else if let Some(url) = raw.strip_prefix("url:") {
        let url = url.trim();
        let lower = url.to_ascii_lowercase();
        if !(lower.starts_with("https://") || lower.starts_with("http://")) {
            return None;
        }
        LinkTarget::Url(url.to_string())
    } else {
        LinkTarget::Page(raw.parse::<u16>().ok()?)
    };
    Some((target, text.trim().to_string()))
}

/// Reads `[文字](目标)` starting at `open` (which must be `[`).
/// Returns the target, the text, and where to continue after the `)`.
fn scan_markdown_link(cs: &[char], open: usize) -> Option<(LinkTarget, String, usize)> {
    let close_text = (open + 1..cs.len()).find(|&j| cs[j] == ']')?;
    if close_text + 1 >= cs.len() || cs[close_text + 1] != '(' {
        return None;
    }
    let close_target = (close_text + 2..cs.len()).find(|&j| cs[j] == ')')?;
    let text: String = cs[open + 1..close_text].iter().collect();
    let target: String = cs[close_text + 2..close_target].iter().collect();
    let target = markdown_link(&target)?;
    Some((target, text, close_target + 1))
}

/// Finds `needle` (a two-character run such as `**`) starting at `from`,
/// returning the index where it starts. Used so `**a**` closes on `**`, not on
/// the first `*` of the closing pair.
fn find_run(cs: &[char], from: usize, needle: &str) -> Option<usize> {
    let pair: Vec<char> = needle.chars().collect();
    if pair.len() != 2 || from >= cs.len() {
        return None;
    }
    (from..cs.len().saturating_sub(1)).find(|&j| cs[j] == pair[0] && cs[j + 1] == pair[1])
}

pub fn inlines(s: &str) -> Vec<Inline> {
    let cs: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut i = 0;

    while i < cs.len() {
        let c = cs[i];
        if c == '\\' && i + 1 < cs.len() {
            buf.push(cs[i + 1]);
            i += 2;
            continue;
        }
        // `[文字](目标)` —— Markdown 的写法。
        if c == '[' {
            if let Some((target, text, next)) = scan_markdown_link(&cs, i) {
                if !buf.is_empty() {
                    out.push(Inline::Text(std::mem::take(&mut buf)));
                }
                out.push(Inline::Link { target, text });
                i = next;
                continue;
            }
        }
        // `[[12|文字]]` inside a run of text is a link, not literal brackets.
        if c == '[' && i + 1 < cs.len() && cs[i + 1] == '[' {
            let search = i + 2..cs.len().saturating_sub(1);
            if let Some(end) = search.into_iter().find(|&j| cs[j] == ']' && cs[j + 1] == ']') {
                let inner: String = cs[i + 2..end].iter().collect();
                if let Some((target, text)) = bracket_link(&inner) {
                    if !buf.is_empty() {
                        out.push(Inline::Text(std::mem::take(&mut buf)));
                    }
                    out.push(Inline::Link { target, text });
                    i = end + 2;
                    continue;
                }
            }
        }
        // Markdown 的强调：`**粗**` / `__粗__`，`*斜*` / `_斜_`。
        // 两个字符一起看，所以双写优先；单写才是斜体。
        if (c == '*' || c == '_') && i + 1 < cs.len() && cs[i + 1] == c {
            let doubled = format!("{c}{c}");
            if let Some(end) = find_run(&cs, i + 2, &doubled) {
                if end > i + 2 {
                    if !buf.is_empty() {
                        out.push(Inline::Text(std::mem::take(&mut buf)));
                    }
                    out.push(Inline::Bold(cs[i + 2..end].iter().collect()));
                    i = end + 2;
                    continue;
                }
            }
        }
        if c == '*' || c == '_' || c == '`' {
            if let Some(end) = (i + 1..cs.len()).find(|&j| cs[j] == c) {
                if end > i + 1 {
                    if !buf.is_empty() {
                        out.push(Inline::Text(std::mem::take(&mut buf)));
                    }
                    let inner: String = cs[i + 1..end].iter().collect();
                    out.push(match c {
                        '*' | '_' => Inline::Italic(inner),
                        _ => Inline::Code(inner),
                    });
                    i = end + 1;
                    continue;
                }
            }
        }
        buf.push(c);
        i += 1;
    }

    if !buf.is_empty() {
        out.push(Inline::Text(buf));
    }
    out
}

#[cfg(test)]
#[path = "markup_tests.rs"]
mod tests;
