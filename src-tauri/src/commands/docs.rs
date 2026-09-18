// 1.0.5：接口文档面板 — 扫描文档目录 + 打开资源管理器

use crate::error::CmdResult;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize)]
pub struct DocFile {
    /// 相对路径（相对于扫描根目录），用 `/` 分隔
    pub relative_path: String,
    /// 文件名（含扩展名）
    pub name: String,
    /// 所在子目录（相对根目录，空串 = 根目录）
    pub dir: String,
    /// 文件大小（字节）
    pub size: u64,
    /// 修改时间（ISO 8601）
    pub modified_at: String,
    /// 绝对路径（前端打开用）
    pub absolute_path: String,
}

/// 递归扫描目录下的 .md 文件
fn scan_md_files(root: &Path, base: &Path, out: &mut Vec<DocFile>) {
    let Ok(entries) = std::fs::read_dir(root) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            scan_md_files(&path, base, out);
        } else if path.extension().map_or(false, |e| e.eq_ignore_ascii_case("md")) {
            let relative = path.strip_prefix(base).unwrap_or(&path);
            let relative_str = relative.to_string_lossy().replace('\\', "/");
            let name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let dir = relative.parent().map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default();
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            let modified_at = entry
                .metadata()
                .and_then(|m| m.modified())
                .map(|t| {
                    let datetime: chrono::DateTime<chrono::Local> = t.into();
                    datetime.to_rfc3339()
                })
                .unwrap_or_default();
            out.push(DocFile {
                relative_path: relative_str,
                name,
                dir,
                size,
                modified_at,
                absolute_path: path.to_string_lossy().to_string(),
            });
        }
    }
}

/// 获取用户主目录（前端拼默认文档路径用）
#[tauri::command]
pub async fn get_home_dir() -> CmdResult<String> {
    #[cfg(target_os = "windows")]
    let home = std::env::var("USERPROFILE").ok();
    #[cfg(not(target_os = "windows"))]
    let home = std::env::var("HOME").ok();
    home.ok_or_else(|| crate::error::AppError::Custom("无法获取用户主目录".to_string()))
}

/// 扫描文档目录，返回 .md 文件列表
/// dir 为空时返回空列表（前端负责传入已解析的目录）
#[tauri::command]
pub async fn scan_docs_dir(dir: String) -> CmdResult<Vec<DocFile>> {
    let root = PathBuf::from(&dir);
    if !root.exists() || !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    scan_md_files(&root, &root, &mut files);
    // 按路径排序，便于树形展示
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(files)
}

/// 在资源管理器中选中文件（Windows: explorer /select, / macOS: open -R / Linux: xdg-open 目录）
#[tauri::command]
pub async fn reveal_in_explorer(path: String) -> CmdResult<()> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(crate::error::AppError::Custom(format!("路径不存在: {path}")));
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(format!("/select,{}", p.display()))
            .spawn()
            .map_err(|e| crate::error::AppError::Custom(format!("打开资源管理器失败: {e}")))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&p)
            .spawn()
            .map_err(|e| crate::error::AppError::Custom(format!("打开 Finder 失败: {e}")))?;
    }
    #[cfg(target_os = "linux")]
    {
        // xdg-open 不支持选中文件，退而打开所在目录
        let dir = p.parent().unwrap_or(&p);
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map_err(|e| crate::error::AppError::Custom(format!("打开文件管理器失败: {e}")))?;
    }
    Ok(())
}

/// 用系统默认程序打开文件
#[tauri::command]
pub async fn open_file_with_default(path: String) -> CmdResult<()> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(crate::error::AppError::Custom(format!("路径不存在: {path}")));
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &p.to_string_lossy()])
            .spawn()
            .map_err(|e| crate::error::AppError::Custom(format!("打开文件失败: {e}")))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&p)
            .spawn()
            .map_err(|e| crate::error::AppError::Custom(format!("打开文件失败: {e}")))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&p)
            .spawn()
            .map_err(|e| crate::error::AppError::Custom(format!("打开文件失败: {e}")))?;
    }
    Ok(())
}
