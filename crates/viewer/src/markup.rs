//! Line-oriented markup parser. Deliberately small: 10 block kinds, 3 inline kinds.

#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Heading(u8, Vec<Inline>),
    Para(Vec<Inline>),
    Quote(Vec<Inline>),
    List(Vec<Vec<Inline>>),
    Pre(String),
    Image { id: u16, pct: f32, ratio: Option<f32> },
    Link { target: u16, text: String },
    Form { target: u16 },
    Input { name: String, label: String, secret: bool },
    Button { label: String },
    Gap,
    /// Side-by-side layout: `[[cols 2]] … [[end]]`, with `[[col]]` to force a
    /// break. Parsing decides the actual partition, so rendering only sees the
    /// columns themselves.
    Columns(Vec<Vec<Block>>),
}

pub fn parse(text: &str) -> Vec<Block> {
    let lines: Vec<&str> = text.lines().collect();
    let mut out = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let raw = lines[i];
        let t = raw.trim_start();

        if t.is_empty() {
            push_gap(&mut out);
            i += 1;
            continue;
        }

        if let Some((level, rest)) = heading(t) {
            out.push(Block::Heading(level, inlines(rest)));
            i += 1;
            continue;
        }

        if t.starts_with('>') {
            let mut parts = Vec::new();
            while i < lines.len() {
                let q = lines[i].trim_start();
                if let Some(rest) = q.strip_prefix('>') {
                    parts.push(rest.trim_start().to_string());
                    i += 1;
                } else {
                    break;
                }
            }
            out.push(Block::Quote(inlines(&parts.join(" "))));
            continue;
        }

        if t.starts_with("- ") {
            let mut items = Vec::new();
            while i < lines.len() {
                let q = lines[i].trim_start();
                if let Some(rest) = q.strip_prefix("- ") {
                    items.push(inlines(rest));
                    i += 1;
                } else {
                    break;
                }
            }
            out.push(Block::List(items));
            continue;
        }

        if t.starts_with('|') {
            let mut pre = Vec::new();
            while i < lines.len() {
                let q = lines[i].trim_start();
                if let Some(rest) = q.strip_prefix('|') {
                    pre.push(rest.strip_prefix(' ').unwrap_or(rest).to_string());
                    i += 1;
                } else {
                    break;
                }
            }
            out.push(Block::Pre(pre.join("\n")));
            continue;
        }

        if let Some(rest) = t.strip_prefix("[[cols ") {
            let wanted = rest
                .trim_end_matches("]]")
                .trim()
                .parse::<u8>()
                .unwrap_or(2)
                .clamp(1, 4) as usize;

            let mut segments: Vec<Vec<&str>> = vec![Vec::new()];
            let mut depth = 1usize;
            let mut j = i + 1;
            while j < lines.len() {
                let q = lines[j].trim();
                if q.starts_with("[[cols ") {
                    depth += 1;
                } else if q == "[[end]]" {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                } else if q == "[[col]]" && depth == 1 {
                    segments.push(Vec::new());
                    j += 1;
                    continue;
                }
                if let Some(last) = segments.last_mut() {
                    last.push(lines[j]);
                }
                j += 1;
            }

            let mut parts: Vec<Vec<Block>> = segments
                .iter()
                .map(|s| parse(&s.join("\n")))
                .filter(|p| !p.is_empty())
                .collect();
            // No explicit breaks: split the blocks into equal-ish columns.
            if parts.len() == 1 && wanted > 1 {
                let all = parts.pop().unwrap_or_default();
                let per = all.len().div_ceil(wanted).max(1);
                parts = all.chunks(per).map(|c| c.to_vec()).collect();
            }
            if !parts.is_empty() {
                out.push(Block::Columns(parts));
            }
            i = if j < lines.len() { j + 1 } else { j };
            continue;
        }

        if let Some(rest) = t.strip_prefix("!img ") {
            let mut it = rest.split_whitespace();
            if let Some(id) = it.next().and_then(|v| v.parse::<u16>().ok()) {
                let pct = it
                    .next()
                    .and_then(|v| v.trim_end_matches('%').parse::<f32>().ok())
                    .map(|v| (v / 100.0).clamp(0.05, 1.0))
                    .unwrap_or(1.0);
                // Optional `WxH` from publish: lets the client reserve the right
                // box before the image arrives.
                let ratio = it
                    .next()
                    .and_then(|v| v.split_once('x'))
                    .and_then(|(w, h)| {
                        let w = w.parse::<f32>().ok()?;
                        let h = h.parse::<f32>().ok()?;
                        (w > 0.0 && h > 0.0).then(|| w / h)
                    });
                out.push(Block::Image { id, pct, ratio });
            }
            i += 1;
            continue;
        }

        if let Some(rest) = t.strip_prefix("[[form ") {
            if let Some(id) = rest.trim_end_matches("]]").trim().parse::<u16>().ok() {
                out.push(Block::Form { target: id });
            }
            i += 1;
            continue;
        }

        if let Some(rest) = t.strip_prefix("[[input ") {
            if let Some((name, label)) = split_pipe(rest.trim_end_matches("]]")) {
                let label = label.trim().to_string();
                let default_label = name.trim().to_string();
                out.push(Block::Input {
                    name: name.trim().to_string(),
                    label: if label.is_empty() { default_label } else { label },
                    secret: false,
                });
            }
            i += 1;
            continue;
        }

        if let Some(rest) = t.strip_prefix("[[secret ") {
            if let Some((name, label)) = split_pipe(rest.trim_end_matches("]]")) {
                let label = label.trim().to_string();
                let default_label = name.trim().to_string();
                out.push(Block::Input {
                    name: name.trim().to_string(),
                    label: if label.is_empty() { default_label } else { label },
                    secret: true,
                });
            }
            i += 1;
            continue;
        }

        if let Some(rest) = t.strip_prefix("[[button ") {
            out.push(Block::Button {
                label: rest.trim_end_matches("]]").trim().to_string(),
            });
            i += 1;
            continue;
        }

        if let Some((target, text)) = page_link(t) {
            out.push(Block::Link { target, text });
            i += 1;
            continue;
        }

        // Unknown `[[...]]` directive: show it as text instead of dropping it.
        if t.starts_with("[[") {
            out.push(Block::Para(inlines(t)));
            i += 1;
            continue;
        }

        // Paragraph: consume until a blank line or a block-starting line.
        let mut parts = Vec::new();
        while i < lines.len() {
            let q = lines[i];
            let qt = q.trim_start();
            if qt.is_empty() || starts_block(qt) {
                break;
            }
            parts.push(qt.to_string());
            i += 1;
        }
        if !parts.is_empty() {
            out.push(Block::Para(inlines(&parts.join(" "))));
        } else {
            i += 1;
        }
    }

    out
}

