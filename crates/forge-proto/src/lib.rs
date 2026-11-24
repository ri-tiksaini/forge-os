#![forbid(unsafe_code)]

//! Minimal newline-framed JSON-RPC 2.0 protocol shared by `forged` and `dev` CLI (Blueprint 6.5.2).

pub mod error;

pub use error::{RpcError, RpcErrorCode};

pub const DEFAULT_SOCKET_PATH: &str = "/run/forge/forged.sock";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: u64,
    pub method: String,
    pub params_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    Ok { id: u64, result_json: String },
    Err { id: u64, error: RpcError },
}

pub fn escape_json(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

impl Request {
    pub fn to_line(&self) -> String {
        format!(
            "{{\"jsonrpc\":\"2.0\",\"id\":{},\"method\":\"{}\",\"params\":{}}}",
            self.id,
            escape_json(&self.method),
            self.params_json
        )
    }

    pub fn parse_line(line: &str) -> Result<Self, RpcError> {
        let trimmed = line.trim();
        let id = extract_u64(trimmed, "id").unwrap_or(1);
        let method = extract_str(trimmed, "method")
            .ok_or_else(|| RpcError::new(RpcErrorCode::InvalidRequest, "missing method"))?;
        let params_json = extract_raw_object(trimmed, "params").unwrap_or_else(|| "{}".to_string());
        Ok(Self {
            id,
            method,
            params_json,
        })
    }
}

impl Response {
    pub fn to_line(&self) -> String {
        match self {
            Response::Ok { id, result_json } => {
                format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result_json}}}")
            }
            Response::Err { id, error } => format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{{\"code\":{},\"message\":\"{}\"}}}}",
                error.code,
                escape_json(&error.message)
            ),
        }
    }

    pub fn parse_line(line: &str) -> Result<Self, RpcError> {
        let trimmed = line.trim();
        let id = extract_u64(trimmed, "id").unwrap_or(1);
        if let Some(err_obj) = extract_raw_object(trimmed, "error") {
            let code =
                extract_i32(&err_obj, "code").unwrap_or(RpcErrorCode::InternalError.as_i32());
            let message =
                extract_str(&err_obj, "message").unwrap_or_else(|| "unknown error".to_string());
            return Ok(Response::Err {
                id,
                error: RpcError { code, message },
            });
        }
        let result_json = extract_raw_value(trimmed, "result")
            .ok_or_else(|| RpcError::new(RpcErrorCode::ParseError, "missing result or error"))?;
        Ok(Response::Ok { id, result_json })
    }
}

pub fn extract_str(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let pos = json.find(&needle)?;
    let rest = &json[pos + needle.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    let mut chars = after.chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut out = String::new();
    let mut escaped = false;
    for ch in chars {
        if escaped {
            match ch {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                other => out.push(other),
            }
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(out);
        } else {
            out.push(ch);
        }
    }
    None
}

pub fn extract_u64(json: &str, key: &str) -> Option<u64> {
    let needle = format!("\"{key}\"");
    let pos = json.find(&needle)?;
    let rest = &json[pos + needle.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

pub fn extract_i32(json: &str, key: &str) -> Option<i32> {
    let needle = format!("\"{key}\"");
    let pos = json.find(&needle)?;
    let rest = &json[pos + needle.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    let num: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '-')
        .collect();
    num.parse().ok()
}

pub fn extract_bool(json: &str, key: &str) -> Option<bool> {
    let needle = format!("\"{key}\"");
    let pos = json.find(&needle)?;
    let rest = &json[pos + needle.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    if after.starts_with("true") {
        Some(true)
    } else if after.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

pub fn extract_str_array(json: &str, key: &str) -> Vec<String> {
    let Some(raw) = extract_raw_value(json, key) else {
        return Vec::new();
    };
    let trimmed = raw.trim();
    if !trimmed.starts_with('[') || !trimmed.ends_with(']') {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut in_str = false;
    let mut escaped = false;
    let mut cur = String::new();
    for ch in trimmed[1..trimmed.len() - 1].chars() {
        if in_str {
            if escaped {
                cur.push(ch);
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                out.push(std::mem::take(&mut cur));
                in_str = false;
            } else {
                cur.push(ch);
            }
        } else if ch == '"' {
            in_str = true;
        }
    }
    out
}

pub fn extract_raw_object(json: &str, key: &str) -> Option<String> {
    let raw = extract_raw_value(json, key)?;
    if raw.starts_with('{') {
        Some(raw)
    } else {
        None
    }
}

pub fn extract_raw_value(json: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let pos = json.find(&needle)?;
    let rest = &json[pos + needle.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    let first = after.chars().next()?;

    if first == '{' || first == '[' {
        let close = if first == '{' { '}' } else { ']' };
        let mut depth = 0usize;
        let mut in_str = false;
        let mut escaped = false;
        for (idx, ch) in after.char_indices() {
            if in_str {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    in_str = false;
                }
            } else {
                match ch {
                    '"' => in_str = true,
                    c if c == first => depth += 1,
                    c if c == close => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(after[..=idx].to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    } else if first == '"' {
        let s = extract_str(json, key)?;
        Some(format!("\"{}\"", escape_json(&s)))
    } else {
        let end = after.find([',', '}', ']']).unwrap_or(after.len());
        Some(after[..end].trim().to_string())
    }
}
