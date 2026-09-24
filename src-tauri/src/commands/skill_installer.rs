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
        // canonicalize 在 Windows 返回 UNC 路径（\\?\...），mklink /J 不认，去掉前缀
        let canon = dev.canonicalize().unwrap_or(dev);
        let s = canon.to_string_lossy();
        let stripped = s.strip_prefix(r"\\?\").map(|x| x.to_string()).unwrap_or_else(|| s.to_string());
        return Ok(PathBuf::from(stripped));
    }
    // 生产模式：Tauri 资源目录（三平台一致：
    //   Windows = {InstallDir}\resources；macOS = .app/Contents/Resources；Linux = /usr/lib/{exe}/…）
    //
    // 落点取决于 tauri.conf.json 的 resources 写法：
    //   · 当前用 map 形式 {"../skills/": "skills/"} → 显式指定目标，落 {resource_dir}/skills
    //   · 早期数组形式 ["../skills/"] → bundler 把父级 ".." 重命名为 "_up_"，落 {resource_dir}/_up_/skills
    // 两个候选都探测，兼容已发布的旧安装包（改 map 前打包的那批）。顺序：先当前写法，后旧写法。
    if let Ok(res_dir) = app.path().resource_dir() {
        let candidates = [
            res_dir.join("skills"),              // 当前配置（map 形式）
            res_dir.join("_up_").join("skills"), // 旧包兼容（数组形式）
        ];
        for res in candidates {
            if res.exists() {
                return Ok(res);
            }
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
        check_target("claude", "Claude Code / OpenCode / mimocode", home.join(".claude").join("skills")),
        check_target("codex", "Codex", home.join(".codex").join("skills")),
    ];
    Ok(targets)
}

/// App 启动时静默检测已安装的技能链接是否有效，失效则重建。
/// 只处理「已安装但源目录不存在」的情况（App 更新后资源目录被替换）。
/// 未安装的目标不处理（用户没装过就不自动装）。
pub fn repair_skill_links(app: &tauri::AppHandle) {
    let Ok(src) = builtin_skills_dir(app) else { return };
    let Ok(home) = home_dir() else { return };
    let version = app.package_info().version.to_string();

    for (id, skills_dir) in [
        ("claude", home.join(".claude").join("skills")),
        ("codex", home.join(".codex").join("skills")),
    ] {
        // 该目标下是否装过技能：以「链接/目录**本身**存在」为准。
        // 不能用 exists() —— 它跟随链接看目标，悬空链接会返回 false，导致下面的修复永远不触发。
        let any_installed = SKILL_NAMES
            .iter()
            .any(|s| skills_dir.join(s).symlink_metadata().is_ok());
        if !any_installed {
            continue;
        }

        let mut repaired = Vec::new();
        for skill in SKILL_NAMES {
            let dest = skills_dir.join(skill);
            // 链接/目录本身不存在 → 该技能没装过，跳过
            if dest.symlink_metadata().is_err() {
                continue;
            }

            // 需要重建的两种情况：
            //   1) 链接悬空 —— 链接本身在（symlink_metadata 成功）但目标不存在（exists 跟随链接后为 false）
            //      （App 被移动/更新到新路径时）
            //   2) 副本版本过期 —— 副本不会像链接那样自动跟随 App，需比对「出厂版本」标记
            let reason = if !dest.exists() {
                Some("检测到悬空链接")
            } else if !is_link(&dest) {
                let marked = std::fs::read_to_string(dest.join(COPY_VERSION_MARKER)).ok();
                let stale = marked.as_deref().map(str::trim) != Some(version.as_str());
                if stale { Some("副本版本过期") } else { None }
            } else {
                None // 有效链接：自动跟随 App 资源目录，无需处理
            };

            if let Some(reason) = reason {
                eprintln!("[skill_installer] {reason}: {} → 重建", dest.display());
                match create_link(&src.join(skill), &dest, &version) {
                    Ok(method) => repaired.push(format!("{skill}:{method}")),
                    Err(e) => eprintln!("[skill_installer] 重建 {skill} 失败: {e}"),
                }
            }
        }
        if !repaired.is_empty() {
            eprintln!("[skill_installer] {id} 技能已修复: {}", repaired.join(", "));
        }
    }
}

/// 复制式安装的版本标记文件名（记录该副本由哪个 App 版本产出）。
/// 副本不会像链接那样自动跟随 App 资源目录，故用它判断是否需要刷新。
const COPY_VERSION_MARKER: &str = ".apicat-skills-version";

/// 是否为「链接式」安装（Unix symlink / Windows junction）。
/// 链接自动跟随 App 资源目录的更新，无需版本刷新；只有副本才需要。
/// 用 read_link 判定：对 symlink 与 junction 均返回 Ok，对真实目录/副本返回 Err（本机实测）。
fn is_link(path: &Path) -> bool {
    std::fs::read_link(path).is_ok()
}

