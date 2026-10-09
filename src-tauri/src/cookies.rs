//! Cookie 语义层：domain/path 匹配、Set-Cookie 解析、注入与回存。
//!
//! 1.0.7 决策落地（详见 docs/release/1.0.7.md）：
//!   - 归属层级 全局 / 项目 / 环境，覆盖优先级 环境 > 项目 > 全局
//!   - domain 支持子域匹配（`host == d || host.ends_with(".d")`）
//!   - path 简单前缀匹配（Q22-B，接受 `/api` 串到 `/apixyz` 的偏差）
//!   - expires / secure 参与过滤；http_only 存而不匹配（ApiCat 不是浏览器，没有页面脚本会去读 cookie）
//!   - Max-Age 优先于 Expires；两者皆无 = 会话 cookie（不写 expires_at，永不过期）
//!
//! 本模块是 cookie 查询/注入的**唯一真源**（Q30-A）：
//! `get_cookies_for_domain` 与各发送链路都走 `collect_cookies_for_url`，
//! 不允许再各自内联 SQL。

use crate::{
    http::client::{ParamItem, SendRequestParams},
    sql_cols::COOKIE_COLS,
    types::Cookie,
};
use sqlx::SqlitePool;
use std::collections::HashSet;

// ── 纯函数：domain / path / 过期 ─────────────────────────────

/// 规范化 domain 输入：剥 scheme / path / query / userinfo / port / 开头的点，转小写。
///
/// 用户习惯填 `https://ias-app.cms.intl.miui.com/path`，而发送侧拿 URL host
/// （`ias-app.cms.intl.miui.com`）做匹配——不规范化就永不命中（1.0.7 根因 #1）。
///
/// **顺序敏感**：必须先切 path/query/fragment 再剥 userinfo。
/// 反过来的话 `https://a.com/v1?mail=x@y.com` 会被 query 里的 `@` 截成 `y.com`。
pub fn normalize_domain(input: &str) -> String {
    let mut s = input.trim();
    if let Some(idx) = s.find("://") {
        s = &s[idx + 3..];
    }
    // 1. 先切 path / query / fragment —— 此后 s 只剩 [userinfo@]host[:port]
    if let Some(idx) = s.find(|c| c == '/' || c == '?' || c == '#') {
        s = &s[..idx];
    }
    // 2. 再剥 userinfo（URL 里它在 host 前，且不会含 `/`）
    if let Some(idx) = s.rfind('@') {
        s = &s[idx + 1..];
    }
    // 3. 剥端口。IPv6 字面量含多个冒号，不能按第一个冒号切。
    //    reqwest 的 host_str() 对 IPv6 返回无括号的 `::1`，用户输入则常是 `[::1]:8080`。
    if s.starts_with('[') {
        if let Some(end) = s.find(']') {
            s = &s[..=end]; // 保留 `[..]`
        }
    } else if s.matches(':').count() == 1 {
        // 单冒号才是 host:port
        if let Some(idx) = s.find(':') {
            s = &s[..idx];
        }
    }
    s.trim_start_matches('.').to_lowercase()
}

/// host 是否命中 cookie 的 domain（支持子域）。
///
/// 存的 domain 是父域（`intl.miui.com`）时，`a.intl.miui.com` 也命中；
/// 兄弟域（`other.com`）不命中。两侧都先规范化，容忍用户填了完整 URL。
pub fn domain_matches(cookie_domain: &str, host: &str) -> bool {
    let d = normalize_domain(cookie_domain);
    let h = normalize_domain(host);
    if d.is_empty() || h.is_empty() {
        return false;
    }
    h == d || h.ends_with(&format!(".{d}"))
}

/// 请求 path 是否命中 cookie 的 path。
///
/// ponytail: 简单前缀匹配（Q22-B 明确接受偏差：`/api` 也会匹配 `/apixyz`）。
/// 若日后踩到，补一条边界判断即可：`rp.len()==cp.len() || rp[cp.len()]=='/'`。
pub fn path_matches(cookie_path: &str, request_path: &str) -> bool {
    let cp = if cookie_path.is_empty() { "/" } else { cookie_path };
    let rp = if request_path.is_empty() { "/" } else { request_path };
    rp.starts_with(cp)
}

/// 存库时间格式（UTC，与 SQLite `datetime('now')` 同形，可直接字符串比较）
pub const TS_FMT: &str = "%Y-%m-%d %H:%M:%S";

