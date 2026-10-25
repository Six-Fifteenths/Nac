#![cfg_attr(
    all(target_os = "windows", feature = "windows_gui"),
    windows_subsystem = "windows"
)]

//! Nac 阅读器。
//!
//! 这个文件只留三件事：模块声明、共享的 `use`、以及测试。真正的代码按职责
//! 分在下面几个模块里 —— 每个模块都从 `use crate::*;` 拿到同一批名字，
//! 所以看起来跟原来在一个文件里没有区别。
//!
//! | 模块 | 管什么 |
//! |---|---|
//! | `boot` | 启动、命令行参数、`--selftest` |
//! | `fonts` | 编进二进制的字体 |
//! | `address` | 地址解析、站点解析、路径归一化 |
//! | `widgets` | 标签页方块、地址栏同步规则、终端配色 |
//! | `state` | `Tab` / `Viewer` 两个结构体 |
//! | `render` | 页面的排版与绘制 |
//! | `save` | 附件落地到哪里、文件名怎么变干净 |
//! | `app` | 窗口：标签栏、地址栏、状态栏、内容区 |

use std::collections::HashMap;
use std::sync::Arc;

use eframe::egui;
use egui::text::{LayoutJob, TextFormat};
use egui::{Align, Color32, FontFamily, FontId, Layout, RichText};

mod address;
mod app;
mod blocks;
mod boot;
mod fetch;
mod fonts;
mod loader;
mod markup;
mod net;
mod render;
mod routes;
mod save;
mod state;
mod style;
mod tab;
mod tls;
#[cfg(feature = "admin")]
mod tunnel;
mod widgets;

use address::{
    normalize_path, open_in_browser, parse_address, resolve_address, stash_fields, Address, Place,
    Resolved, Target, Trail,
};
use fonts::{bold_of, fnv1a, install_fonts, BOX_PAD};
use markup::{Block, Inline, LinkTarget, NoteKind};
use state::{SaveState, Tab, Viewer};
use style::{Style, Text};
use widgets::{
    should_sync_address, strip_line, tab_chip, unescape_lines, TabHit, TERM_ASCII, TERM_BG,
    TERM_BORDER, TERM_CMD, TERM_FG, TERM_HOST, TERM_INPUT, TERM_LINE, TERM_LINK, TERM_PROMPT,
    TERM_USER,
};

fn main() -> eframe::Result<()> {
    boot::run()
}

#[cfg(test)]
mod tests {
    use crate::address::{looks_like_host, resolve_path};
    use crate::state::active_after_close;
    use crate::widgets::shorten;
    use super::{
        net, normalize_path, parse_address, resolve_address, routes, should_sync_address,
        stash_fields, unescape_lines,
        Address, Place, Trail,
    };
    use std::collections::HashMap;

    #[test]
    fn fields_survive_a_page_switch() {
        let mut saved: HashMap<u16, HashMap<String, String>> = HashMap::new();
        let mut typed = HashMap::new();
        typed.insert("name".to_string(), "阿离".to_string());

        stash_fields(&mut saved, Some(7), &typed);
        assert_eq!(saved.len(), 1);
        assert_eq!(
            saved.remove(&7).unwrap().get("name").map(String::as_str),
            Some("阿离")
        );
        // Nothing to stash when the form is empty or there is no page yet.
        stash_fields(&mut saved, Some(7), &HashMap::new());
        stash_fields(&mut saved, None, &typed);
        assert!(saved.is_empty());
    }

    fn sites() -> routes::Routes {
        routes::Routes::parse(
            "*nac.local 127.0.0.1:7000\niris.out 127.0.0.1:7002\nEX_MYTH 127.0.0.1:7003\n",
        )
    }

    #[test]
    fn page_paths_need_no_host() {
        let r = sites();
        assert_eq!(parse_address("/7", &r, "home"), known("home", "/7"));
        assert_eq!(parse_address("7", &r, "home"), known("home", "/7"));
        assert_eq!(parse_address("/", &r, "home"), known("home", "/"));
        assert_eq!(
            parse_address("  /about/  ", &r, "home"),
            known("home", "/about/")
        );
        assert_eq!(parse_address("about", &r, "home"), known("home", "/about"));
        assert_eq!(parse_address("/a//b/.", &r, "home"), known("home", "/a/b"));
    }

