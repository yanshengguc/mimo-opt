use std::path::{Path, PathBuf};

use crate::util::now_secs;

/// 去除 Windows 的 \\?\ 前缀，避免 canonicalize 后路径比较失败
fn strip_unc_prefix(path: &Path) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix("\\\\?\\") {
        PathBuf::from(stripped)
    } else {
        path.to_path_buf()
    }
}

/// 规范化路径：canonicalize + 去除 Windows UNC 前缀
fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize()
        .map(|p| strip_unc_prefix(&p))
        .unwrap_or_else(|_| path.to_path_buf())
}

/// 验证路径安全性：禁止 ../穿越、禁止绝对路径、.git/ 不可写
pub fn validate_path(path_str: &str, cwd: &Path) -> Result<PathBuf, String> {
    let path = Path::new(path_str);

    if path.is_absolute() {
        return Err("禁止绝对路径".into());
    }

    let normalized = cwd.join(path);
    let canonical = normalize_path(&normalized);
    let canonical_cwd = normalize_path(cwd);

    if !canonical.starts_with(&canonical_cwd) {
        return Err("路径穿越被禁止".into());
    }

    let rel = canonical.strip_prefix(&canonical_cwd).unwrap_or(&canonical);
    for component in rel.components() {
        if let std::path::Component::Normal(name) = component {
            if name == ".git" {
                return Err("禁止访问 .git 目录".into());
            }
        }
    }

    Ok(normalized)
}

/// 读取文件，可选指定行范围 (start, end)，均为 1-indexed
pub fn read_file(path: &Path, line_range: Option<(usize, usize)>) -> Result<String, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取失败: {}", e))?;

    match line_range {
        Some((start, end)) => {
            let lines: Vec<&str> = content.lines().collect();
            let s = start.saturating_sub(1).min(lines.len());
            let e = end.min(lines.len());
            if s >= e {
                return Err("行范围无效".into());
            }
            let selected = &lines[s..e];
            let mut result = String::new();
            for (i, line) in selected.iter().enumerate() {
                result.push_str(&format!("{:>4} | {}\n", start + i, line));
            }
            Ok(result)
        }
        None => {
            let total = content.lines().count();
            let mut result = String::new();
            for (i, line) in content.lines().enumerate() {
                result.push_str(&format!("{:>4} | {}\n", i + 1, line));
            }
            result.push_str(&format!("\n[{} lines]", total));
            Ok(result)
        }
    }
}

/// 写入文件，自动备份（如果已存在）
pub fn write_file(path: &Path, content: &str) -> Result<String, String> {
    if path.exists() {
        backup_file(path)?;
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("创建目录失败: {}", e))?;
    }
    std::fs::write(path, content)
        .map_err(|e| format!("写入失败: {}", e))?;
    Ok(format!("已写入 {} ({} B)", path.display(), content.len()))
}

/// 备份文件到 .mimo-opt/backups/，保留最近 5 个版本
fn backup_file(path: &Path) -> Result<(), String> {
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    let backup_dir = cwd.join(".mimo-opt").join("backups");
    std::fs::create_dir_all(&backup_dir)
        .map_err(|e| format!("创建备份目录失败: {}", e))?;

    let file_name = path
        .file_name()
        .ok_or("无效文件名")?
        .to_string_lossy();
    let ts = now_secs();
    let backup_path = backup_dir
        .join(format!("{}.{}.bak", file_name, ts));

    std::fs::copy(path, &backup_path)
        .map_err(|e| format!("备份失败: {}", e))?;

    // 清理旧备份，保留最近 5 个
    let prefix = format!("{}.", file_name);
    let suffix = ".bak";
    let mut backups: Vec<_> = std::fs::read_dir(&backup_dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let n = name.to_string_lossy();
            n.starts_with(&prefix) && n.ends_with(suffix)
        })
        .collect();

    backups.sort_by_key(|e| e.file_name());
    while backups.len() > 5 {
        let oldest = backups.remove(0);
        let _ = std::fs::remove_file(oldest.path());
    }

    Ok(())
}

/// 编辑文件：精确字符串替换，自动备份
pub fn apply_edit(path: &Path, old: &str, new: &str) -> Result<String, String> {
    if old.is_empty() {
        return Err("查找文本不能为空".into());
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("读取失败: {}", e))?;

    if !content.contains(old) {
        return Err("未找到匹配文本".into());
    }

    let count = content.matches(old).count();
    let new_content = content.replace(old, new);

    backup_file(path)?;

    std::fs::write(path, &new_content)
        .map_err(|e| format!("写入失败: {}", e))?;

    Ok(format!(
        "已编辑 {} ({} 处替换 → {} B)",
        path.display(),
        count,
        new_content.len()
    ))
}