fn fmt_ts(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|d| d.format(TS_FMT).to_string())
        .unwrap_or_default()
}

/// 是否已过期。`None` = 会话 cookie，永不过期（Q23-A）
pub fn is_expired(expires_at: Option<&str>) -> bool {
    let Some(e) = expires_at else { return false };
    let now = chrono::Utc::now().format(TS_FMT).to_string();
    e <= now.as_str()
}

/// Set-Cookie 无 Path 属性时的默认值：请求路径的目录（RFC 6265 §5.1.4）
fn default_path(request_path: &str) -> String {
    if !request_path.starts_with('/') {
        return "/".to_string();
    }
    match request_path.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(i) => request_path[..i].to_string(),
    }
}

/// 解析 `Expires=` 的 HTTP 日期（IMF-fixdate 及常见变体），返回存库格式
fn parse_http_date(v: &str) -> Option<String> {
    let normalized = v.trim().replace("GMT", "+0000");
    if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(&normalized) {
        return Some(fmt_ts(dt.timestamp()));
    }
    for fmt in ["%a, %d %b %Y %H:%M:%S %z", "%A, %d-%b-%y %H:%M:%S %z"] {
        if let Ok(dt) = chrono::DateTime::parse_from_str(&normalized, fmt) {
            return Some(fmt_ts(dt.timestamp()));
        }
    }
    for fmt in ["%a, %d %b %Y %H:%M:%S", "%Y-%m-%d %H:%M:%S"] {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(v.trim(), fmt) {
            return Some(fmt_ts(dt.and_utc().timestamp()));
        }
    }
    None
}

/// 解析结果（未落库形态）
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedSetCookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    /// `None` = 会话 cookie；`Some(...)` = 存库格式 UTC 时间串
    pub expires_at: Option<String>,
    pub http_only: bool,
    pub secure: bool,
}

/// 解析单条 `Set-Cookie`。`request_host`/`request_path` 用于补默认 domain/path。
///
/// Max-Age 优先于 Expires（Q23-A）。`Max-Age<=0` 表示立即删除——
/// 照样返回一条已过期记录，回存后覆盖旧值，等效于从请求中摘掉。
pub fn parse_set_cookie(raw: &str, request_host: &str, request_path: &str) -> Option<ParsedSetCookie> {
    let mut parts = raw.split(';');
    let first = parts.next()?.trim();
    let eq = first.find('=')?;
    let name = first[..eq].trim().to_string();
    let value = first[eq + 1..].trim().to_string();
    if name.is_empty() {
        return None;
    }

    let mut domain = normalize_domain(request_host);
    let mut path = default_path(request_path);
    let mut expires_at: Option<String> = None;
    let mut max_age: Option<i64> = None;
    let mut http_only = false;
    let mut secure = false;

    for attr in parts {
        let attr = attr.trim();
        let Some(eq) = attr.find('=') else {
            if attr.eq_ignore_ascii_case("httponly") {
                http_only = true;
            } else if attr.eq_ignore_ascii_case("secure") {
                secure = true;
            }
            continue;
        };
        let k = attr[..eq].trim().to_ascii_lowercase();
        let v = attr[eq + 1..].trim();
        match k.as_str() {
            "domain" => {
                let d = normalize_domain(v);
                if !d.is_empty() {
                    domain = d;
                }
            }
            "path" => {
                if !v.is_empty() {
                    path = v.to_string();
                }
            }
            "max-age" => {
                if let Ok(n) = v.parse::<i64>() {
                    max_age = Some(n);
                }
            }
            "expires" => expires_at = parse_http_date(v),
            "httponly" => http_only = true,
            "secure" => secure = true,
            _ => {}
        }
    }

    let expires_at = match max_age {
        Some(n) => Some(fmt_ts(chrono::Utc::now().timestamp() + n)),
        None => expires_at,
    };

    Some(ParsedSetCookie {
        name,
        value,
        domain,
        path,
        expires_at,
        http_only,
        secure,
    })
}

// ── 过滤 / 拼装 ─────────────────────────────────────────────

