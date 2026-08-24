//! Text transformations offered in the detail pane. Each one produces a new
//! text that the app records as a fresh history entry.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transform {
    Trim,
    Lower,
    Upper,
    Title,
    StripBreaks,
    JsonPretty,
    UrlEncode,
    UrlDecode,
}

pub fn apply(t: Transform, input: &str) -> Result<String, String> {
    Ok(match t {
        Transform::Trim => input.trim().to_string(),
        Transform::Lower => input.to_lowercase(),
        Transform::Upper => input.to_uppercase(),
        Transform::Title => title_case(input),
        Transform::StripBreaks => strip_breaks(input),
        Transform::JsonPretty => {
            let v: serde_json::Value =
                serde_json::from_str(input).map_err(|e| format!("invalid JSON: {e}"))?;
            serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?
        }
        Transform::UrlEncode => url_encode(input),
        Transform::UrlDecode => url_decode(input)?,
    })
}

fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut at_word_start = true;
    for c in s.chars() {
        if c.is_whitespace() {
            at_word_start = true;
            out.push(c);
        } else if at_word_start {
            out.extend(c.to_uppercase());
            at_word_start = false;
        } else {
            out.extend(c.to_lowercase());
        }
    }
    out
}

fn strip_breaks(s: &str) -> String {
    s.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~')
}

fn url_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if is_unreserved(b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn url_decode(s: &str) -> Result<String, String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = bytes
                .get(i + 1..i + 3)
                .and_then(|h| std::str::from_utf8(h).ok())
                .and_then(|h| u8::from_str_radix(h, 16).ok())
                .ok_or_else(|| format!("malformed escape at byte {i}"))?;
            out.push(hex);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "decoded bytes are not valid UTF-8".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_cases() {
        assert_eq!(apply(Transform::Trim, "  a b \n").unwrap(), "a b");
        assert_eq!(apply(Transform::Lower, "ÄBc").unwrap(), "äbc");
        assert_eq!(apply(Transform::Upper, "straße").unwrap(), "STRASSE");
        assert_eq!(apply(Transform::Title, "hELLO  wORLD\nüber").unwrap(), "Hello  World\nÜber");
    }

    #[test]
    fn strip_breaks_joins_lines() {
        assert_eq!(apply(Transform::StripBreaks, "a\r\nb\n\n  c ").unwrap(), "a b c");
    }

    #[test]
    fn json_pretty_and_error() {
        assert_eq!(apply(Transform::JsonPretty, r#"{"a":[1,2]}"#).unwrap(), "{\n  \"a\": [\n    1,\n    2\n  ]\n}");
        assert!(apply(Transform::JsonPretty, "{nope").is_err());
    }

    #[test]
    fn url_roundtrip_and_malformed() {
        let src = "a b/ü?x=1&y=~";
        let enc = apply(Transform::UrlEncode, src).unwrap();
        assert_eq!(enc, "a%20b%2F%C3%BC%3Fx%3D1%26y%3D~");
        assert_eq!(apply(Transform::UrlDecode, &enc).unwrap(), src);
        assert!(apply(Transform::UrlDecode, "%zz").is_err());
        assert!(apply(Transform::UrlDecode, "%4").is_err());
        assert!(apply(Transform::UrlDecode, "%FF").is_err());
    }
}