    fn entry(id: u16, path: &str) -> net::Entry {
        net::Entry {
            id,
            dynamic: false,
            hash: 0,
            title: String::new(),
            path: path.to_string(),
            matches: None,
        }
    }

    fn dynamic_entry(id: u16, path: &str, matches: &str) -> net::Entry {
        net::Entry {
            id,
            dynamic: true,
            hash: 0,
            title: String::new(),
            path: path.to_string(),
            matches: Some(matches.to_string()),
        }
    }

    #[test]
    fn paths_resolve_directories_and_404() {
        let entries = vec![entry(1, "/"), entry(2, "/notes/"), entry(3, "/notes/a")];
        assert_eq!(resolve_path(&entries, Some(1), "/"), Some(1));
        assert_eq!(resolve_path(&entries, Some(1), "/notes"), Some(2));
        assert_eq!(resolve_path(&entries, Some(1), "/notes/"), Some(2));
        assert_eq!(resolve_path(&entries, Some(1), "/notes/a"), Some(3));
        assert_eq!(resolve_path(&entries, Some(1), "/missing"), None);
        assert_eq!(normalize_path("//a/./b/").as_deref(), Some("/a/b/"));
    }

    fn known(site: &str, path: &str) -> Option<Address> {
        Some(Address::Known {
            site: site.to_string(),
            path: path.to_string(),
        })
    }

    fn unknown(site: &str, path: &str) -> Option<Address> {
        Some(Address::Unknown {
            site: site.to_string(),
            path: path.to_string(),
        })
    }

    #[test]
    fn hosts_select_the_site_and_path() {
        let r = sites();
        assert_eq!(
            parse_address("nac.local/", &r, "nac.local"),
            known("nac.local", "/")
        );
        assert_eq!(
            parse_address("nac.local/about", &r, "nac.local"),
            known("nac.local", "/about")
        );
        assert_eq!(
            parse_address("iris.out/notes", &r, "nac.local"),
            known("iris.out", "/notes")
        );
        assert_eq!(
            parse_address("EX_MYTH/3", &r, "nac.local"),
            known("EX_MYTH", "/3")
        );
    }

    #[test]
    fn named_sites_can_be_switched_by_name() {
        let r = sites();
        assert_eq!(
            parse_address("iris.out/3", &r, "nac.local"),
            known("iris.out", "/3")
        );
        assert_eq!(
            parse_address("nac://EX_MYTH/3", &r, "nac.local"),
            known("EX_MYTH", "/3")
        );
        assert_eq!(
            parse_address("iris.out", &r, "nac.local"),
            known("iris.out", "/")
        );
    }

    #[test]
    fn a_name_the_table_lacks_is_resolved_not_rejected() {
        let r = sites();
        // The whole name is kept: there is no suffix to strip.
        assert_eq!(
            parse_address("shop.nac.local/hats", &r, "nac.local"),
            unknown("shop.nac.local", "/hats")
        );
        assert_eq!(
            parse_address("NOWHERE.out/", &r, "nac.local"),
            unknown("NOWHERE.out", "/")
        );
    }

    #[test]
    fn a_learned_site_becomes_a_known_one() {
        let mut r = sites();
        assert_eq!(
            parse_address("iris.out/", &r, "nac.local"),
            known("iris.out", "/")
        );
        r.learn("nowhere.out", "10.0.0.5:7100", routes::TlsMode::On);
        assert_eq!(
            parse_address("nowhere.out/notes", &r, "nac.local"),
            known("nowhere.out", "/notes")
        );
        assert_eq!(
            parse_address("NOWHERE.OUT/notes", &r, "nac.local"),
            known("nowhere.out", "/notes")
        );
        assert_eq!(
            r.get("nowhere.out").map(|s| s.endpoint.as_str()),
            Some("10.0.0.5:7100")
        );
        assert_eq!(
            r.get("nowhere.out").map(|s| s.tls),
            Some(routes::TlsMode::On)
        );
    }