/// 过滤 + 优先级去重：同 `name+path` 只保留最高层、最新的一条。
///
/// 入参 `rows` 必须已按「env > project > global，层内 id DESC」排序，
/// 这样 `seen` 命中即跳过就等价于「高层覆盖低层」。
///
/// `secure_ok` = 本次请求是否处于安全上下文（https / wss）。按 HTTP 规范，
/// `secure=1` 的 cookie 不该出现在明文请求上，故非安全上下文一律跳过。
///
/// 返回 `(命中的 cookie, 因 Secure 被跳过的条数)`。跳过数只统计已通过
/// domain / path / 过期筛选的候选——即「本该发送、却被 Secure 拦下」的那些。
pub fn filter_cookies(
    rows: Vec<Cookie>,
    host: &str,
    path: &str,
    secure_ok: bool,
) -> (Vec<Cookie>, usize) {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut skipped_secure = 0usize;
    for c in rows {
        if !domain_matches(&c.domain, host) {
            continue;
        }
        if !path_matches(&c.path, path) {
            continue;
        }
        if is_expired(c.expires_at.as_deref()) {
            continue;
        }
        // 必须放在 seen 去重**之前**：被 Secure 拦下的不该占用「高层覆盖低层」的名额，
        // 否则高层那条被拦后，低层同名的非 Secure cookie 会被误判为重复而丢弃
        if c.secure == 1 && !secure_ok {
            skipped_secure += 1;
            continue;
        }
        if seen.insert(format!("{}\u{0}{}", c.name, c.path)) {
            out.push(c);
        }
    }
    (out, skipped_secure)
}

/// 拼 `Cookie` 请求头的值：`a=1; b=2`
pub fn build_cookie_header(cookies: &[Cookie]) -> String {
    cookies
        .iter()
        .map(|c| format!("{}={}", c.name, c.value))
        .collect::<Vec<_>>()
        .join("; ")
}

/// 把 cookie 注入请求参数。**手动 `Cookie` 行在前、存储在后**，拼成一条头，
/// 不产生第二个 `Cookie` 头（`send_request.rs` 原有合并语义，此处收拢）。
pub fn inject_cookies(params: &mut SendRequestParams, cookies: &[Cookie]) {
    let value = build_cookie_header(cookies);
    if value.is_empty() {
        return;
    }
    if let Some(existing) = params
        .headers
        .iter_mut()
        .find(|h| h.enabled && h.key.eq_ignore_ascii_case("cookie"))
    {
        if existing.value.is_empty() {
            existing.value = value;
        } else {
            existing.value = format!("{}; {}", existing.value, value);
        }
    } else {
        params.headers.push(ParamItem {
            key: "Cookie".to_string(),
            value,
            enabled: true,
        });
    }
}

// ── DB：查询 / 回存 ─────────────────────────────────────────

/// 按 URL 收集可注入 cookie（**唯一真源**，Q30-A）。
///
/// 一次取回全局/项目/环境三层，再在应用层做子域、path、过期、Secure 与优先级过滤。
/// 子域匹配无法用 `domain=?` 表达，故不能只靠 SQL 过滤。
///
/// 返回 `(命中的 cookie, 因 Secure 被跳过的条数)`。拆出 `_with_skipped` 只为不动其余
/// 调用点：用例 / 压测 / Bridge 只关心命中结果，只有调试发送需要把跳过数回显给用户。
/// 二者共用本函数，过滤规则仍只有一份。
pub async fn collect_cookies_for_url_with_skipped(
    pool: &SqlitePool,
    url: &str,
    project_id: Option<i64>,
    env_id: Option<i64>,
) -> Result<(Vec<Cookie>, usize), crate::error::AppError> {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return Ok((Vec::new(), 0));
    };
    let Some(host) = parsed.host_str().map(|s| s.to_string()) else {
        return Ok((Vec::new(), 0));
    };
    let path = parsed.path().to_string();
    // wss 也算安全上下文——它同样是加密通道
    let secure_ok = matches!(parsed.scheme(), "https" | "wss");

    let rows = sqlx::query_as::<_, Cookie>(&format!(
        "SELECT {COOKIE_COLS} FROM cookies WHERE enabled=1 AND \
         (scope_type='global' \
          OR (scope_type='project' AND project_id=?) \
          OR (scope_type='env' AND env_id=?)) \
         ORDER BY CASE scope_type WHEN 'env' THEN 0 WHEN 'project' THEN 1 ELSE 2 END, id DESC"
    ))
    .bind(project_id)
    .bind(env_id)
    .fetch_all(pool)
    .await?;

    Ok(filter_cookies(rows, &host, &path, secure_ok))
}

