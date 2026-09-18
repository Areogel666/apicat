// 1.0.5：AI 技能安装器 — 安装/卸载技能到各 AI Agent 的 skills 目录
//
// 安装目标（2 个，覆盖 4 个 agent）：
//   ~/.claude/skills/  → Claude Code + OpenCode（兼容）+ mimocode（共用）
//   ~/.codex/skills/   → Codex
//
// 链接方式：Windows junction / macOS·Linux symlink；失败回退文件复制。

use crate::error::CmdResult;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::Manager;

/// App 内置技能目录（打包时 resources 带入）
fn builtin_skills_dir(app: &tauri::AppHandle) -> Result<PathBuf, crate::error::AppError> {
    // 开发模式：源码目录下的 skills/
    let dev = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../skills");
    if dev.exists() {
        return Ok(dev.canonicalize().unwrap_or(dev));
    }
    // 生产模式：Tauri 资源目录
    if let Ok(res_dir) = app.path().resource_dir() {
        let res = res_dir.join("skills");
        if res.exists() {
            return Ok(res);
        }
    }
    Err(crate::error::AppError::Custom(
        "未找到内置技能目录（skills/）".to_string(),
    ))
}

/// 用户主目录
fn home_dir() -> Result<PathBuf, crate::error::AppError> {
    #[cfg(target_os = "windows")]
    let home = std::env::var("USERPROFILE").ok();
    #[cfg(not(target_os = "windows"))]
    let home = std::env::var("HOME").ok();
    home.map(PathBuf::from)
        .ok_or_else(|| crate::error::AppError::Custom("无法获取用户主目录".to_string()))
}

/// 安装目标定义
#[derive(Debug, Serialize)]
pub struct SkillTarget {
    /// 目标 id（claude / codex）
    pub id: String,
    /// 显示名
    pub name: String,
    /// skills 目录绝对路径
    pub path: String,
    /// 该目录的父目录是否存在（agent 是否已安装）
    pub agent_installed: bool,
    /// apicat 技能是否已安装（junction/目录存在）
    pub skills_installed: bool,
    /// 已安装的技能名列表
    pub installed_skills: Vec<String>,
}

/// 内置技能名列表
const SKILL_NAMES: &[&str] = &["apicat-lib", "apicat-edit", "apicat-test-gen", "apicat-doc-gen"];

fn check_target(id: &str, name: &str, skills_dir: PathBuf) -> SkillTarget {
    let agent_installed = skills_dir.parent().map_or(false, |p| p.exists());
    let mut installed = Vec::new();
    for skill in SKILL_NAMES {
        if skills_dir.join(skill).exists() {
            installed.push(skill.to_string());
        }
    }
    SkillTarget {
        id: id.to_string(),
        name: name.to_string(),
        path: skills_dir.to_string_lossy().to_string(),
        agent_installed,
        skills_installed: !installed.is_empty(),
        installed_skills: installed,
    }
}

/// 查询所有安装目标的状态
#[tauri::command]
pub async fn get_skill_targets() -> CmdResult<Vec<SkillTarget>> {
    let home = home_dir()?;
    let targets = vec![
        check_target("claude", "Claude Code / OpenCode / mimocode", home.join(".claude/skills")),
        check_target("codex", "Codex", home.join(".codex/skills")),
    ];
    Ok(targets)
}

/// 在目录上建链接（junction / symlink），失败回退复制
fn create_link(src: &Path, dest: &Path) -> Result<String, crate::error::AppError> {
    // 确保父目录存在
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| crate::error::AppError::Custom(format!("创建目录失败: {e}")))?;
    }

    // 如果已存在，先删
    if dest.exists() || dest.symlink_metadata().is_ok() {
        remove_link(dest)?;
    }

    // Windows: junction（无需管理员权限）
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::fs::symlink_dir;
        match symlink_dir(src, dest) {
            Ok(_) => return Ok("junction".to_string()),
            Err(e) => {
                eprintln!("[skill_installer] junction 失败，回退复制: {e}");
            }
        }
    }

    // macOS/Linux: symlink
    #[cfg(not(target_os = "windows"))]
    {
        use std::os::unix::fs::symlink;
        match symlink(src, dest) {
            Ok(_) => return Ok("symlink".to_string()),
            Err(e) => {
                eprintln!("[skill_installer] symlink 失败，回退复制: {e}");
            }
        }
    }

    // 回退：文件复制
    copy_dir_recursive(src, dest)?;
    Ok("copy".to_string())
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), crate::error::AppError> {
    std::fs::create_dir_all(dest)
        .map_err(|e| crate::error::AppError::Custom(format!("创建目录失败: {e}")))?;
    for entry in std::fs::read_dir(src)
        .map_err(|e| crate::error::AppError::Custom(format!("读目录失败: {e}")))?
        .flatten()
    {
        let from = entry.path();
        let to = dest.join(entry.file_name());
        if from.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)
                .map_err(|e| crate::error::AppError::Custom(format!("复制文件失败: {e}")))?;
        }
    }
    Ok(())
}

/// 删除链接（junction / symlink / 目录）
fn remove_link(dest: &Path) -> Result<(), crate::error::AppError> {
    let meta = dest.symlink_metadata();
    match meta {
        Ok(m) => {
            if m.file_type().is_symlink() {
                // symlink / junction
                #[cfg(target_os = "windows")]
                std::fs::remove_dir(dest)
                    .map_err(|e| crate::error::AppError::Custom(format!("删除链接失败: {e}")))?;
                #[cfg(not(target_os = "windows"))]
                std::fs::remove_file(dest)
                    .map_err(|e| crate::error::AppError::Custom(format!("删除链接失败: {e}")))?;
            } else if m.is_dir() {
                // 回退复制产生的真实目录
                std::fs::remove_dir_all(dest)
                    .map_err(|e| crate::error::AppError::Custom(format!("删除目录失败: {e}")))?;
            } else {
                std::fs::remove_file(dest)
                    .map_err(|e| crate::error::AppError::Custom(format!("删除文件失败: {e}")))?;
            }
            Ok(())
        }
        Err(_) => Ok(()), // 不存在，无需删除
    }
}

/// 安装技能到指定目标
#[tauri::command]
pub async fn install_skills(app: tauri::AppHandle, target_id: String) -> CmdResult<String> {
    let src = builtin_skills_dir(&app)?;
    let home = home_dir()?;
    let dest = match target_id.as_str() {
        "claude" => home.join(".claude/skills"),
        "codex" => home.join(".codex/skills"),
        _ => return Err(crate::error::AppError::Custom(format!("未知目标: {target_id}"))),
    };

    let mut methods = Vec::new();
    for skill in SKILL_NAMES {
        let skill_src = src.join(skill);
        if !skill_src.exists() {
            return Err(crate::error::AppError::Custom(format!(
                "内置技能目录缺少: {skill}"
            )));
        }
        let skill_dest = dest.join(skill);
        let method = create_link(&skill_src, &skill_dest)?;
        methods.push(format!("{skill}:{method}"));
    }
    Ok(methods.join(", "))
}

/// 从指定目标卸载技能
#[tauri::command]
pub async fn uninstall_skills(target_id: String) -> CmdResult<()> {
    let home = home_dir()?;
    let dest = match target_id.as_str() {
        "claude" => home.join(".claude/skills"),
        "codex" => home.join(".codex/skills"),
        _ => return Err(crate::error::AppError::Custom(format!("未知目标: {target_id}"))),
    };
    for skill in SKILL_NAMES {
        remove_link(&dest.join(skill))?;
    }
    Ok(())
}
