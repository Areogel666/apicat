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

/// 单个技能的安装状态
#[derive(Debug, Serialize, Clone)]
pub struct SkillState {
    /// 技能名
    pub name: String,
    /// "link"（软链 / junction，自动跟随 App 资源）| "copy"（副本，靠版本标记刷新）| "missing"
    pub kind: String,
    /// 副本的出厂版本标记（写下时的 App 版本）；link / missing 为 None
    pub version: Option<String>,
    /// 是否落后于当前 App 内置的技能（需重建）
    pub stale: bool,
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
    /// 各技能的安装方式与是否需更新
    pub skills: Vec<SkillState>,
    /// 当前 App 版本 = 技能应有的版本（技能随 App 打包分发，无独立版本号）
    pub bundled_version: String,
}

/// 内置技能名列表
const SKILL_NAMES: &[&str] = &["apicat-lib", "apicat-edit", "apicat-test-gen", "apicat-doc-gen", "apicat-doctor"];

/// 判定单个技能的状态 —— 技能的**版本判定唯一真源**。
///
/// `get_skill_targets`（UI 展示）与 `repair_skill_links`（启动静默修复）共用同一份，
/// 避免出现「UI 说过期、启动认为没事」的两套标准。
///
/// `builtin` = 当前 App 的内置技能目录。技能随 App 打包分发，故「最新版本」= 当前 App 的 skills/。
fn skill_state(builtin: Option<&Path>, dest: &Path, name: &str, bundled_version: &str) -> SkillState {
    // 链接/目录**本身**是否存在。不能用 exists()——它跟随链接看目标，悬空链接返回 false，
    // 会把「装过但链接失效」误判成「没装过」，于是永远不修。
    if dest.symlink_metadata().is_err() {
        return SkillState {
            name: name.to_string(),
            kind: "missing".to_string(),
            version: None,
            stale: false,
        };
    }

    if is_link(dest) {
        // 链接的判据是「是否指向当前 App 的内置技能目录」。
        // 指向别处 = 旧版 App 残留的资源目录 → 内容可能是旧的，需重建。
        // （只看链接是否悬空会漏掉这种：旧 App 还在磁盘上时链接并不悬空）
        let stale = match builtin {
            Some(b) => !link_points_to(&b.join(name), dest),
            // 内置目录不可用：无法判定，保守视为最新，不误报「需更新」
            None => false,
        };
        return SkillState {
            name: name.to_string(),
            kind: "link".to_string(),
            version: None,
            stale,
        };
    }

    // 副本不跟随 App 资源目录，靠出厂版本标记判断；标记缺失也判过期（下次启动自愈重刷）
    let marked = std::fs::read_to_string(dest.join(COPY_VERSION_MARKER)).ok();
    let stale = marked.as_deref().map(str::trim) != Some(bundled_version);
    SkillState {
        name: name.to_string(),
        kind: "copy".to_string(),
        version: marked.map(|s| s.trim().to_string()),
        stale,
    }
}