pub fn plain(parts: &[Inline]) -> String {
    let mut s = String::new();
    for p in parts {
        match p {
            Inline::Text(t) | Inline::Bold(t) | Inline::Italic(t) | Inline::Code(t) => s.push_str(t),
        }
    }
    s
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

fn starts_block(line: &str) -> bool {
    line.starts_with('>')
        || line.starts_with("- ")
        || line.starts_with('|')
        || line.starts_with("!img ")
        || line.starts_with("[[")
        || heading(line).is_some()
}

fn split_pipe(s: &str) -> Option<(&str, &str)> {
    s.split_once('|')
}

fn page_link(line: &str) -> Option<(u16, String)> {
    let rest = line.strip_prefix("[[")?;
    let end = rest.find("]]")?;
    let inner = &rest[..end];
    let (id, text) = inner.split_once('|')?;
    let target = id.trim().parse::<u16>().ok()?;
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

    #[test]
    fn columns_split_evenly_or_at_markers() {
        // No markers: split into the requested number of columns.
        let auto = parse("[[cols 2]]\na\n\nb\n\nc\n\nd\n[[end]]\n");
        match &auto[0] {
            Block::Columns(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(parts.iter().all(|p| !p.is_empty()));
            }
            other => panic!("期望列布局，得到 {other:?}"),
        }

        // Explicit break wins over the requested count.
        let marked = parse("[[cols 2]]\n# 左\n[[col]]\n# 右\n[[col]]\n# 第三\n[[end]]\n");
        match &marked[0] {
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
        match &blocks[0] {
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
        assert_eq!(images[0].0, 4);
        assert_eq!(images[0].1, Some(2.0));
        assert_eq!(images[1], (5, None));
        assert_eq!(images[2], (6, None));
    }

    #[test]
    fn parses_structure() {
        let src = "# 标题\n\n一段 *粗* 与 _斜_ 和 `码`。\n\n> 引用\n\n- a\n- b\n\n!img 3 80%\n\n[[2|第二页]]\n[[form 7]]\n[[input name|名字]]\n[[button 提交]]\n";
        let blocks = parse(src);
        assert!(blocks
            .iter()
            .any(|b| matches!(b, Block::Heading(1, _))));
        assert!(blocks.iter().any(|b| matches!(b, Block::Para(_))));
        assert!(blocks.iter().any(|b| matches!(b, Block::Quote(_))));
        assert!(blocks
            .iter()
            .any(|b| matches!(b, Block::List(items) if items.len() == 2)));
        assert!(blocks
            .iter()
            .any(|b| matches!(b, Block::Image { id: 3, .. })));
        assert!(blocks
            .iter()
            .any(|b| matches!(b, Block::Link { target: 2, .. })));
        assert!(blocks
            .iter()
            .any(|b| matches!(b, Block::Form { target: 7 })));
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
}