/// 取某 URL 命中的可用 cookie（子域 / path / 过期 / Secure / 优先级过滤后）。
pub async fn collect_cookies_for_url(
    pool: &SqlitePool,
    url: &str,
    project_id: Option<i64>,
    env_id: Option<i64>,
) -> Result<Vec<Cookie>, crate::error::AppError> {
    Ok(
        collect_cookies_for_url_with_skipped(pool, url, project_id, env_id)
            .await?
            .0,
    )
}

/// 按「作用域 + domain + path + name」查已有行 id —— cookie 查重的唯一真源。
///
/// `IS ?` 而非 `=`：NULL 必须能参与等值比较——`=` 遇 NULL 恒为 unknown，
/// global / project 层（project_id / env_id 为 NULL）会永远查不中。
///
/// **不要改依赖表的 UNIQUE 约束**：SQLite 的 UNIQUE 索引遇 NULL 列不判等，
/// `UNIQUE(scope_type, project_id, env_id, domain, path, name)` 对 global / project
/// 作用域实际失效（实测可无限插入完全相同的行），查重一律走本函数。
///
/// 调用方：`upsert_cookie`（Set-Cookie 回存，命中即覆盖）与
/// `commands::cookie::create_cookie_impl`（手工新建，命中即拦截）。
pub async fn find_cookie_id(
    pool: &SqlitePool,
    scope_type: &str,
    project_id: Option<i64>,
    env_id: Option<i64>,
    domain: &str,
    path: &str,
    name: &str,
) -> Result<Option<i64>, crate::error::AppError> {
    let id = sqlx::query_scalar::<_, i64>(
        "SELECT id FROM cookies WHERE scope_type=? AND project_id IS ? AND env_id IS ? \
         AND domain=? AND path=? AND name=? LIMIT 1",
    )
    .bind(scope_type)
    .bind(project_id)
    .bind(env_id)
    .bind(domain)
    .bind(path)
    .bind(name)
    .fetch_optional(pool)
    .await?;
    Ok(id)
}

/// 同名（domain+path+name+作用域）覆盖写入；找不到就插一条
async fn upsert_cookie(
    pool: &SqlitePool,
    scope_type: &str,
    project_id: Option<i64>,
    env_id: Option<i64>,
    pc: &ParsedSetCookie,
) -> Result<(), crate::error::AppError> {
    let existing =
        find_cookie_id(pool, scope_type, project_id, env_id, &pc.domain, &pc.path, &pc.name).await?;

    let (http_only, secure) = (pc.http_only as i64, pc.secure as i64);
    match existing {
        Some(id) => {
            sqlx::query(
                "UPDATE cookies SET value=?, expires_at=?, http_only=?, secure=?, enabled=1 WHERE id=?",
            )
            .bind(&pc.value)
            .bind(&pc.expires_at)
            .bind(http_only)
            .bind(secure)
            .bind(id)
            .execute(pool)
            .await?;
        }
        None => {
            sqlx::query(
                "INSERT INTO cookies (scope_type, project_id, env_id, domain, name, value, path, \
                 expires_at, http_only, secure, enabled) VALUES (?,?,?,?,?,?,?,?,?,?,1)",
            )
            .bind(scope_type)
            .bind(project_id)
            .bind(env_id)
            .bind(&pc.domain)
            .bind(&pc.name)
            .bind(&pc.value)
            .bind(&pc.path)
            .bind(&pc.expires_at)
            .bind(http_only)
            .bind(secure)
            .execute(pool)
            .await?;
        }
    }
    Ok(())
}

/// 回存 `Set-Cookie`（Q5-B）。落库层级（Q9-A）：
/// 有项目+环境 → `env` 层；只有项目 → `project` 层；都没有 → `global`。
/// 返回成功写入条数。
pub async fn persist_set_cookies(
    pool: &SqlitePool,
    resp_headers: &[(String, String)],
    request_url: &str,
    project_id: Option<i64>,
    env_id: Option<i64>,
) -> Result<usize, crate::error::AppError> {
    let Ok(parsed) = reqwest::Url::parse(request_url) else {
        return Ok(0);
    };
    let Some(host) = parsed.host_str() else {
        return Ok(0);
    };
    let req_path = parsed.path().to_string();

    let (scope_type, pid, eid) = if env_id.is_some() && project_id.is_some() {
        ("env", project_id, env_id)
    } else if project_id.is_some() {
        ("project", project_id, None)
    } else {
        ("global", None, None)
    };

    let mut saved = 0;
    for (k, v) in resp_headers {
        if !k.eq_ignore_ascii_case("set-cookie") {
            continue;
        }
        let Some(pc) = parse_set_cookie(v, host, &req_path) else {
            continue;
        };
        upsert_cookie(pool, scope_type, pid, eid, &pc).await?;
        saved += 1;
    }
    Ok(saved)
}

