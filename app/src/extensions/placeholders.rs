use crate::icu::{self, Style};
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

pub static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)\{([a-z-]+)((?:\s+[a-z]+="[^"]*")*)\s*\}"#).unwrap());
static ATTRIBUTE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#"(?i)([a-z]+)="([^"]*)""#).unwrap());

pub fn attributes(text: &str) -> HashMap<String, String> {
    ATTRIBUTE.captures_iter(text).map(|c| (c[1].to_lowercase(), c[2].to_owned())).collect()
}

pub fn uuid() -> String {
    let mut b = [0u8; 16];
    let _ = getrandom::fill(&mut b);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

pub fn date_text(kind: &str, ms: f64, locale: &str) -> Option<String> {
    Some(match kind {
        "date" => icu::format_date(ms, locale, Style::Medium, Style::None, None),
        "time" => icu::format_date(ms, locale, Style::None, Style::Short, None),
        "datetime" => icu::format_date(ms, locale, Style::Medium, Style::Short, None),
        "day" => icu::format_pattern(ms, locale, "EEEE", None),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_attributes_and_makes_v4_uuids() {
        let caps = PLACEHOLDER.captures(r#"x {argument name="Query" default="1"} y"#).unwrap();
        assert_eq!(&caps[1], "argument");
        let attrs = attributes(&caps[2]);
        assert_eq!((attrs["name"].as_str(), attrs["default"].as_str()), ("Query", "1"));
        let id = uuid();
        assert_eq!((id.len(), &id[14..15]), (36, "4"));
    }
}
