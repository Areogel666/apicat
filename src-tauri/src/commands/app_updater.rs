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

/// 与 tauri.conf.json 的 updater endpoint 保持同一仓库。
///
/// 用 releases.atom 而不是 REST API：未认证 REST 是 60 次/小时/IP，
/// 共享出口 IP 很容易打满返回 403；atom feed 走网页 CDN，无此限流（最近 10 个版本）。
const RELEASES_ATOM: &str = "https://github.com/Areogel666/apicat/releases.atom";
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

/// 从 atom feed 条目中截取 start..end 之间的文本
fn between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let i = s.find(start)? + start.len();
    let j = s[i..].find(end)? + i;
    Some(&s[i..j])
}

/// 只解码 atom 里会出现的几个实体（标题是应用自己生成的中英文，无嵌套标记）
fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

/// 解析 GitHub releases.atom。tag 从 `<link .../releases/tag/vX.Y.Z />` 取，
/// 时间取 `<updated>`，名称取 `<title>`。
pub fn parse_releases_atom(xml: &str) -> Vec<AppVersionInfo> {
    let mut out = Vec::new();
    for entry in xml.split("<entry>").skip(1) {
        let Some(tag) = between(entry, "/releases/tag/", "\"").map(str::trim) else {
            continue;
        };
        if validate_tag(tag).is_err() {
            continue;
        }
        out.push(AppVersionInfo {
            tag: tag.to_string(),
            version: tag.trim_start_matches('v').to_string(),
            name: between(entry, "<title>", "</title>").map(|s| decode_entities(s.trim())),
            published_at: between(entry, "<updated>", "</updated>").map(|s| s.trim().to_string()),
            // atom feed 不区分 prerelease，统一按正式版本展示
            prerelease: false,
        });
    }
    out
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
        .get(RELEASES_ATOM)
        .send()
        .await
        .map_err(|e| AppError::Custom(format!("获取版本列表失败: {e}")))?;
    if !resp.status().is_success() {
        return Err(AppError::Custom(format!(
            "获取版本列表失败: HTTP {}",
            resp.status()
        )));
    }
    let xml = resp
        .text()
        .await
        .map_err(|e| AppError::Custom(format!("读取版本列表失败: {e}")))?;
    Ok(parse_releases_atom(&xml))
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

    /// atom 解析：tag 从 link 取、名称/时间正确、非法 tag 被过滤
    #[test]
    fn parses_releases_atom() {
        let xml = r#"<feed>
  <entry>
    <id>tag:github.com,2008:Repository/1/v1.0.10</id>
    <updated>2026-11-11T10:00:00Z</updated>
    <link rel="alternate" type="text/html" href="https://github.com/x/apicat/releases/tag/v1.0.10"/>
    <title>ApiCat v1.0.10</title>
  </entry>
  <entry>
    <updated>2026-11-10T10:00:00Z</updated>
    <link rel="alternate" href="https://github.com/x/apicat/releases/tag/nightly"/>
    <title>nightly build</title>
  </entry>
</feed>"#;
        let versions = parse_releases_atom(xml);
        assert_eq!(versions.len(), 1);
        assert_eq!(versions[0].tag, "v1.0.10");
        assert_eq!(versions[0].version, "1.0.10");
        assert_eq!(versions[0].name.as_deref(), Some("ApiCat v1.0.10"));
        assert_eq!(
            versions[0].published_at.as_deref(),
            Some("2026-11-11T10:00:00Z")
        );
    }

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
