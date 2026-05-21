use std::path::Path;

use crate::util::get_cwd;

pub fn scan_project_tree() -> String {
    let cwd = match get_cwd() {
        Ok(p) => p,
        Err(_) => return String::new(),
    };

    let default_excludes: &[&str] = &[
        ".git",
        "node_modules",
        "target",
        "__pycache__",
        ".venv",
        "venv",
        "vendor",
        ".idea",
        ".vscode",
        ".DS_Store",
        "dist",
        "build",
    ];

    let mut entries: Vec<String> = Vec::new();
    let max_files = 100;

    scan_dir_recursive(&cwd, &cwd, default_excludes, &mut entries, max_files, 0);

    if entries.is_empty() {
        return String::new();
    }

    entries.sort();
    entries.join("\n")
}

fn scan_dir_recursive(
    dir: &Path,
    base: &Path,
    excludes: &[&str],
    entries: &mut Vec<String>,
    max_files: usize,
    depth: usize,
) {
    if entries.len() >= max_files || depth > 5 {
        return;
    }

    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    let mut dirs_to_scan: Vec<std::path::PathBuf> = Vec::new();

    for entry in read_dir.flatten() {
        if entries.len() >= max_files {
            break;
        }

        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if excludes.iter().any(|&e| e == name_str.as_ref()) {
            continue;
        }

        if name_str.starts_with('.') {
            continue;
        }

        let path = entry.path();

        if path.is_dir() {
            dirs_to_scan.push(path);
        } else if path.is_file() {
            let rel = path.strip_prefix(base).unwrap_or(&path);
            entries.push(rel.display().to_string());
        }
    }

    for subdir in dirs_to_scan {
        if entries.len() >= max_files {
            break;
        }
        scan_dir_recursive(&subdir, base, excludes, entries, max_files, depth + 1);
    }
}
