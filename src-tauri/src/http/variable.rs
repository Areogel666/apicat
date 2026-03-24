use std::collections::HashMap;

/// 替换字符串中的 {{variable}} 占位符。
/// 当 is_json_body=true 时，会先对值做 JSON 转义，避免破坏 JSON 结构。
pub fn replace_variables(
    input: &str,
    variables: &HashMap<String, String>,
    is_json_body: bool,
) -> String {
    // 快速路径：没有变量或输入中没有占位符标记，直接返回原文
    if variables.is_empty() || !input.contains("{{") {
        return input.to_string();
    }

    let mut result = input.to_string();
    for (key, value) in variables {
        let placeholder = format!("{{{{{key}}}}}");
        let replacement = if is_json_body {
            serde_json::to_string(value)
                .unwrap_or_else(|_| value.clone())
                .trim_matches('"')
                .to_string()
        } else {
            value.clone()
        };

        result = result.replace(&placeholder, &replacement);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::replace_variables;
    use std::collections::HashMap;

    #[test]
    fn test_basic_replacement() {
        let mut vars = HashMap::new();
        vars.insert(
            "base_url".to_string(),
            "https://api.example.com".to_string(),
        );

        let output = replace_variables("{{base_url}}/users", &vars, false);
        assert_eq!(output, "https://api.example.com/users");
    }

    #[test]
    fn test_json_escape() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "foo\"bar".to_string());

        let output = replace_variables(r#"{"name":"{{name}}"}"#, &vars, true);
        assert_eq!(output, r#"{"name":"foo\"bar"}"#);
    }

    #[test]
    fn test_no_variables() {
        let vars = HashMap::new();
        let output = replace_variables("plain text", &vars, false);
        assert_eq!(output, "plain text");
    }

    #[test]
    fn test_multiple_variables() {
        let mut vars = HashMap::new();
        vars.insert("host".to_string(), "localhost".to_string());
        vars.insert("port".to_string(), "8080".to_string());

        let output = replace_variables("{{host}}:{{port}}", &vars, false);
        assert_eq!(output, "localhost:8080");
    }
}
