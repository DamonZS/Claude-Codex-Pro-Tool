//! 安全写入：改客户端文件前先比对磁盘现状。
//!
//! Codex / Claude 自己也会写这些文件。`settings::atomic_write` 只保证"要么旧内容要么新内容"，
//! 但如果我们读到 A、算完新内容时磁盘已经变成 B，再写就会把 B 覆盖掉。
//! 这里的入口在落盘前重读一次：只有磁盘内容和调用方读到的内容一致才写。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// 磁盘内容和调用方读到的内容已经不一致，本次写入没有发生。
#[derive(Debug)]
pub(crate) struct Conflict {
    pub path: PathBuf,
}

impl std::fmt::Display for Conflict {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} 在我们读取之后被其它程序改动，本次写入已放弃（磁盘内容未被覆盖），请重试。",
            self.path.display()
        )
    }
}

impl std::error::Error for Conflict {}

/// 判断错误是不是"文件被别人改了"，调用方据此决定能不能用旧字节回滚。
pub(crate) fn is_conflict(error: &anyhow::Error) -> bool {
    error.downcast_ref::<Conflict>().is_some()
}

/// 写入 `path`，但只在磁盘内容仍等于 `expected` 时执行。
///
/// `expected` 为 `None` 表示"调用方读到的是文件不存在"。
pub(crate) fn write_bytes_if_unchanged(
    path: &Path,
    bytes: &[u8],
    expected: Option<&[u8]>,
) -> Result<()> {
    let current = read_optional_bytes(path)?;
    let unchanged = match (current.as_deref(), expected) {
        (None, None) => true,
        (Some(current), Some(expected)) => current == expected,
        _ => false,
    };
    if !unchanged {
        return Err(anyhow::Error::new(Conflict {
            path: path.to_path_buf(),
        }));
    }
    crate::settings::atomic_write(path, bytes)?;
    cleanup_stale_temps(path);
    Ok(())
}

fn read_optional_bytes(path: &Path) -> Result<Option<Vec<u8>>> {
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("读取 {} 失败", path.display())),
    }
}

/// 清掉本模块和 `settings::atomic_write` 崩在落盘中途留下的临时文件。
fn cleanup_stale_temps(path: &Path) {
    let (Some(parent), Some(name)) = (path.parent(), path.file_name().and_then(|v| v.to_str()))
    else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    let prefix = format!("{name}.");
    for entry in entries.flatten() {
        let candidate = entry.file_name();
        let Some(candidate) = candidate.to_str() else {
            continue;
        };
        if !candidate.starts_with(&prefix) || !candidate.ends_with(".tmp") {
            continue;
        }
        let middle = &candidate[prefix.len()..candidate.len() - ".tmp".len()];
        let is_temp = middle
            .split('.')
            .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()));
        if is_temp {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("ccp-live-write-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("建临时目录");
        dir
    }

    #[test]
    fn writes_when_disk_still_matches_expected() {
        let dir = temp_dir("same");
        let path = dir.join("config.toml");
        std::fs::write(&path, b"old").unwrap();

        write_bytes_if_unchanged(&path, b"new", Some(b"old")).expect("应该写入");

        assert_eq!(std::fs::read(&path).unwrap(), b"new");
    }

    #[test]
    fn refuses_when_disk_changed_underneath_and_keeps_external_content() {
        let dir = temp_dir("conflict");
        let path = dir.join("config.toml");
        std::fs::write(&path, b"someone-else").unwrap();

        let error = write_bytes_if_unchanged(&path, b"mine", Some(b"what-i-read"))
            .expect_err("应该拒绝写入");
        let message = error.to_string();

        assert!(message.contains("被其它程序改动"), "实际报错：{message}");
        assert_eq!(std::fs::read(&path).unwrap(), b"someone-else");
    }

    #[test]
    fn treats_missing_file_as_expected_missing() {
        let dir = temp_dir("missing");
        let path = dir.join("config.toml");

        write_bytes_if_unchanged(&path, b"fresh", None).expect("文件不存在时应该写入");
        assert_eq!(std::fs::read(&path).unwrap(), b"fresh");

        let error =
            write_bytes_if_unchanged(&path, b"again", None).expect_err("文件已出现应该拒绝");
        assert!(error.to_string().contains("被其它程序改动"));
    }

    #[test]
    fn removes_orphan_temp_files_left_by_a_crash() {
        let dir = temp_dir("orphan");
        let path = dir.join("config.toml");
        std::fs::write(&path, b"old").unwrap();
        let orphan = dir.join("config.toml.999999.7.tmp");
        std::fs::write(&orphan, b"half-written").unwrap();
        let unrelated = dir.join("notes.tmp");
        std::fs::write(&unrelated, b"keep me").unwrap();

        write_bytes_if_unchanged(&path, b"new", Some(b"old")).expect("应该写入");

        assert!(!orphan.exists(), "孤儿临时文件应被清掉");
        assert!(unrelated.exists(), "不相关的 .tmp 不能动");
    }
}