/// 在目录上建链接（junction / symlink），失败回退复制
fn create_link(src: &Path, dest: &Path, version: &str) -> Result<String, crate::error::AppError> {
    // 确保父目录存在
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| crate::error::AppError::Custom(format!("创建目录失败: {e}")))?;
    }

    // 如果已存在，先删
    if dest.exists() || dest.symlink_metadata().is_ok() {
        remove_link(dest)?;
    }

    // Windows: junction（mklink /J，无需管理员权限）
    #[cfg(target_os = "windows")]
    {
        let src_str = src.to_string_lossy().to_string();
        let dest_str = dest.to_string_lossy().to_string();
        eprintln!("[skill_installer] mklink /J \"{dest_str}\" \"{src_str}\"");
        let output = std::process::Command::new("cmd")
            .args(["/C", "mklink", "/J", &dest_str, &src_str])
            .output();
        match output {
            Ok(o) if o.status.success() => return Ok("junction".to_string()),
            Ok(o) => {
                eprintln!("[skill_installer] mklink /J 失败: {}", String::from_utf8_lossy(&o.stderr));
            }
            Err(e) => {
                eprintln!("[skill_installer] mklink 命令执行失败: {e}");
            }
        }
    }

    // macOS/Linux: symlink
    //
    // 例外 —— AppImage：每次启动挂载到**新的**临时目录（/tmp/.mount_XXXX/），软链到挂载点内的
    // 资源会随 App 退出立刻悬空，导致 Claude Code 在 ApiCat 未运行时读不到技能。
    // 检测到 AppImage（运行时注入 APPDIR 环境变量）时跳过软链，直接走下面的文件复制，让安装自包含。
    // 副本的“过期”问题由 repair_skill_links 依据副本内的版本标记（COPY_VERSION_MARKER）刷新解决。
    #[cfg(not(target_os = "windows"))]
    {
        if std::env::var_os("APPDIR").is_none() {
            use std::os::unix::fs::symlink;
            if symlink(src, dest).is_ok() {
                return Ok("symlink".to_string());
            }
        }
    }

    // 回退 / AppImage：文件复制
    copy_dir_recursive(src, dest)?;
    // 副本携带「出厂版本」，供 repair_skill_links 在 App 升级后判断是否需要刷新。
    // 写失败不报错：标记缺失会在下次启动被判为「版本过期」从而自动重刷，自愈。
    let _ = std::fs::write(dest.join(COPY_VERSION_MARKER), version);
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
    let Ok(meta) = dest.symlink_metadata() else { return Ok(()) };
    // Windows 分支说明：
    // junction 在 Rust 的 symlink_metadata 下 is_dir() 返回 false（reparse point，is_symlink()
    // 也返回 false），不能靠 is_dir() 分流。用三段式尝试，不依赖任何元数据判定：
    //   remove_dir     → 删 junction / symlink→目录 / 空目录 的链接本身
    //   remove_file    → 删普通文件 / symlink→文件
    //   remove_dir_all → 删真实非空目录
    // 每一种类型恰好命中其中一步，顺序不可调换。
    #[cfg(target_os = "windows")]
    {
        let _ = &meta; // meta 仅用于上方存在性检查；Windows 删除不依赖元数据
        if std::fs::remove_dir(dest).is_ok() {
            return Ok(());
        }
        if std::fs::remove_file(dest).is_ok() {
            return Ok(());
        }
        std::fs::remove_dir_all(dest)
            .map_err(|e| crate::error::AppError::Custom(format!("删除目录失败: {e}")))?;
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        if meta.file_type().is_symlink() || meta.is_file() {
            std::fs::remove_file(dest)
                .map_err(|e| crate::error::AppError::Custom(format!("删除链接失败: {e}")))?;
        } else {
            std::fs::remove_dir_all(dest)
                .map_err(|e| crate::error::AppError::Custom(format!("删除目录失败: {e}")))?;
        }
        Ok(())
    }
}

/// 安装技能到指定目标
#[tauri::command]
pub async fn install_skills(app: tauri::AppHandle, target_id: String) -> CmdResult<String> {
    let src = builtin_skills_dir(&app)?;
    let home = home_dir()?;
    let version = app.package_info().version.to_string();
    let dest = match target_id.as_str() {
        "claude" => home.join(".claude").join("skills"),
        "codex" => home.join(".codex").join("skills"),
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
        let method = create_link(&skill_src, &skill_dest, &version)?;
        methods.push(format!("{skill}:{method}"));
    }
    Ok(methods.join(", "))
}

/// 从指定目标卸载技能
#[tauri::command]
pub async fn uninstall_skills(target_id: String) -> CmdResult<()> {
    let home = home_dir()?;
    let dest = match target_id.as_str() {
        "claude" => home.join(".claude").join("skills"),
        "codex" => home.join(".codex").join("skills"),
        _ => return Err(crate::error::AppError::Custom(format!("未知目标: {target_id}"))),
    };
    for skill in SKILL_NAMES {
        remove_link(&dest.join(skill))?;
    }
    Ok(())
}
