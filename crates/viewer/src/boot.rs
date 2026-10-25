//! 启动：解析命令行参数、拼站点表、`--selftest` 自检。

use crate::*;


pub(crate) fn report_panics() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static SHOWN: AtomicBool = AtomicBool::new(false);

    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let text = format!("{info}");
        eprintln!("{text}");
        if SHOWN.swap(true, Ordering::SeqCst) {
            previous(info);
            return;
        }
        let path = std::env::temp_dir().join("nac-viewer-crash.txt");
        let _ = std::fs::write(&path, &text);
        let message = format!(
            "Nac 客户端遇到了内部错误，无法继续。\n\n{text}\n\n详细信息已写入：\n{}",
            path.display()
        );
        alert(&message);
        previous(info);
    }));
}

/// A modal box, so the message is visible even with no console attached.
#[cfg(target_os = "windows")]
pub(crate) fn alert(text: &str) {
    use std::os::windows::ffi::OsStrExt;
    use std::ptr;

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            owner: *mut std::ffi::c_void,
            text: *const u16,
            caption: *const u16,
            kind: u32,
        ) -> i32;
    }

    let wide = |s: &str| -> Vec<u16> {
        std::ffi::OsStr::new(s)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    };
    let body = wide(text);
    let title = wide("Nac");
    const MB_ICONERROR: u32 = 0x10;
    // SAFETY: both strings are NUL-terminated and outlive the call.
    unsafe {
        MessageBoxW(
            ptr::null_mut(),
            body.as_ptr(),
            title.as_ptr(),
            MB_ICONERROR,
        );
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn alert(text: &str) {
    eprintln!("{text}");
}

/// What clicking a tab did.

pub(crate) fn run() -> eframe::Result<()> {
    report_panics();

    let mut addr: Option<String> = std::env::var("NAC_ADDR").ok();
    let mut routes_path: Option<String> = None;
    let mut site_name: Option<String> = None;
    let mut selftest = false;
    let mut encode: Option<String> = None;
    let mut list_sites = false;
    let mut start_page: Option<u16> = None;
    let mut start_path: Option<String> = None;
    let mut plaintext = false;

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--addr" => {
                if let Some(v) = args.get(i + 1) {
                    addr = Some(v.clone());
                    i += 1;
                }
            }
            "--routes" => {
                if let Some(v) = args.get(i + 1) {
                    routes_path = Some(v.clone());
                    i += 1;
                }
            }
            "--site" => {
                if let Some(v) = args.get(i + 1) {
                    site_name = Some(v.clone());
                    i += 1;
                }
            }
            "--routes-encode" => {
                if let Some(v) = args.get(i + 1) {
                    encode = Some(v.clone());
                    i += 1;
                }
            }
            "--routes-check" => list_sites = true,
            "--page" => {
                if let Some(v) = args.get(i + 1) {
                    start_page = v.parse::<u16>().ok();
                    i += 1;
                }
            }
            "--selftest" => selftest = true,
            // `--at /hn/item/12`: open a deep address, which a page number alone
            // cannot express once one page answers a whole family of addresses.
            "--at" => {
                if let Some(v) = args.get(i + 1) {
                    start_path = Some(v.clone());
                    i += 1;
                }
            }
            "--plaintext" => plaintext = true,
            _ => {}
        }
        i += 1;
    }

    if plaintext {
        tls::disable_plaintext();
    }

    if let Some(value) = encode {
        println!("{}", routes::encode(&value));
        return Ok(());
    }

    let (routes, from_embedded) = build_routes(routes_path.as_deref(), addr);
    // The SNI name is cosmetic here (the certificate is pinned, not name-checked),
    // so the default site's name is good enough.
    tls::set_server_name(
        routes
            .default_site()
            .map(|site| site.name.as_str())
            .unwrap_or(routes::DEFAULT_DOMAIN),
    );
    // A site marked `tls` cannot work without a pin, and the failure would
    // otherwise show up only as "cannot connect". Say it once, up front.
    if !plaintext && !tls::has_pin() {
        let forced: Vec<&str> = routes
            .sites
            .iter()
            .filter(|s| s.tls == routes::TlsMode::On)
            .map(|s| s.name.as_str())
            .collect();
        if !forced.is_empty() {
            eprintln!(
                "注意：站点 {} 要求 TLS，但这份构建没有证书 pin（NAC_TLS_PIN）——这些站点会连不上。",
                forced.join(", ")
            );
        }
    }

    if list_sites {
        for site in &routes.sites {
            let mode = match site.tls {
                routes::TlsMode::Auto => "auto",
                routes::TlsMode::On => "tls",
                routes::TlsMode::Off => "plain",
            };
            let tunnel = match &site.ssh {
                Some(dest) if cfg!(feature = "admin") => format!("\tssh={dest}"),
                Some(_) => "\tssh（本构建不支持）".to_string(),
                None => String::new(),
            };
            if from_embedded {
                println!(
                    "{}{}\t{}{}",
                    if site.default { "*" } else { " " },
                    site.name,
                    mode,
                    tunnel
                );
            } else {
                println!(
                    "{}{}\t{}\t{}{}",
                    if site.default { "*" } else { " " },
                    site.name,
                    site.endpoint,
                    mode,
                    tunnel
                );
            }
        }
        return Ok(());
    }

    let site = match site_name {
        Some(name) if routes.get(&name).is_some() => name,
        Some(name) => {
            eprintln!("没有名为 {name} 的站点");
            return Ok(());
        }
        None => routes
            .default_site()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "dev".to_string()),
    };

    if selftest {
        return selftest_run(&routes, &site);
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 760.0])
            .with_min_inner_size([560.0, 420.0])
            .with_title("nac"),
        ..Default::default()
    };

    eframe::run_native(
        "nac",
        options,
        Box::new(move |cc| {
            Ok(Box::new(Viewer::new(
                cc,
                routes,
                site,
                start_path.map(Target::Path).or(start_page.map(Target::Page)),
            )))
        }),
    )
}

