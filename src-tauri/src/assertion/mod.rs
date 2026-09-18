// 1.0.5 断言求值引擎
// 断言 JSON 存在 test_cases.assertions 列，形状：
//   [{"type":"status_code","operator":"eq","expected":"200"},
//    {"type":"json_path","path":"$.code","operator":"eq","expected":"0"}]
// 起步支持两种 type：status_code / json_path

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assertion {
    /// "status_code" | "json_path"
    #[serde(rename = "type")]
    pub kind: String,
    /// json_path 专用，如 "$.code" / "$.data.list[0].id"
    #[serde(default)]
    pub path: String,
    /// "eq" | "ne" | "not_null" | "contains"
    pub operator: String,
    pub expected: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssertionResult {
    pub kind: String,
    pub path: String,
    pub operator: String,
    pub expected: String,
    /// 实际值的字符串表示（截断到 200 字符，便于前端展示）
    pub actual: String,
    pub passed: bool,
    /// 失败原因（路径不存在 / 类型不匹配等），通过时为空
    pub message: String,
}

/// 解析断言 JSON 数组；空串 / 非法 JSON → 空 Vec（非法时打日志便于排查）
pub fn parse_assertions(raw: &str) -> Vec<Assertion> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return Vec::new();
    }
    match serde_json::from_str::<Vec<Assertion>>(trimmed) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[assertion] 解析断言 JSON 失败: {e}, raw={}", &trimmed[..trimmed.len().min(200)]);
            Vec::new()
        }
    }
}

/// 按路径取值。支持：
///   $           → 根
///   $.a.b.c     → 嵌套字段
///   $.list[0]   → 数组下标
///   $.a[0].b    → 混合
/// 路径不存在返回 None
pub fn extract_json_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let path = path.trim();
    if path.is_empty() || path == "$" {
        return Some(root);
    }
    let path = path.strip_prefix("$").unwrap_or(path);
    let mut current = root;

    // 按 . 切段，段内再解析 [n] 下标
    for segment in path.split('.') {
        if segment.is_empty() {
            continue;
        }
        // 拆出 key 与下标序列：list[0][1] → key="list", indices=[0,1]
        let (key, rest) = match segment.find('[') {
            Some(i) => (&segment[..i], &segment[i..]),
            None => (segment, ""),
        };

        if !key.is_empty() {
            current = current.get(key)?;
        }

        // 逐个吃掉 [n]
        let mut rest = rest;
        while rest.starts_with('[') {
            let end = rest.find(']')?;
            let idx_str = &rest[1..end];
            let idx: usize = idx_str.parse().ok()?;
            current = current.get(idx)?;
            rest = &rest[end + 1..];
        }
    }
    Some(current)
}

/// 把 JSON 值转成用于比较/展示的字符串
fn value_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => v.to_string(), // object/array → compact JSON
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let cut: String = s.chars().take(max).collect();
        format!("{cut}…")
    }
}

/// 求值单条断言
pub fn evaluate(assertion: &Assertion, status_code: u16, body: &Value) -> AssertionResult {
    let mut result = AssertionResult {
        kind: assertion.kind.clone(),
        path: assertion.path.clone(),
        operator: assertion.operator.clone(),
        expected: assertion.expected.clone(),
        actual: String::new(),
        passed: false,
        message: String::new(),
    };

    match assertion.kind.as_str() {
        "status_code" => {
            let actual = status_code.to_string();
            result.actual = actual.clone();
            result.passed = match assertion.operator.as_str() {
                "eq" => actual == assertion.expected,
                "ne" => actual != assertion.expected,
                op => {
                    result.message = format!("不支持的 status_code 操作符: {op}");
                    false
                }
            };
            if !result.passed && result.message.is_empty() {
                result.message = format!("期望 status_code {} {}，实际 {actual}", assertion.operator, assertion.expected);
            }
        }
        "json_path" => {
            match extract_json_path(body, &assertion.path) {
                None => {
                    result.actual = "<路径不存在>".to_string();
                    result.message = format!("路径 {} 在响应中不存在", assertion.path);
                }
                Some(v) => {
                    let actual = value_to_string(v);
                    result.actual = truncate(&actual, 200);
                    result.passed = match assertion.operator.as_str() {
                        "eq" => actual == assertion.expected,
                        "ne" => actual != assertion.expected,
                        "not_null" => !v.is_null(),
                        "contains" => actual.contains(&assertion.expected),
                        op => {
                            result.message = format!("不支持的 json_path 操作符: {op}");
                            false
                        }
                    };
                    if !result.passed && result.message.is_empty() {
                        result.message = format!(
                            "期望 {} {} {}，实际 {}",
                            assertion.path, assertion.operator, assertion.expected, result.actual
                        );
                    }
                }
            }
        }
        other => {
            result.message = format!("不支持的断言类型: {other}");
        }
    }

    result
}

/// 批量求值。断言为空时返回空 Vec（调用方据此判定「无断言 = 不判对错」）
pub fn evaluate_all(assertions: &[Assertion], status_code: u16, body_str: &str) -> Vec<AssertionResult> {
    let body: Value = serde_json::from_str(body_str).unwrap_or(Value::Null);
    assertions.iter().map(|a| evaluate(a, status_code, &body)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_nested() {
        let v = json!({"code": 0, "data": {"list": [{"id": 42}]}});
        assert_eq!(extract_json_path(&v, "$.code"), Some(&json!(0)));
        assert_eq!(extract_json_path(&v, "$.data.list[0].id"), Some(&json!(42)));
        assert_eq!(extract_json_path(&v, "$.missing"), None);
    }

    #[test]
    fn test_evaluate_status() {
        let a = Assertion { kind: "status_code".into(), path: "".into(), operator: "eq".into(), expected: "200".into() };
        let body = json!({});
        assert!(evaluate(&a, 200, &body).passed);
        assert!(!evaluate(&a, 404, &body).passed);
    }

    #[test]
    fn test_evaluate_json_path() {
        let body = json!({"code": 0, "msg": "success", "data": null});
        let a = Assertion { kind: "json_path".into(), path: "$.code".into(), operator: "eq".into(), expected: "0".into() };
        assert!(evaluate(&a, 200, &body).passed);

        let a2 = Assertion { kind: "json_path".into(), path: "$.msg".into(), operator: "contains".into(), expected: "succ".into() };
        assert!(evaluate(&a2, 200, &body).passed);

        let a3 = Assertion { kind: "json_path".into(), path: "$.data".into(), operator: "not_null".into(), expected: "".into() };
        assert!(!evaluate(&a3, 200, &body).passed);
    }
}
