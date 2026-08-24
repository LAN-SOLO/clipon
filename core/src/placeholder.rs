//! Snippet placeholders, expanded when a snippet is copied:
//! `{date}`, `{date:FORMAT}` (chrono strftime), `{time}`, `{clipboard}`.
//! Unknown tokens and invalid formats are left as written.

use chrono::format::{Item, StrftimeItems};
use chrono::{DateTime, Local};

pub fn expand(text: &str, now: DateTime<Local>, clipboard: Option<&str>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('}') {
            None => {
                out.push_str(&rest[start..]);
                return out;
            }
            Some(end) => {
                let token = &after[..end];
                match resolve(token, now, clipboard) {
                    Some(v) => out.push_str(&v),
                    None => {
                        out.push('{');
                        out.push_str(token);
                        out.push('}');
                    }
                }
                rest = &after[end + 1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn resolve(token: &str, now: DateTime<Local>, clipboard: Option<&str>) -> Option<String> {
    match token {
        "date" => Some(now.format("%Y-%m-%d").to_string()),
        "time" => Some(now.format("%H:%M").to_string()),
        "clipboard" => Some(clipboard.unwrap_or_default().to_string()),
        _ => {
            let fmt = token.strip_prefix("date:")?;
            format_checked(now, fmt)
        }
    }
}

/// chrono panics when formatting an invalid pattern, so validate first.
fn format_checked(now: DateTime<Local>, fmt: &str) -> Option<String> {
    let items: Vec<Item> = StrftimeItems::new(fmt).collect();
    if fmt.is_empty() || items.iter().any(|i| matches!(i, Item::Error)) {
        return None;
    }
    Some(now.format_with_items(items.into_iter()).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn now() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 8, 24, 9, 5, 0).unwrap()
    }

    #[test]
    fn expands_known_tokens() {
        assert_eq!(expand("d={date} t={time}", now(), None), "d=2026-08-24 t=09:05");
        assert_eq!(expand("{date:%d.%m.%Y}", now(), None), "24.08.2026");
        assert_eq!(expand("cb: {clipboard}!", now(), Some("x")), "cb: x!");
        assert_eq!(expand("cb: {clipboard}!", now(), None), "cb: !");
    }

    #[test]
    fn leaves_unknown_and_invalid_literal() {
        assert_eq!(expand("{foo} {date:%Q} {date:} {", now(), None), "{foo} {date:%Q} {date:} {");
        assert_eq!(expand("no tokens", now(), None), "no tokens");
        assert_eq!(expand("}{date}{", now(), None), "}2026-08-24{");
    }
}