    #[test]
    fn back_crosses_a_site_boundary() {
        let mut trail = Trail::default();
        let home = Place {
            site: "home".into(),
            path: "/".into(),
        };
        let shop = Place {
            site: "shop".into(),
            path: "/hats".into(),
        };
        assert!(!trail.can_back());

        // Reading home, then following a link that lands on another site.
        trail.record(Some(home.clone()));
        assert!(trail.can_back());
        assert!(!trail.can_forward());

        assert_eq!(trail.go_back(Some(shop.clone())), Some(home.clone()));
        assert!(!trail.can_back());
        assert!(trail.can_forward());

        assert_eq!(trail.go_forward(Some(home.clone())), Some(shop));
    }

    #[test]
    fn a_new_step_drops_the_forward_branch() {
        let mut trail = Trail::default();
        let one = Place {
            site: "home".into(),
            path: "/one".into(),
        };
        let two = Place {
            site: "home".into(),
            path: "/two".into(),
        };
        let other = Place {
            site: "notes".into(),
            path: "/".into(),
        };
        trail.record(Some(one.clone()));
        assert_eq!(trail.go_back(Some(two.clone())), Some(one.clone()));
        assert!(trail.can_forward());

        // Going somewhere new makes the old "forward" unreachable.
        trail.record(Some(two.clone()));
        assert!(!trail.can_forward());

        // and the stack now holds `one` then `two`.
        assert_eq!(trail.go_back(Some(other)), Some(two));
        assert!(!trail.can_back());
    }

    #[test]
    fn a_name_is_whatever_the_file_calls_it() {
        // No suffix and no sub-domain rule: `iris.out` and `EX_MYTH` are just
        // names, the same way `nac.local` is.
        let r = sites();
        assert_eq!(parse_address("nac.local/", &r, "nac.local"), known("nac.local", "/"));
        assert_eq!(
            parse_address("iris.out/notes", &r, "nac.local"),
            known("iris.out", "/notes")
        );
        assert_eq!(
            parse_address("EX_MYTH/", &r, "nac.local"),
            known("EX_MYTH", "/")
        );
        // Case is not something a reader should have to remember.
        assert_eq!(
            parse_address("ex_myth/", &r, "nac.local"),
            known("EX_MYTH", "/")
        );
        // A name the table has not seen is looked up, not rejected.
        assert_eq!(
            parse_address("somewhere.else/", &r, "nac.local"),
            unknown("somewhere.else", "/")
        );
    }

    #[test]
    fn a_bare_word_is_a_path_and_a_host_shaped_one_is_not() {
        let r = sites();
        // No dot and no underscore: a path on the current site.
        assert_eq!(
            parse_address("about", &r, "nac.local"),
            known("nac.local", "/about")
        );
        assert_eq!(parse_address("7", &r, "nac.local"), known("nac.local", "/7"));
        // Names may use dots and underscores; spaces and colons are not names.
        assert!(looks_like_host("EX_MYTH"));
        assert!(looks_like_host("iris.out"));
        assert!(!looks_like_host("two words"));
        assert!(!looks_like_host(""));
        assert!(!looks_like_host("about"));
        assert!(!looks_like_host("127.0.0.1:7000"));
    }

    #[test]
    fn prefilled_text_gets_its_newlines_back() {
        // One markup line cannot hold a newline, so pages send `\n`.
        assert_eq!(unescape_lines("a\\n\\nb"), "a\n\nb");
        assert_eq!(unescape_lines("no escapes"), "no escapes");
        // A real backslash is doubled by the page, and must come back singly.
        assert_eq!(unescape_lines("50\\\\%"), "50\\%");
    }