// ── 自检 ────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ck(name: &str, domain: &str, path: &str, expires: Option<&str>) -> Cookie {
        Cookie {
            id: 0,
            scope_type: "global".into(),
            project_id: None,
            env_id: None,
            domain: domain.into(),
            name: name.into(),
            value: "v".into(),
            path: path.into(),
            expires_at: expires.map(|s| s.to_string()),
            http_only: 0,
            secure: 0,
            enabled: 1,
        }
    }

    // ── normalize_domain：根因 #1 的直接回归 ──
    #[test]
    fn normalize_strips_scheme_path_port_and_dot() {
        assert_eq!(
            normalize_domain("https://ias-app.cms.intl.miui.com/path?x=1"),
            "ias-app.cms.intl.miui.com"
        );
        assert_eq!(normalize_domain(".intl.miui.com"), "intl.miui.com");
        assert_eq!(normalize_domain("EXAMPLE.COM:8080"), "example.com");
        assert_eq!(normalize_domain("http://user:pass@Host.Example.com/a"), "host.example.com");
        assert_eq!(normalize_domain("  example.com  "), "example.com");
    }

    // 顺序敏感：先切 path/query 再剥 userinfo，否则 query 里的 `@` 会截错域名
    #[test]
    fn normalize_at_sign_in_query_is_not_userinfo() {
        assert_eq!(
            normalize_domain("https://api.example.com/v1/user?mail=x@y.com"),
            "api.example.com"
        );
        assert_eq!(
            normalize_domain("https://api.example.com/v1@2"),
            "api.example.com"
        );
    }

    // reqwest host_str() 对 IPv6 返回无括号形式；用户输入常带括号+端口
    #[test]
    fn normalize_ipv6_keeps_the_address() {
        assert_eq!(normalize_domain("[::1]:8080"), "[::1]");
        assert_eq!(normalize_domain("::1"), "::1");
        assert_eq!(normalize_domain("2001:db8::1"), "2001:db8::1");
    }

    #[test]
    fn normalize_bare_host_unchanged() {
        assert_eq!(normalize_domain("ias-app.cms.intl.miui.com"), "ias-app.cms.intl.miui.com");
    }

    // ── domain_matches：子域 ──
    #[test]
    fn domain_matches_exact_and_subdomain_only() {
        assert!(domain_matches("example.com", "example.com"));
        assert!(domain_matches(".example.com", "www.example.com"));
        assert!(domain_matches("example.com", "a.b.example.com"));
        // 用户填完整 URL 也要能命中（Q7-C 规范化）
        assert!(domain_matches("https://example.com/", "example.com"));
        // 兄弟域不得命中
        assert!(!domain_matches("example.com", "notexample.com"));
        assert!(!domain_matches("example.com", "example.com.evil.org"));
        assert!(!domain_matches("example.org", "example.com"));
    }

    // ── path_matches：简单前缀 ──
    #[test]
    fn path_matches_is_plain_prefix() {
        assert!(path_matches("/", "/anything"));
        assert!(path_matches("/api", "/api/user"));
        assert!(path_matches("/api", "/api"));
        // Q22-B 明确接受的偏差
        assert!(path_matches("/api", "/apixyz"));
        assert!(!path_matches("/api", "/other"));
    }

    // ── 过期 ──
    #[test]
    fn session_cookie_never_expires() {
        assert!(!is_expired(None));
    }

    #[test]
    fn expired_by_string_compare() {
        assert!(is_expired(Some("2000-01-01 00:00:00")));
        assert!(!is_expired(Some("2999-01-01 00:00:00")));
    }

    // ── parse_set_cookie ──
    #[test]
    fn parse_basic_uses_request_host_and_default_path() {
        let p = parse_set_cookie("token=abc", "api.example.com", "/v1/users").unwrap();
        assert_eq!(p.name, "token");
        assert_eq!(p.value, "abc");
        assert_eq!(p.domain, "api.example.com");
        assert_eq!(p.path, "/v1"); // 目录默认
        assert_eq!(p.expires_at, None); // 会话 cookie
        assert!(!p.http_only && !p.secure);
    }

    #[test]
    fn parse_domain_attribute_allows_subdomain_and_strips_dot() {
        let p = parse_set_cookie("sid=1; Domain=.intl.miui.com", "a.intl.miui.com", "/").unwrap();
        assert_eq!(p.domain, "intl.miui.com");
        assert!(domain_matches(&p.domain, "ias-app.cms.intl.miui.com"));
    }

    #[test]
    fn parse_max_age_beats_expires() {
        let raw = "t=1; Max-Age=3600; Expires=Thu, 01 Jan 1970 00:00:00 GMT";
        let p = parse_set_cookie(raw, "h.com", "/").unwrap();
        let exp = p.expires_at.expect("Max-Age 必须给出过期时间");
        assert!(exp > chrono::Utc::now().format(TS_FMT).to_string(), "Max-Age=3600 应在未来");
    }

    #[test]
    fn parse_zero_max_age_is_immediately_expired() {
        let p = parse_set_cookie("t=1; Max-Age=0", "h.com", "/").unwrap();
        assert!(is_expired(p.expires_at.as_deref()));
    }

    #[test]
    fn parse_expires_http_date() {
        let p = parse_set_cookie("t=1; Expires=Wed, 21 Oct 2099 07:28:00 GMT", "h.com", "/").unwrap();
        assert_eq!(p.expires_at.as_deref(), Some("2099-10-21 07:28:00"));
    }

    #[test]
    fn parse_flags_and_custom_path() {
        let p = parse_set_cookie("t=1; Path=/admin; HttpOnly; Secure", "h.com", "/").unwrap();
        assert_eq!(p.path, "/admin");
        assert!(p.http_only && p.secure);
    }

    #[test]
    fn parse_rejects_empty_name() {
        assert!(parse_set_cookie("=x", "h.com", "/").is_none());
        assert!(parse_set_cookie("noequals", "h.com", "/").is_none());
    }

    // ── filter_cookies：优先级去重 ──
    #[test]
    fn higher_scope_and_newer_id_win_dedupe() {
        // rows 已按 env > project > global，层内 id DESC
        let rows = vec![
            Cookie { id: 9, scope_type: "env".into(), env_id: Some(1), value: "env".into(), ..ck("a", "e.com", "/", None) },
            Cookie { id: 5, scope_type: "project".into(), project_id: Some(1), value: "proj".into(), ..ck("a", "e.com", "/", None) },
            Cookie { id: 3, scope_type: "global".into(), value: "glob".into(), ..ck("a", "e.com", "/", None) },
            Cookie { id: 8, scope_type: "env".into(), env_id: Some(1), value: "b".into(), ..ck("b", "e.com", "/", None) },
        ];
        let (out, _) = filter_cookies(rows, "e.com", "/", true);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].value, "env", "同 name+path 应保留最高层");
        assert_eq!(out[1].value, "b");
    }

    #[test]
    fn filter_drops_mismatched_domain_path_and_expired() {
        let rows = vec![
            ck("ok", "e.com", "/", None),
            ck("sub", ".e.com", "/", None),
            Cookie { value: "x".into(), ..ck("sibling", "other.com", "/", None) },
            Cookie { value: "x".into(), ..ck("badpath", "e.com", "/admin", None) },
            Cookie { value: "x".into(), ..ck("dead", "e.com", "/", Some("2000-01-01 00:00:00")) },
        ];
        let (out, _) = filter_cookies(rows, "sub.e.com", "/api", true);
        let names: Vec<_> = out.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["ok", "sub"]);
    }

    // ── Secure：只在安全上下文发送 ──
    #[test]
    fn secure_cookies_only_sent_over_secure_scheme() {
        let mk = || {
            vec![
                Cookie { secure: 1, ..ck("token", "e.com", "/", None) },
                ck("plain", "e.com", "/", None),
            ]
        };

        // https（安全上下文）：两条都发
        let (out, skipped) = filter_cookies(mk(), "e.com", "/", true);
        assert_eq!(out.len(), 2);
        assert_eq!(skipped, 0);

        // http（明文）：Secure 那条被拦下，并计入跳过数
        let (out, skipped) = filter_cookies(mk(), "e.com", "/", false);
        let names: Vec<_> = out.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["plain"]);
        assert_eq!(skipped, 1);
    }

    // 被 Secure 拦下的不占「高层覆盖低层」的名额：
    // 否则高层那条被拦后，低层同名的非 Secure cookie 会被误判为重复而一起丢掉
    #[test]
    fn skipped_secure_does_not_consume_dedupe_slot() {
        let rows = vec![
            Cookie { id: 2, scope_type: "project".into(), project_id: Some(1), secure: 1, ..ck("t", "e.com", "/", None) },
            Cookie { id: 1, scope_type: "global".into(), ..ck("t", "e.com", "/", None) },
        ];
        let (out, skipped) = filter_cookies(rows, "e.com", "/", false);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, 1, "高层被 Secure 拦下后，低层同名应补位");
        assert_eq!(skipped, 1);
    }

    // ── build / inject ──
    #[test]
    fn build_header_joins_pairs() {
        let rows = vec![ck("a", "e.com", "/", None), ck("b", "e.com", "/", None)];
        assert_eq!(build_cookie_header(&rows), "a=v; b=v");
    }

    #[test]
    fn inject_appends_to_manual_cookie_header_first() {
        let mut p = SendRequestParams {
            method: "GET".into(),
            url: "https://e.com".into(),
            query_params: vec![],
            headers: vec![ParamItem { key: "Cookie".into(), value: "manual=1".into(), enabled: true }],
            body_type: "none".into(),
            body: String::new(),
            path_params: vec![],
            auth_type: String::new(),
            auth_config: String::new(),
        };
        let rows = vec![ck("jar", "e.com", "/", None)];
        inject_cookies(&mut p, &rows);
        assert_eq!(p.headers.len(), 1, "不得产生第二个 Cookie 头");
        assert_eq!(p.headers[0].value, "manual=1; jar=v");
    }

    #[test]
    fn inject_creates_cookie_header_when_absent() {
        let mut p = SendRequestParams {
            method: "GET".into(),
            url: "https://e.com".into(),
            query_params: vec![],
            headers: vec![],
            body_type: "none".into(),
            body: String::new(),
            path_params: vec![],
            auth_type: String::new(),
            auth_config: String::new(),
        };
        inject_cookies(&mut p, &[ck("a", "e.com", "/", None)]);
        assert_eq!(p.headers.len(), 1);
        assert_eq!(p.headers[0].key, "Cookie");
        assert_eq!(p.headers[0].value, "a=v");
    }

    // ── find_cookie_id：cookie 查重的唯一真源 ──
    //
    // 锁住「NULL 必须用 `IS ?` 比对」：一旦改回 `=`，global / project 层
    // （project_id / env_id 为 NULL）永远查不中，重复拦截静默失效。
    // 同理也不能改依赖表的 UNIQUE 约束——SQLite 的 UNIQUE 遇 NULL 列不判等。
    #[tokio::test]
    async fn find_cookie_id_matches_rows_with_null_columns() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::query(
            "CREATE TABLE cookies (id INTEGER PRIMARY KEY, scope_type TEXT, project_id INTEGER, \
             env_id INTEGER, domain TEXT, name TEXT, path TEXT)",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO cookies (id, scope_type, project_id, env_id, domain, name, path) \
             VALUES (1, 'global', NULL, NULL, 'a.com', 'k', '/')",
        )
        .execute(&pool)
        .await
        .unwrap();

        // global 层：两个 NULL 列也要能命中
        assert_eq!(
            find_cookie_id(&pool, "global", None, None, "a.com", "/", "k")
                .await
                .unwrap(),
            Some(1)
        );
        // 作用域不同 → 不误判
        assert_eq!(
            find_cookie_id(&pool, "project", Some(7), None, "a.com", "/", "k")
                .await
                .unwrap(),
            None
        );
        // path 不同 → 不误判
        assert_eq!(
            find_cookie_id(&pool, "global", None, None, "a.com", "/api", "k")
                .await
                .unwrap(),
            None
        );
        // name 不同 → 不误判
        assert_eq!(
            find_cookie_id(&pool, "global", None, None, "a.com", "/", "other")
                .await
                .unwrap(),
            None
        );
    }
}