/// 链接是否指向给定目录（跟随链接后与目标同一路径）。
///
/// 两侧都 `canonicalize` 再比：Windows 上 junction 的路径带 `\\?\` 前缀且大小写不敏感，
/// 直接比字符串会把**所有**有效链接误判成过期。canonicalize 统一前缀与大小写；
/// 链接悬空时 dest 侧失败 → 返回 false（判为需重建）。
fn link_points_to(expected: &Path, dest: &Path) -> bool {
    match (std::fs::canonicalize(expected), std::fs::canonicalize(dest)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn check_target(
    id: &str,
    name: &str,
    skills_dir: PathBuf,
    builtin: Option<&Path>,
    bundled_version: &str,
) -> SkillTarget {
    let agent_installed = skills_dir.parent().map_or(false, |p| p.exists());
    let skills: Vec<SkillState> = SKILL_NAMES
        .iter()
        .map(|s| skill_state(builtin, &skills_dir.join(s), s, bundled_version))
        .collect();
    // 「已安装」以链接/目录本身存在为准（skill_state 内用 symlink_metadata），
    // 悬空链接也算装过——它该被显示出来并修，而不是装作没有
    let installed_count = skills.iter().filter(|s| s.kind != "missing").count();
    SkillTarget {
        id: id.to_string(),
        name: name.to_string(),
        path: skills_dir.to_string_lossy().to_string(),
        agent_installed,
        skills_installed: installed_count > 0,
        skills,
        bundled_version: bundled_version.to_string(),
    }
}

/// 查询所有安装目标的状态（含每个技能的安装方式与是否需更新）
#[tauri::command]
pub async fn get_skill_targets(app: tauri::AppHandle) -> CmdResult<Vec<SkillTarget>> {
    let home = home_dir()?;
    let version = app.package_info().version.to_string();
    // 内置技能目录拿不到时降级：仍返回目标列表，只是无法判定链接是否指向当前 App
    // （保守视为最新，不误报「需更新」）——好过整个弹窗打不开
    let builtin = builtin_skills_dir(&app).ok();
    let targets = vec![
        check_target(
            "claude",
            "Claude Code / OpenCode / mimocode",
            home.join(".claude").join("skills"),
            builtin.as_deref(),
            &version,
        ),
        check_target(
            "codex",
            "Codex",
            home.join(".codex").join("skills"),
            builtin.as_deref(),
            &version,
        ),
    ];
    Ok(targets)
}

/// App 启动时静默检测已安装的技能是否为「当前 App 那一份」，不是则重建。
/// 覆盖三种：链接悬空 / 链接指向旧版 App 残留的资源目录 / 副本版本标记过期。
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
            // 判定与 UI（get_skill_targets）走同一份逻辑，避免两套标准
            let st = skill_state(Some(&src), &dest, skill, &version);
            if st.kind == "missing" || !st.stale {
                continue;
            }
            eprintln!(
                "[skill_installer] {skill} 需重建（{}）: {} → 重建",
                st.kind,
                dest.display()
            );
            match create_link(&src.join(skill), &dest, &version) {
                Ok(method) => repaired.push(format!("{skill}:{method}")),
                Err(e) => eprintln!("[skill_installer] 重建 {skill} 失败: {e}"),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 锁住本次修复的核心：链接判据是「指向当前 App 的内置技能目录」，
    /// 而不是「链接没悬空」。旧逻辑下，链接指向旧版 App 残留目录时
    /// （旧 App 还在磁盘上 → 不悬空）会被误判为最新。
    #[cfg(unix)]
    #[test]
    fn link_points_to_requires_exact_target() {
        let base = std::env::temp_dir().join(format!("apicat_link_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let real = base.join("real");
        let stale = base.join("stale");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::create_dir_all(&stale).unwrap();

        let link = base.join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();

        // 指向当前目录 → 最新
        assert!(link_points_to(&real, &link));
        // 指向旧版 App 残留目录 → 必须判为「不是最新」（旧逻辑会漏掉这种）
        assert!(!link_points_to(&stale, &link));

        // 目标被删（链接悬空）→ 也判为需重建
        std::fs::remove_dir_all(&real).unwrap();
        assert!(!link_points_to(&real, &link));

        let _ = std::fs::remove_dir_all(&base);
    }

    /// 副本靠版本标记判断；标记与当前 App 版本不符即过期
    #[test]
    fn skill_state_flags_copy_by_version_marker() {
        let base = std::env::temp_dir().join(format!("apicat_copy_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let dest = base.join("apicat-lib");
        std::fs::create_dir_all(&dest).unwrap();

        // 标记为旧版本 → 过期
        std::fs::write(dest.join(COPY_VERSION_MARKER), "1.0.6").unwrap();
        let st = skill_state(None, &dest, "apicat-lib", "1.0.7");
        assert_eq!(st.kind, "copy");
        assert!(st.stale);
        assert_eq!(st.version.as_deref(), Some("1.0.6"));

        // 标记与当前版本一致 → 最新
        std::fs::write(dest.join(COPY_VERSION_MARKER), "1.0.7").unwrap();
        assert!(!skill_state(None, &dest, "apicat-lib", "1.0.7").stale);

        // 未安装（目录不存在）
        let st = skill_state(None, &base.join("nope"), "apicat-lib", "1.0.7");
        assert_eq!(st.kind, "missing");
        assert!(!st.stale);

        let _ = std::fs::remove_dir_all(&base);
    }
}
