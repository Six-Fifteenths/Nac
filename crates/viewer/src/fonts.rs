//! 字体：四个字族都编在二进制里。
//!
//! 系统字体是"这台机器碰巧装了什么"，同一页在别人的电脑上就变样；编进来之后，
//! 谁看到都是同一副字。两份字体都是 SIL OFL，许可原文在 assets/fonts/。

use crate::*;


pub(crate) const FONT_SANS: &[u8] = include_bytes!("../assets/fonts/TitilliumWeb-Regular.ttf");
pub(crate) const FONT_SANS_BOLD: &[u8] = include_bytes!("../assets/fonts/TitilliumWeb-Bold.ttf");
pub(crate) const FONT_SERIF: &[u8] = include_bytes!("../assets/fonts/PTSerif-Regular.ttf");
pub(crate) const FONT_SERIF_BOLD: &[u8] = include_bytes!("../assets/fonts/PTSerif-Bold.ttf");
/// What every box with a background or a border pads itself by.
pub(crate) const BOX_PAD: (f32, f32) = (14.0, 12.0);

pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    hash
}

/// The bold cut of a family name, so `**bold**` inside a serif paragraph stays serif.
pub(crate) fn bold_of(family: &FontFamily) -> FontFamily {
    match family {
        FontFamily::Name(name) => match &**name {
            "serif" | "serif-bold" => FontFamily::Name(Arc::from("serif-bold")),
            _ => FontFamily::Name(Arc::from("sans-bold")),
        },
        _ => FontFamily::Name(Arc::from("sans-bold")),
    }
}

pub(crate) fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    let cjk = load_first(&[
        "C:/Windows/Fonts/Deng.ttf",
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/simkai.ttf",
        "/System/Library/Fonts/PingFang.ttc",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-zenhei.ttc",
    ]);

    for (name, bytes) in [
        ("sans", FONT_SANS),
        ("sans-bold", FONT_SANS_BOLD),
        ("serif", FONT_SERIF),
        ("serif-bold", FONT_SERIF_BOLD),
    ] {
        fonts.font_data.insert(
            name.to_string(),
            Arc::new(egui::FontData::from_static(bytes)),
        );
    }

    let cjk_name = if let Some(bytes) = cjk {
        let name = "cjk".to_string();
        fonts
            .font_data
            .insert(name.clone(), Arc::new(egui::FontData::from_owned(bytes)));
        Some(name)
    } else {
        None
    };

    // Every family ends with the CJK face so Chinese never turns into boxes.
    let stack = |first: &str| {
        let mut list = vec![first.to_string()];
        if let Some(cjk) = &cjk_name {
            list.push(cjk.clone());
        }
        list
    };
    for name in ["sans", "sans-bold", "serif", "serif-bold"] {
        fonts.families.insert(
            egui::FontFamily::Name(Arc::from(name)),
            stack(name),
        );
    }
    // The document default becomes Titillium, with the CJK face behind it.
    fonts.families.insert(
        egui::FontFamily::Proportional,
        stack("sans"),
    );
    let mut mono = fonts
        .families
        .get(&egui::FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();
    if let Some(cjk) = &cjk_name {
        mono.push(cjk.clone());
    }
    fonts.families.insert(egui::FontFamily::Monospace, mono);
    // `bold` stays as the generic "bold" family every existing render path uses.
    fonts.families.insert(
        egui::FontFamily::Name(Arc::from("bold")),
        stack("sans-bold"),
    );

    ctx.set_fonts(fonts);
}

pub(crate) fn load_first(paths: &[&str]) -> Option<Vec<u8>> {
    paths
        .iter()
        .find_map(|p| std::fs::read(p).ok())
        .filter(|b| !b.is_empty())
}
