//! 附件落地：选目录、净化文件名、绝不覆盖已有文件。
//!
//! 页面能送来名字和字节，别的什么都不能定：路径由这里决定，`../` 之类在
//! `safe_name` 里就被削掉了。宁可存成一个陌生的名字，也不要让一页纸决定
//! 往磁盘的哪里写字。

use std::path::{Path, PathBuf};

/// 附件落在哪里：系统的下载目录。没有下载目录就退到主目录，再不行退到临时
/// 目录 —— 总比什么都不做、只报一句「失败」要好。
pub fn dir() -> PathBuf {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    match home {
        Some(home) => {
            let home = PathBuf::from(home);
            let downloads = home.join("Downloads");
            if downloads.is_dir() {
                downloads
            } else {
                home
            }
        }
        None => std::env::temp_dir(),
    }
}

/// 把一附件写到下载目录，返回落地的完整路径。
pub fn write(name: &str, bytes: &[u8]) -> Result<String, String> {
    write_to(&dir(), name, bytes)
}

/// 真正干活的那一半：目录由调用者给，所以这一半可以单独测。
pub fn write_to(dir: &Path, name: &str, bytes: &[u8]) -> Result<String, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("建不了目录 {e}"))?;
    let path = unique(dir, &safe_name(name));
    std::fs::write(&path, bytes).map_err(|e| format!("写不进去 {e}"))?;
    Ok(path.display().to_string())
}

/// 只留下一个文件名。分隔符、控制字符、开头的点一律削掉，Windows 的保留名
/// 加个下划线绕开。
pub fn safe_name(name: &str) -> String {
    let base = name
        .rsplit(|c| c == '/' || c == '\\')
        .next()
        .unwrap_or("")
        .trim();
    let cleaned: String = base
        .chars()
        .filter(|c| !c.is_control() && *c != '|' && *c != '"' && *c != '<' && *c != '>')
        .collect();
    let trimmed = cleaned.trim_matches(|c| c == '.' || c == ' ');
    if trimmed.is_empty() {
        return "nac-file".to_string();
    }
    let stem = trimmed
        .split('.')
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    const RESERVED: [&str; 22] = [
        "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7",
        "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
    ];
    if RESERVED.contains(&stem.as_str()) {
        format!("_{trimmed}")
    } else {
        trimmed.to_string()
    }
}

/// `名字.txt` → `名字 (2).txt`，直到不撞为止。
fn unique(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem.to_string(), format!(".{ext}")),
        _ => (name.to_string(), String::new()),
    };
    for n in 2..1000 {
        let candidate = dir.join(format!("{stem} ({n}){ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    // 1000 个重名：不猜了，让系统自己报错，也不要覆盖。
    dir.join(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_cannot_choose_where_a_file_lands() {
        assert_eq!(safe_name("../../etc/passwd"), "passwd");
        assert_eq!(safe_name("C:\\Windows\\evil.exe"), "evil.exe");
        assert_eq!(safe_name(".."), "nac-file");
        assert_eq!(safe_name("  "), "nac-file");
        assert_eq!(safe_name("a|b\nc.zip"), "abc.zip");
        assert_eq!(safe_name("钥匙.zip"), "钥匙.zip");
        assert_eq!(safe_name("con.txt"), "_con.txt");
        assert_eq!(safe_name("CON"), "_CON");
    }

    #[test]
    fn a_second_copy_does_not_overwrite_the_first() {
        let dir = std::env::temp_dir().join(format!(
            "nac-save-test-{}",
            std::process::id() as u64 * 1000 + std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos() as u64)
                .unwrap_or(0)
        ));
        let first = write_to(&dir, "钥匙.zip", b"one").unwrap();
        let second = write_to(&dir, "钥匙.zip", b"two").unwrap();
        assert_ne!(first, second);
        assert!(first.ends_with("钥匙.zip"));
        assert!(second.ends_with("钥匙 (2).zip"));
        assert_eq!(std::fs::read(&first).unwrap(), b"one");
        assert_eq!(std::fs::read(&second).unwrap(), b"two");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
