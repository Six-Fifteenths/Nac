//! Line-oriented markup parser.
//!
//! The vocabulary is small on purpose, but the blocks nest: a column can hold a
//! table, a note can hold a list. Parsing is recursive — every container
//! (`[[cols]]`, `[[note]]`, `[[details]]`, `[[tabs]]`) opens a new sequence and
//! returns at its own `[[end]]`, so nesting depth is bounded by the input rather
//! than by bespoke depth counters.

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
}

/// Severity of a `[[note]]` box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteKind {
    Info,
    Ok,
    Warn,
    Danger,
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
    /// `- item`
    List(Vec<Vec<Inline>>),
    /// `1. item` — the numbers written in the source are ignored.
    Ordered(Vec<Vec<Inline>>),
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
    Select {
        name: String,
        label: String,
        options: Vec<String>,
    },
    Checkbox {
        name: String,
        label: String,
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

    if head == "---" {
        *i += 1;
        return Some(Block::Hr);
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

    if t.starts_with("- ") {
        let mut items = Vec::new();
        while *i < lines.len() {
            let q = lines[*i].trim_start();
            if let Some(rest) = q.strip_prefix("- ") {
                items.push(inlines(rest));
                *i += 1;
            } else {
                break;
            }
        }
        return Some(Block::List(items));
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
        return Some(Block::Ordered(items));
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

    if let Some(rest) = t.strip_prefix("[[textarea ") {
        *i += 1;
        return named_field(rest).map(|(name, label)| Block::Textarea { name, label });
    }

    if let Some(rest) = t.strip_prefix("[[check ") {
        *i += 1;
        return named_field(rest).map(|(name, label)| Block::Checkbox { name, label });
    }

    if let Some(rest) = t.strip_prefix("[[select ") {
        *i += 1;
        let body = rest.trim_end_matches("]]");
        let mut parts = body.splitn(3, '|');
        let name = parts.next()?.trim().to_string();
        let label = parts.next().unwrap_or("").trim().to_string();
        let options: Vec<String> = parts
            .next()
            .unwrap_or("")
            .split(';')
            .map(|o| o.trim().to_string())
            .filter(|o| !o.is_empty())
            .collect();
        if name.is_empty() || options.is_empty() {
            return None;
        }
        return Some(Block::Select {
            name,
            label: if label.is_empty() {
                "选择".to_string()
            } else {
                label
            },
            options,
        });
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
    let wanted = spec.parse::<u8>().unwrap_or(2).clamp(1, 4) as usize;
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

fn cells(line: &str) -> Vec<Vec<Inline>> {
    let inner = line.trim().trim_matches('|');
    inner.split('|').map(|c| inlines(c.trim())).collect()
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
    for (prefix, level) in [("### ", 3u8), ("## ", 2), ("# ", 1)] {
        if let Some(rest) = line.strip_prefix(prefix) {
            return Some((level, rest.trim()));
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
    bracket_link(&rest[..end])
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
    } else {
        LinkTarget::Page(raw.parse::<u16>().ok()?)
    };
    Some((target, text.trim().to_string()))
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
        if c == '*' || c == '_' || c == '`' {
            if let Some(end) = (i + 1..cs.len()).find(|&j| cs[j] == c) {
                if end > i + 1 {
                    if !buf.is_empty() {
                        out.push(Inline::Text(std::mem::take(&mut buf)));
                    }
                    let inner: String = cs[i + 1..end].iter().collect();
                    out.push(match c {
                        '*' => Inline::Bold(inner),
                        '_' => Inline::Italic(inner),
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
mod tests {
    use super::*;

    fn only(blocks: &[Block]) -> &Block {
        blocks.first().unwrap_or_else(|| panic!("期望至少一个块"))
    }

    #[test]
    fn columns_split_evenly_or_at_markers() {
        // No markers: split into the requested number of columns.
        let auto = parse("[[cols 2]]\na\n\nb\n\nc\n\nd\n[[end]]\n");
        match only(&auto) {
            Block::Columns(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(parts.iter().all(|p| !p.is_empty()));
            }
            other => panic!("期望列布局，得到 {other:?}"),
        }

        // Explicit break wins over the requested count.
        let marked = parse("[[cols 2]]\n# 左\n[[col]]\n# 右\n[[col]]\n# 第三\n[[end]]\n");
        match only(&marked) {
            Block::Columns(parts) => {
                assert_eq!(parts.len(), 3);
                assert!(matches!(parts[0][0], Block::Heading(1, _)));
            }
            other => panic!("期望列布局，得到 {other:?}"),
        }
    }

    #[test]
    fn columns_can_nest() {
        let src = "[[cols 2]]\n[[cols 2]]\n甲\n[[col]]\n乙\n[[end]]\n[[col]]\n丙\n[[end]]\n";
        let blocks = parse(src);
        match only(&blocks) {
            Block::Columns(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(parts[0][0], Block::Columns(_)));
            }
            other => panic!("期望列布局，得到 {other:?}"),
        }
    }

    #[test]
    fn image_size_becomes_an_aspect_ratio() {
        let blocks = parse("!img 4 50% 64x32\n!img 5\n!img 6 100% 0x0\n");
        let images: Vec<(u16, Option<f32>)> = blocks
            .iter()
            .filter_map(|b| match b {
                Block::Image { id, ratio, .. } => Some((*id, *ratio)),
                _ => None,
            })
            .collect();
        assert_eq!(images.len(), 3);
        assert_eq!(images[0], (4, Some(2.0)));
        assert_eq!(images[1], (5, None));
        assert_eq!(images[2], (6, None));
    }

    #[test]
    fn parses_structure() {
        let src = "# 标题\n\n一段 *粗* 与 _斜_ 和 `码`。\n\n> 引用\n\n- a\n- b\n\n!img 3 80%\n\n[[2|第二页]]\n[[path:/notes/|笔记]]\n[[form 7]]\n[[input name|名字]]\n[[button 提交]]\n";
        let blocks = parse(src);
        assert!(blocks.iter().any(|b| matches!(b, Block::Heading(1, _))));
        assert!(blocks.iter().any(|b| matches!(b, Block::Para(_))));
        assert!(blocks.iter().any(|b| matches!(b, Block::Quote(_))));
        assert!(blocks
            .iter()
            .any(|b| matches!(b, Block::List(items) if items.len() == 2)));
        assert!(blocks.iter().any(|b| matches!(b, Block::Image { id: 3, .. })));
        assert!(blocks.iter().any(|b| matches!(
            b,
            Block::Link {
                target: LinkTarget::Page(2),
                ..
            }
        )));
        assert!(blocks.iter().any(|b| matches!(
            b,
            Block::Link { target: LinkTarget::Path(path), .. } if path == "/notes/"
        )));
        assert!(blocks.iter().any(|b| matches!(b, Block::Form { target: 7 })));
        assert!(blocks
            .iter()
            .any(|b| matches!(b, Block::Input { secret: false, .. })));
        assert!(blocks.iter().any(|b| matches!(b, Block::Button { .. })));

        let form = blocks
            .iter()
            .position(|b| matches!(b, Block::Form { .. }))
            .unwrap();
        let button = blocks
            .iter()
            .position(|b| matches!(b, Block::Button { .. }))
            .unwrap();
        assert!(form < button);
    }

    #[test]
    fn inline_runs() {
        let parts = inlines("a *b* c");
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1], Inline::Bold("b".into()));
    }

    #[test]
    fn escaped_markers_stay_text() {
        let parts = inlines(r"a \*b\* c");
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0], Inline::Text("a *b* c".into()));
    }

    #[test]
    fn ignores_unknown_directive() {
        let blocks = parse("[[nonsense]]\n");
        assert!(blocks.iter().any(|b| matches!(b, Block::Para(_))));
    }

    #[test]
    fn link_inside_a_paragraph() {
        let parts = inlines("见 [[path:/notes/|笔记]] 与 [[7|第七页]]");
        assert_eq!(
            parts[0],
            Inline::Text("见 ".into()),
            "链接前面那段文字要保住"
        );
        assert_eq!(
            parts[1],
            Inline::Link {
                target: LinkTarget::Path("/notes/".into()),
                text: "笔记".into()
            }
        );
        assert_eq!(
            parts[3],
            Inline::Link {
                target: LinkTarget::Page(7),
                text: "第七页".into()
            }
        );
    }

    #[test]
    fn ordered_list_and_rule() {
        let blocks = parse("1. 甲\n2. 乙\n\n---\n");
        assert!(blocks
            .iter()
            .any(|b| matches!(b, Block::Ordered(items) if items.len() == 2)));
        assert!(blocks.iter().any(|b| matches!(b, Block::Hr)));
    }

    #[test]
    fn table_takes_the_first_row_as_header() {
        let src = "[[table]]\n| 名字 | 说明 |\n| 甲 | *第一* |\n| 乙 | 第二 |\n[[end]]\n";
        match only(&parse(src)) {
            Block::Table { header, rows } => {
                assert_eq!(header.len(), 2);
                assert_eq!(plain(&header[0]), "名字");
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0][1], vec![Inline::Bold("第一".into())]);
            }
            other => panic!("期望表格，得到 {other:?}"),
        }
    }

    #[test]
    fn note_and_details_hold_blocks() {
        let src = "[[note warn|小心]]\n第一段\n\n- 甲\n[[end]]\n\n[[details 展开]]\n藏着\n[[end]]\n";
        let blocks = parse(src);
        match &blocks[0] {
            Block::Note { kind, title, body } => {
                assert_eq!(*kind, NoteKind::Warn);
                assert_eq!(plain(title), "小心");
                assert!(body.iter().any(|b| matches!(b, Block::List(_))));
            }
            other => panic!("期望提示框，得到 {other:?}"),
        }
        match blocks.iter().find(|b| matches!(b, Block::Details { .. })) {
            Some(Block::Details { summary, body }) => {
                assert_eq!(plain(summary), "展开");
                assert!(!body.is_empty());
            }
            other => panic!("期望折叠块，得到 {other:?}"),
        }
    }

    #[test]
    fn tabs_split_at_each_tab_marker() {
        let src = "[[tabs 甲;乙]]\n第一页\n[[tab]]\n第二页\n[[end]]\n";
        match only(&parse(src)) {
            Block::Tabs { titles, bodies } => {
                assert_eq!(titles.as_slice(), ["甲", "乙"]);
                assert_eq!(bodies.len(), 2);
                assert!(matches!(bodies[0][0], Block::Para(_)));
            }
            other => panic!("期望分页，得到 {other:?}"),
        }
    }

    #[test]
    fn form_controls() {
        let src = "[[textarea body|正文]]\n[[select mode|模式|甲;乙;丙]]\n[[check agree|同意]]\n";
        let blocks = parse(src);
        assert!(blocks.iter().any(
            |b| matches!(b, Block::Textarea { name, label } if name == "body" && label == "正文")
        ));
        assert!(blocks.iter().any(|b| matches!(
            b,
            Block::Select { options, .. } if options == &vec!["甲".to_string(), "乙".to_string(), "丙".to_string()]
        )));
        assert!(blocks
            .iter()
            .any(|b| matches!(b, Block::Checkbox { name, .. } if name == "agree")));
    }

    #[test]
    fn caption_after_an_image() {
        let blocks = parse("!img 1\n!cap 一张图\n");
        assert!(matches!(blocks[0], Block::Image { id: 1, .. }));
        assert!(matches!(&blocks[1], Block::Caption(parts) if plain(parts) == "一张图"));
    }
}