    #[test]
    fn typing_is_never_overwritten_by_the_tab() {
        // The bug this guards: every frame the model won and the caret jumped
        // back, so keystrokes appeared to vanish.
        assert!(!should_sync_address(true, false, true, "nac.l", "nac.local/"));
        assert!(!should_sync_address(true, true, false, "", "nac.local/"));
        // A resolved address changes the tab under the reader's cursor; if they
        // have not touched the bar, it should follow.
        assert!(should_sync_address(true, false, false, "nac.local/", "iris.out/"));
        // Just committed an address: keep showing it while it loads, focused or
        // not -- otherwise the bar snaps back to the old address.
        assert!(!should_sync_address(false, true, false, "iris.out", "nac.local/"));
        assert!(!should_sync_address(true, true, true, "iris.out", "nac.local/"));
        // Idle and out of date: follow the tab.
        assert!(should_sync_address(false, false, false, "nac.local/", "iris.out/"));
        // Idle and already right: do nothing (no needless caret resets).
        assert!(!should_sync_address(false, false, false, "iris.out/", "iris.out/"));
    }

    #[test]
    fn a_long_title_is_cut_by_characters_not_bytes() {
        assert_eq!(shorten("short", 20), "short");
        assert_eq!(shorten("a".repeat(20).as_str(), 20), "a".repeat(20));
        assert_eq!(shorten(&"a".repeat(21), 20).chars().count(), 20);
        assert!(shorten(&"a".repeat(21), 20).ends_with('…'));
        // 20 Chinese characters are 60 bytes; cutting by bytes would panic.
        let chinese = "汉".repeat(30);
        let cut = shorten(&chinese, 20);
        assert_eq!(cut.chars().count(), 20);
        assert!(cut.ends_with('…'));
    }

    #[test]
    fn closing_a_tab_keeps_the_same_one_active() {
        // Three tabs, looking at the middle one, close the first: the tab that
        // was in the middle is now index 0 and must stay selected.
        assert_eq!(active_after_close(3, 1, 0), 0);
        // Close the active one: fall to whatever took its place.
        assert_eq!(active_after_close(3, 1, 1), 1);
        // Close the last while standing on it: step back.
        assert_eq!(active_after_close(3, 2, 2), 1);
        // Close something to the right: no change.
        assert_eq!(active_after_close(3, 0, 2), 0);
        // Two tabs, close the other one.
        assert_eq!(active_after_close(2, 1, 0), 0);
    }

    #[test]
    fn a_prefix_page_answers_the_whole_family() {
        let entries = vec![
            entry(1, "/"),
            dynamic_entry(2, "/item/", "/item/"),
            dynamic_entry(3, "/item/special/", "/item/special/"),
        ];
        // Exact match keeps naming the page's own address.
        assert_eq!(
            resolve_address(&entries, Some(1), "/item/").map(|r| (r.id, r.path)),
            Some((2, "/item/".to_string()))
        );
        // Anything under the prefix goes to the same page, and the page is told
        // which address was asked for.
        assert_eq!(
            resolve_address(&entries, Some(1), "/item/123").map(|r| (r.id, r.path)),
            Some((2, "/item/123".to_string()))
        );
        // The longest declared prefix wins, so a more specific page can exist.
        assert_eq!(
            resolve_address(&entries, Some(1), "/item/special/9").map(|r| r.id),
            Some(3)
        );
        // No prefix matches: still a 404.
        assert_eq!(resolve_address(&entries, Some(1), "/nope"), None);
    }

    #[test]
    fn endpoints_are_not_addresses() {
        let r = sites();
        // An endpoint is not something a reader should ever paste, so it is
        // refused instead of becoming a 404 on the current site.
        assert_eq!(parse_address("127.0.0.1:7000/1", &r, "nac.local"), None);
        assert_eq!(parse_address("[::1]:7000/", &r, "nac.local"), None);
        // A dot does not make a name wrong — it makes it worth looking up.
        assert_eq!(
            parse_address("example.test", &r, "nac.local"),
            unknown("example.test", "/")
        );
        assert_eq!(parse_address("/../secret", &r, "nac.local"), None);
    }
}
