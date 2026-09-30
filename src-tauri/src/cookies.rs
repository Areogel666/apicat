//! Cookie 语义层：domain/path 匹配、Set-Cookie 解析、注入与回存。
//!
//! 1.0.7 决策落地（详见 docs/release/1.0.7.md）：
//!   - 归属层级 全局 / 项目 / 环境，覆盖优先级 环境 > 项目 > 全局
//!   - domain 支持子域匹配（`host == d || host.ends_with(".d")`）
//!   - path 简单前缀匹配（Q22-B，接受 `/api` 串到 `/apixyz` 的偏差）
//!   - expires 参与过滤；secure / http_only 存而不匹配
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
pub fn filter_cookies(rows: Vec<Cookie>, host: &str, path: &str) -> Vec<Cookie> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
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
        if seen.insert(format!("{}\u{0}{}", c.name, c.path)) {
            out.push(c);
        }
    }
    out
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
/// 一次取回全局/项目/环境三层，再在应用层做子域、path、过期与优先级过滤。
/// 子域匹配无法用 `domain=?` 表达，故不能只靠 SQL 过滤。
pub async fn collect_cookies_for_url(
    pool: &SqlitePool,
    url: &str,
    project_id: Option<i64>,
    env_id: Option<i64>,
) -> Result<Vec<Cookie>, crate::error::AppError> {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return Ok(Vec::new());
    };
    let Some(host) = parsed.host_str().map(|s| s.to_string()) else {
        return Ok(Vec::new());
    };
    let path = parsed.path().to_string();

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

    Ok(filter_cookies(rows, &host, &path))
}

/// 同名（domain+path+name+作用域）覆盖写入；找不到就插一条
async fn upsert_cookie(
    pool: &SqlitePool,
    scope_type: &str,
    project_id: Option<i64>,
    env_id: Option<i64>,
    pc: &ParsedSetCookie,
) -> Result<(), crate::error::AppError> {
    // `IS ?` 让 NULL 也能参与等值比较（`=` 遇 NULL 恒不匹配）
    let existing: Option<i64> = sqlx::query_scalar(
        "SELECT id FROM cookies WHERE scope_type=? AND project_id IS ? AND env_id IS ? \
         AND domain=? AND path=? AND name=? LIMIT 1",
    )
    .bind(scope_type)
    .bind(project_id)
    .bind(env_id)
    .bind(&pc.domain)
    .bind(&pc.path)
    .bind(&pc.name)
    .fetch_optional(pool)
    .await?;

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
        let out = filter_cookies(rows, "e.com", "/");
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
        let out = filter_cookies(rows, "sub.e.com", "/api");
        let names: Vec<_> = out.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["ok", "sub"]);
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
}
