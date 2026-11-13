use super::command;
use crate::extension::{Command, Extension, Icon, Query};
use std::rc::Rc;

const FILE_EXTENSIONS: &[&str] = &["exe", "msc", "cpl", "bat", "cmd", "ps1", "lnk", "txt", "json", "js", "ts", "tsx", "py", "rs", "png", "jpg", "jpeg", "gif", "pdf", "docx", "xlsx", "zip", "dll", "log", "ini", "csv", "html"];

fn is_ipv4(host: &str) -> bool {
    let parts: Vec<&str> = host.split('.').collect();
    parts.len() == 4 && parts.iter().all(|p| (1..=3).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_digit()))
}

fn is_domain(host: &str) -> bool {
    let labels: Vec<&str> = host.split('.').collect();
    let Some((tld, rest)) = labels.split_last() else { return false };
    !rest.is_empty()
        && (2..=63).contains(&tld.len())
        && tld.bytes().all(|b| b.is_ascii_alphabetic())
        && rest.iter().all(|l| !l.is_empty() && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
}

pub fn to_url(text: &str) -> Option<String> {
    let value = text.trim();
    if value.is_empty() || value.contains(char::is_whitespace) {
        return None;
    }
    if let Some((scheme, rest)) = value.split_once("://") {
        let valid_scheme = scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c));
        let host = rest.split(['/', '?', '#']).next().unwrap_or("");
        return (valid_scheme && (!host.is_empty() || scheme.eq_ignore_ascii_case("file"))).then(|| value.to_owned());
    }
    let end = value.find(['/', '?', '#']).unwrap_or(value.len());
    let (authority, _) = value.split_at(end);
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => (host, Some(port)),
        None => (authority, None),
    };
    if port.is_some_and(|p| p.is_empty() || p.len() > 5 || !p.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let local = host.eq_ignore_ascii_case("localhost") || is_ipv4(host);
    if !local && !is_domain(host) {
        return None;
    }
    let extension = host.rsplit('.').next().unwrap_or("").to_lowercase();
    if !value.contains('/') && FILE_EXTENSIONS.contains(&extension.as_str()) {
        return None;
    }
    Some(format!("{}://{value}", if local { "http" } else { "https" }))
}

pub struct Urls;

impl Extension for Urls {
    fn id(&self) -> &'static str {
        "urls"
    }

    fn commands(&self) -> Vec<Command> {
        let mut cmd = command(self.id(), "quick-open", "Open in Browser", Icon::Tile("globe", crate::extension::Tone::Blue), "Command", "Open in Browser", |_| Ok(()));
        cmd.keywords = ["url", "site", "abrir"].map(String::from).to_vec();
        cmd.query = Some(Query {
            matches: |text| to_url(text).is_some(),
            run: Rc::new(|text, _| crate::apps::open_url(&to_url(text).ok_or("not a link")?, None)),
        });
        vec![cmd]
    }
}

const ENGINES: &[(&str, &str, &str, &str, &str)] = &[
    ("google", "Google Search", "https://www.google.com", "https://www.google.com/search?q=", "google.com"),
    ("duckDuckGo", "DuckDuckGo Search", "https://duckduckgo.com", "https://duckduckgo.com/?q=", "duckduckgo.com"),
    ("bing", "Bing", "https://www.bing.com", "https://www.bing.com/search?q=", "bing.com"),
];

pub fn encode_query(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'!' | b'~' | b'*' | b'\'' | b'(' | b')' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub struct WebSearches;

impl Extension for WebSearches {
    fn id(&self) -> &'static str {
        "web-searches"
    }

    fn commands(&self) -> Vec<Command> {
        ENGINES
            .iter()
            .map(|&(id, title, home, search, domain)| {
                let icon = Icon::Url(format!("https://www.google.com/s2/favicons?domain={domain}&sz=64"));
                let mut cmd = command(self.id(), id, title, icon, "Command", "Open Command", move |_| crate::apps::open_url(home, None));
                cmd.subtitle = Some("Web Searches".into());
                cmd.keywords = ["search", "web", "pesquisa", "busca na web"].map(String::from).to_vec();
                cmd.fallback = Some(Rc::new(move |text, _| crate::apps::open_url(&format!("{search}{}", encode_query(text)), None)));
                cmd
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_links_like_the_web_version() {
        assert_eq!(to_url("https://raycast.com/store").as_deref(), Some("https://raycast.com/store"));
        assert_eq!(to_url("ftp://files.example.org").as_deref(), Some("ftp://files.example.org"));
        assert_eq!(to_url("github.com/ShareX/ShareX").as_deref(), Some("https://github.com/ShareX/ShareX"));
        assert_eq!(to_url("localhost:1420").as_deref(), Some("http://localhost:1420"));
        assert_eq!(to_url("192.168.0.1/admin").as_deref(), Some("http://192.168.0.1/admin"));
        for text in ["hello world", "notepad", "calc.exe", "notes.txt", "3.14", "1+2", "http://"] {
            assert_eq!(to_url(text), None, "{text}");
        }
    }

    #[test]
    fn encodes_queries_like_encode_uri_component() {
        assert_eq!(encode_query("rust egui & más"), "rust%20egui%20%26%20m%C3%A1s");
    }
}
