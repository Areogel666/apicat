//! 应用版本回退
//!
//! Tauri updater 默认只接受「比当前新」的版本；回退靠 UpdaterBuilder 的两个能力：
//!   - endpoints：指向目标 tag 的 latest.json（每个 Release 都带一份）
//!   - version_comparator：只接受该 tag 的版本，绕过「只升不降」比较
//!
//! 下载 → 验签 → 安装流程与正常更新完全一致（同一把 pubkey），安装完成后由前端 relaunch。

use crate::error::{AppError, CmdResult};
use serde::Serialize;
use std::time::Duration;
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

/// 与 tauri.conf.json 的 updater endpoint 保持同一仓库
const RELEASES_API: &str = "https://api.github.com/repos/Areogel666/apicat/releases?per_page=30";
const RELEASE_DOWNLOAD_BASE: &str = "https://github.com/Areogel666/apicat/releases/download";

#[derive(Debug, Serialize)]
pub struct AppVersionInfo {
    pub tag: String,
    pub version: String,
    pub name: Option<String>,
    pub published_at: Option<String>,
    pub prerelease: bool,
}

/// 只接受 `vX.Y.Z`：tag 会拼进下载 URL，必须杜绝任意路径/查询串注入
pub fn validate_tag(tag: &str) -> Result<(), AppError> {
    let invalid = || AppError::Custom(format!("非法版本号: {tag}"));
    let rest = tag.strip_prefix('v').ok_or_else(invalid)?;
    let parts: Vec<&str> = rest.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()))
    {
        return Err(invalid());
    }
    Ok(())
}

/// 列出 GitHub 上的正式版本（草稿与非 `vX.Y.Z` 的 tag 过滤掉）
#[tauri::command]
pub async fn list_app_versions() -> CmdResult<Vec<AppVersionInfo>> {
    let client = reqwest::Client::builder()
        .user_agent(format!("ApiCat/{}", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| AppError::Custom(format!("创建 HTTP 客户端失败: {e}")))?;

    let resp = client
        .get(RELEASES_API)
        .send()
        .await
        .map_err(|e| AppError::Custom(format!("获取版本列表失败: {e}")))?;
    if !resp.status().is_success() {
        return Err(AppError::Custom(format!(
            "获取版本列表失败: HTTP {}",
            resp.status()
        )));
    }
    let releases: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| AppError::Custom(format!("解析版本列表失败: {e}")))?;

    let mut out = Vec::new();
    for r in releases {
        if r["draft"].as_bool().unwrap_or(false) {
            continue;
        }
        let tag = r["tag_name"].as_str().unwrap_or_default().to_string();
        if validate_tag(&tag).is_err() {
            continue;
        }
        out.push(AppVersionInfo {
            version: tag.trim_start_matches('v').to_string(),
            tag,
            name: r["name"].as_str().map(str::to_string),
            published_at: r["published_at"].as_str().map(str::to_string),
            prerelease: r["prerelease"].as_bool().unwrap_or(false),
        });
    }
    Ok(out)
}

/// 下载并安装指定版本（可降级，也可同版本重装）。安装完成后由前端重启。
#[tauri::command]
pub async fn install_app_version(app: AppHandle, tag: String) -> CmdResult<()> {
    validate_tag(&tag)?;
    let target = tag.trim_start_matches('v').to_string();
    let endpoint = format!("{RELEASE_DOWNLOAD_BASE}/{tag}/latest.json")
        .parse::<url::Url>()
        .map_err(|e| AppError::Custom(format!("构造更新地址失败: {e}")))?;

    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(|e| AppError::Custom(format!("配置更新源失败: {e}")))?
        .version_comparator(move |_current, remote| remote.version.to_string() == target)
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| AppError::Custom(format!("初始化更新器失败: {e}")))?;

    let Some(update) = updater
        .check()
        .await
        .map_err(|e| AppError::Custom(format!("获取 {tag} 更新信息失败: {e}")))?
    else {
        return Err(AppError::Custom(format!(
            "版本 {tag} 不存在，或没有适配当前平台的安装包"
        )));
    };

    update
        .download_and_install(|_chunk, _total| {}, || {})
        .await
        .map_err(|e| AppError::Custom(format!("安装 {tag} 失败: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 锁住 tag 校验：不允许 `1.0.10`（缺 v）、路径穿越、查询串、非数字段
    #[test]
    fn tag_validation_rejects_url_injection() {
        for ok in ["v1.0.10", "v1.1.0", "v0.0.1"] {
            assert!(validate_tag(ok).is_ok(), "{ok} 应合法");
        }
        for bad in [
            "1.0.10",
            "v1.0",
            "v1.0.10/../evil",
            "v1.0.a",
            "v..1",
            "v1.0.10?x=1",
            "v1.0.10/",
            "",
        ] {
            assert!(validate_tag(bad).is_err(), "{bad} 应被拒绝");
        }
    }
}