/// Resolution order: `--addr`, then an explicit `--routes <path>`, then an
/// endpoint embedded at build time via `NAC_EMBED_ROUTES`, then whatever
/// directory the usual lookup finds. The explicit flag has to win over the
/// embedded copy, otherwise a standalone build silently ignores it.
///
/// The second value says whether the result came from the embedded copy, which
/// is what decides if endpoints are printed: a standalone build should not
/// print them, but an operator who passed `--routes` asked for the file.
pub(crate) fn build_routes(explicit: Option<&str>, addr: Option<String>) -> (routes::Routes, bool) {
    if let Some(fallback) = addr {
        return (routes::Routes::single("dev", &fallback), false);
    }
    if let Some(path) = explicit {
        match routes::Routes::load(std::path::Path::new(path)) {
            Some(routes) => return (routes, false),
            None => eprintln!("无法读取站点目录 {path}"),
        }
    }
    if let Some(routes) = routes::Routes::embedded() {
        return (routes, true);
    }
    if let Some(path) = routes::Routes::locate(None) {
        if let Some(routes) = routes::Routes::load(&path) {
            return (routes, false);
        }
        eprintln!("无法读取站点目录 {}", path.display());
    }
    (routes::Routes::default(), false)
}

pub(crate) fn selftest_run(routes: &routes::Routes, site_name: &str) -> eframe::Result<()> {
    let Some(site) = routes.get(site_name) else {
        println!("site {site_name}: missing");
        return Ok(());
    };
    // Keep the tunnel alive for the whole run; dropping it kills `ssh`.
    #[cfg(feature = "admin")]
    let _tunnel = match tunnel::Tunnel::ensure(site) {
        Ok(tunnel) => tunnel,
        Err(message) => {
            println!("tunnel: {message}");
            return Ok(());
        }
    };
    let mut client = net::Client::with_tls(site.endpoint.clone(), site.tls);
    match client.hello() {
        Ok((version, name)) => println!("protocol: {version}  site: {name}"),
        Err(()) => println!("protocol: legacy"),
    }
    match client.manifest() {
        Ok(entries) => {
            println!("site {site_name}: {} entries", entries.len());
            for e in &entries {
                println!(
                    "  {}\t{}\t{:016x}\t{}\t{}",
                    e.id,
                    if e.dynamic { "dyn " } else { "stat" },
                    e.hash,
                    e.path,
                    e.title
                );
            }
            let mut attachment: Option<(u16, String)> = None;
            if let Some(id) = entries.iter().map(|e| e.id).min() {
                match client.page(id, "") {
                    Ok(text) => {
                        let blocks = markup::parse(&text);
                        println!("page {id}: {} bytes, {} blocks", text.len(), blocks.len());
                        attachment = render::first_file(&blocks);
                        print!("{text}");
                    }
                    Err(_) => println!("page {id}: EMPTY"),
                }
            }
            match client.image(1) {
                Ok(bytes) => println!("image 1: {} bytes", bytes.len()),
                Err(_) => println!("image 1: EMPTY"),
            }
            // 附件整条路：取字节、净化名字、写进一个临时目录。自检不该往
            // 读者的下载目录里丢东西，所以落点是固定的。
            if let Some((file_id, name)) = attachment {
                match client.image(file_id) {
                    Ok(bytes) => {
                        let dir = std::env::temp_dir().join("nac-selftest");
                        match save::write_to(&dir, &name, &bytes) {
                            Ok(path) => println!("file {name}: {} bytes -> {path}", bytes.len()),
                            Err(why) => println!("file {name}: {why}"),
                        }
                    }
                    Err(_) => println!("file {name}: EMPTY"),
                }
            }
            match client.submit(7, vec![("name".to_string(), "selftest".to_string())], "") {
                Ok((id, text)) => println!("submit 7 -> page {id}: {} bytes", text.len()),
                Err(_) => println!("submit 7: EMPTY"),
            }
        }
        Err(_) => println!("site {site_name}: EMPTY"),
    }
    Ok(())
}

// ------------------------------------------------------------------- fonts
