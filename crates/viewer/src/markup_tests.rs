//! `markup` 的测试。搬出来单独一个文件，解析器本体才看得清。

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
    fn hidden_fields_carry_state_and_links_can_leave_the_site() {
        let blocks = parse("[[hidden item|42]]\n[[hidden back|/hn/item/42]]\n");
        let hidden: Vec<(&str, &str)> = blocks
            .iter()
            .filter_map(|b| match b {
                Block::Hidden { name, value } => Some((name.as_str(), value.as_str())),
                _ => None,
            })
            .collect();
        assert_eq!(hidden, [("item", "42"), ("back", "/hn/item/42")]);

        // Only http(s) addresses become outbound links; anything else stays text
        // so a page cannot hand the shell a `file:` URL.
        let parts = inlines("见 [[url:https://example.invalid/a?b=1&c=2|原文]]");
        assert!(matches!(
            &parts[1],
            Inline::Link { target: LinkTarget::Url(url), .. } if url == "https://example.invalid/a?b=1&c=2"
        ));
        let hostile = inlines("[[url:file:///etc/passwd|偷看]]");
        assert!(hostile.iter().all(|p| !matches!(p, Inline::Link { .. })));
    }

    #[test]
    fn a_terminal_panel_keeps_its_runs_in_order() {
        let src = "[[term panel|WebShell.x64_x86|65206]]\n\
                   [[ascii ███╗]]\n\
                   [[line pu|visitor|pd|@|ph|host|pd|:$ ~ |in|help]]\n\
                   \n\
                   [[cursor cmd|pd|> ]]\n\
                   [[passwd pw|out|Password: ]]\n\
                   [[end]]\n";
        match only(&parse(src)) {
            Block::Term(term) => {
                assert_eq!(term.title, "WebShell.x64_x86");
                assert_eq!(term.target, Some(65206));
                assert!(!term.bare);
                assert_eq!(term.lines.len(), 5);
                assert!(matches!(&term.lines[0], TermLine::Ascii(a) if a == "███╗"));
                match &term.lines[1] {
                    TermLine::Text(runs) => {
                        assert_eq!(runs.len(), 5);
                        assert_eq!(runs[0], (TermRole::User, "visitor".to_string()));
                        assert_eq!(runs[3], (TermRole::Prompt, ":$ ~ ".to_string()));
                        assert_eq!(runs[4], (TermRole::Input, "help".to_string()));
                    }
                    other => panic!("expected runs, got {other:?}"),
                }
                assert!(matches!(&term.lines[2], TermLine::Text(r) if r.is_empty()));
                assert!(matches!(
                    &term.lines[3],
                    TermLine::Field { name, secret, .. } if name == "cmd" && !secret
                ));
                assert!(matches!(
                    &term.lines[4],
                    TermLine::Field { name, secret, .. } if name == "pw" && *secret
                ));
            }
            other => panic!("expected a term block, got {other:?}"),
        }
    }

    #[test]
    fn a_box_carries_its_class_and_its_children() {
        let src = "[[box class=card pad=16]]\n[[h3?]]\n正文\n[[end]]\n";
        match only(&parse(&src.replace("[[h3?]]", "# 标题"))) {
            Block::Box { style, children } => {
                assert_eq!(style.class.as_deref(), Some("card"));
                assert_eq!(children.len(), 2);
                assert!(matches!(children[0], Block::Heading(1, _)));
            }
            other => panic!("expected a box, got {other:?}"),
        }
    }

    #[test]
    fn an_avatar_only_needs_a_name() {
        let plain = parse("[[avatar alice]]\n");
        let sized = parse("[[avatar alice 72]]\n");
        let (one, two) = (only(&plain), only(&sized));
        match (one, two) {
            (
                Block::Avatar { name, size },
                Block::Avatar {
                    name: name2,
                    size: size2,
                },
            ) => {
                assert_eq!(name, "alice");
                assert_eq!(name2, "alice");
                assert_eq!(*size, 40.0);
                assert_eq!(*size2, 72.0);
            }
            other => panic!("expected two avatars, got {other:?}"),
        }
    }

    #[test]
    fn a_sheet_rule_is_one_class_name_and_nothing_else() {
        let src = concat!(
            "[[sheet]]\n",
            ".card { radius: 16; }\n",
            "card { radius: 8; }\n",
            ".a .b { radius: 4; }\n",
            ".a > .b { radius: 2; }\n",
            "@media (max-width: 600px) { radius: 1; }\n",
            ".a:hover { radius: 0; }\n",
            "[[end]]\n"
        );
        match only(&parse(src)) {
            Block::Sheet(rules) => {
                let names: Vec<&str> = rules.iter().map(|(n, _)| n.as_str()).collect();
                // `.card` and `card` are the same class; the second rule wins.
                assert_eq!(names, vec!["card"]);
                assert_eq!(rules[0].1.radius, Some(8.0));
            }
            other => panic!("expected a sheet, got {other:?}"),
        }
    }

    #[test]
    fn a_sheet_turns_rules_into_classes() {
        let src = "[[sheet]]\n.card { bg: #fff; border: 1:#ddd; }\n[[end]]\n";
        match only(&parse(src)) {
            Block::Sheet(rules) => {
                assert_eq!(rules.len(), 1);
                assert_eq!(rules[0].0, "card");
                assert_eq!(rules[0].1.border.map(|b| b.0), Some(1.0));
            }
            other => panic!("expected a sheet, got {other:?}"),
        }
    }

    #[test]
    fn a_paragraph_may_start_with_a_link() {
        // The bug this guards: `[[path:/a|甲]]  其余文字` used to parse as a lone
        // link, silently dropping everything after the bracket.
        let src = "[[path:/profile/demo|demo]]  ·  2026-10-26  ·  ♥ 3\n";
        match only(&parse(src)) {
            Block::Para(parts) => {
                assert_eq!(parts.len(), 2);
                assert!(matches!(&parts[0], Inline::Link { .. }));
                assert!(matches!(&parts[1], Inline::Text(t) if t.contains("2026")));
            }
            other => panic!("expected a paragraph, got {other:?}"),
        }

        // A line that is nothing but a link still becomes a whole-line link.
        match only(&parse("[[path:/a|甲]]\n")) {
            Block::Link { .. } => {}
            other => panic!("expected a block link, got {other:?}"),
        }
    }

    #[test]
    fn an_attachment_keeps_its_name_size_and_spaces() {
        match only(&parse("!file 9|钥匙 (最终版).zip|2048\n")) {
            Block::File { id, name, size } => {
                assert_eq!(*id, 9);
                assert_eq!(name, "钥匙 (最终版).zip");
                assert_eq!(*size, 2048);
            }
            other => panic!("expected a file block, got {other:?}"),
        }
        // 名字缺了就是坏的标记，不画空按钮。
        assert!(parse("!file 9||10\n").is_empty());
    }

    #[test]
    fn a_bare_terminal_has_no_header_and_keeps_a_trailing_bracket() {        let src = "[[term bare||0]]\n[[line out|ends with ]] ]]\n[[end]]\n";
        match only(&parse(src)) {
            Block::Term(term) => {
                assert!(term.bare);
                assert_eq!(term.target, None);
                match &term.lines[0] {
                    TermLine::Text(runs) => {
                        assert_eq!(runs[0].1.trim_end(), "ends with ]]");
                    }
                    other => panic!("expected runs, got {other:?}"),
                }
            }
            other => panic!("expected a term block, got {other:?}"),
        }
    }

    #[test]
    fn scenes_are_bars_with_keyframes() {
        let src = "[[scene 640x120 2.5]]\n\
                   [[bar 6b7280 8 90]]\n\
                   [[at 0 100 30]]\n\
                   [[bar #c0392b 40 12]]\n\
                   [[at 0 60 78]]\n\
                   [[at 1 60 6]]\n\
                   [[at 2 300 78]]\n\
                   [[end]]\n";
        let blocks = parse(src);
        match &blocks[0] {
            Block::Scene { width, height, seconds, bars } => {
                assert_eq!((*width, *height), (640.0, 120.0));
                assert_eq!(*seconds, 2.5);
                assert_eq!(bars.len(), 2);
                assert_eq!(bars[0].color, [0x6b, 0x72, 0x80]);
                assert_eq!(bars[1].color, [0xc0, 0x39, 0x2b]);
                assert_eq!(bars[1].keys.len(), 3);
                assert_eq!(bars[1].keys[1], (1.0, 60.0, 6.0));
            }
            other => panic!("期望场景，得到 {other:?}"),
        }

        // Keyframes are sorted, and a bar with no keys is still drawable (it just
        // never moves), so a page can lay out a static backdrop.
        let unsorted = parse("[[scene 10x10 1]]\n[[bar 000000 1 1]]\n[[at 2 0 0]]\n[[at 1 5 5]]\n[[end]]\n");
        match &unsorted[0] {
            Block::Scene { bars, .. } => {
                assert_eq!(bars[0].keys[0].0, 1.0);
                assert_eq!(bars[0].keys[1].0, 2.0);
            }
            other => panic!("期望场景，得到 {other:?}"),
        }
        // A bad size or colour must not take the page down: the block is simply
        // not a scene, and the lines fall through as ordinary text.
        for bad in [
            "[[scene 0x10 1]]\n[[bar 000000 1 1]]\n[[end]]\n",
            "[[scene 10x10 1]]\n[[bar nothex 1 1]]\n[[end]]\n",
            "[[scene 10x10 1]]\n[[end]]\n",
        ] {
            let blocks = parse(bad);
            assert!(
                !blocks.iter().any(|b| matches!(b, Block::Scene { .. })),
                "{bad:?} should not be a scene"
            );
        }
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
            .any(|b| matches!(b, Block::List { items, .. } if items.len() == 2)));
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
        // Markdown 的强调规则：单星号斜体、双星号粗体。
        let parts = inlines("a *b* c");
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1], Inline::Italic("b".into()));

        let bold = inlines("a **b** c");
        assert_eq!(bold.len(), 3);
        assert_eq!(bold[1], Inline::Bold("b".into()));

        // 粗体里还能套代码：`**` 先闭合，不会吃掉中间的反引号。
        let mixed = inlines("**a `b` c**");
        assert_eq!(mixed.len(), 1);
        assert_eq!(mixed[0], Inline::Bold("a `b` c".into()));
    }

    #[test]
    fn markdown_shorthand_is_understood() {
        // 一到六级标题都认；样式只分三级，但解析不能漏。
        let blocks = parse("#### 四级\n");
        assert!(matches!(blocks[0], Block::Heading(4, _)));

        // 三种圆点都当无序列表。
        for src in ["- 甲\n- 乙\n", "* 甲\n* 乙\n", "+ 甲\n+ 乙\n"] {
            match &parse(src)[0] {
                Block::List { items, .. } => assert_eq!(items.len(), 2, "{src}"),
                other => panic!("期望列表，得到 {other:?}（{src}）"),
            }
        }

        // `* 斜体 *` 在段首不会被当成圆点。
        assert!(matches!(parse("*不是列表*\n")[0], Block::Para(_)));
    }

    #[test]
    fn a_fence_becomes_a_code_block_with_its_language() {
        let src = "```python\nprint(1)\nprint(2)\n```\n";
        match only(&parse(src)) {
            Block::Pre { lang, text } => {
                assert_eq!(lang.as_deref(), Some("python"));
                assert_eq!(text, "print(1)\nprint(2)");
            }
            other => panic!("期望代码块，得到 {other:?}"),
        }
        // 没写语言也行，后面的文字不会被吞。
        let plain = parse("```\nx\n```\n\n后面一段\n");
        assert!(matches!(plain[0], Block::Pre { lang: None, .. }));
        assert!(
            plain.iter().any(|b| matches!(b, Block::Para(_))),
            "代码块后面的文字要留下来：{plain:?}"
        );
    }

    #[test]
    fn markdown_links_become_links() {
        // 站内路径
        assert_eq!(
            inlines("[首页](/)"),
            vec![Inline::Link {
                target: LinkTarget::Path("/".into()),
                text: "首页".into()
            }]
        );
        // 站外
        assert!(matches!(
            &inlines("[外站](https://example.com)")[0],
            Inline::Link { target: LinkTarget::Url(u), .. } if u == "https://example.com"
        ));
        // 页号
        assert!(matches!(
            &inlines("[某页](31345)")[0],
            Inline::Link { target: LinkTarget::Page(31345), .. }
        ));
        // 不认的协议退化成文字，不会变成可点的链接
        let hostile = inlines("[点我](javascript:alert(1))");
        assert!(hostile.iter().all(|p| !matches!(p, Inline::Link { .. })));

        // 句子中间的链接不影响前后文字
        let mixed = inlines("看 [这里](/notes) 就行");
        assert_eq!(mixed.len(), 3);
        assert!(matches!(mixed[0], Inline::Text(_)));
        assert!(matches!(mixed[1], Inline::Link { .. }));
        assert!(matches!(mixed[2], Inline::Text(_)));
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
            .any(|b| matches!(b, Block::List { ordered: true, items } if items.len() == 2)));
        assert!(blocks.iter().any(|b| matches!(b, Block::Hr)));
    }

    #[test]
    fn table_takes_the_first_row_as_header() {
        let src = "[[table]]\n| 名字 | 说明 |\n| 甲 | **第一** |\n| 乙 | 第二 |\n[[end]]\n";
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
    fn table_cells_can_hold_inline_links() {
        let src = "[[table]]\n| 页面 | 路径 |\n| [[path:/notes/|笔记]] | `/notes/` |\n[[end]]\n";
        match only(&parse(src)) {
            Block::Table { rows, .. } => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].len(), 2);
                assert!(matches!(
                    &rows[0][0][0],
                    Inline::Link { target: LinkTarget::Path(path), text } if path == "/notes/" && text == "笔记"
                ));
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
                assert!(body.iter().any(|b| matches!(b, Block::List { .. })));
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
        let src = "[[textarea body|正文]]\n[[check agree|同意]]\n";
        let blocks = parse(src);
        assert!(blocks.iter().any(
            |b| matches!(b, Block::Textarea { name, label } if name == "body" && label == "正文")
        ));
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
