use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Grants {
    pub native_methods: Vec<String>,
    pub http_hosts: Vec<String>,
    pub oauth: Vec<String>,
    pub fs_paths: Vec<String>,
    pub uri_schemes: Vec<String>,
}

impl Grants {
    fn lists(&self) -> [&Vec<String>; 5] {
        [&self.native_methods, &self.http_hosts, &self.oauth, &self.fs_paths, &self.uri_schemes]
    }

    fn zip(&self, other: Option<&Grants>, keep: bool) -> Grants {
        let pick = |mine: &Vec<String>, theirs: Option<&Vec<String>>| mine.iter().filter(|v| theirs.is_some_and(|t| t.contains(v)) == keep).cloned().collect();
        Grants {
            native_methods: pick(&self.native_methods, other.map(|o| &o.native_methods)),
            http_hosts: pick(&self.http_hosts, other.map(|o| &o.http_hosts)),
            oauth: pick(&self.oauth, other.map(|o| &o.oauth)),
            fs_paths: pick(&self.fs_paths, other.map(|o| &o.fs_paths)),
            uri_schemes: pick(&self.uri_schemes, other.map(|o| &o.uri_schemes)),
        }
    }

    pub fn of(manifest: &Value) -> Grants {
        serde_json::from_value(manifest.clone()).unwrap_or_default()
    }

    pub fn intersect(&self, consented: Option<&Grants>) -> Grants {
        self.zip(consented, true)
    }

    pub fn missing(&self, consented: Option<&Grants>) -> Grants {
        self.zip(consented, false)
    }

    pub fn is_empty(&self) -> bool {
        self.lists().iter().all(|l| l.is_empty())
    }

    pub fn describe(&self) -> Vec<String> {
        let capital = |p: &str| p.chars().next().map(|c| c.to_uppercase().collect::<String>() + &p[c.len_utf8()..]).unwrap_or_default();
        self.native_methods
            .iter()
            .map(|m| format!("Use {m}"))
            .chain(self.http_hosts.iter().map(|h| format!("Connect to {}", if h.starts_with('.') { format!("*{h}") } else { h.clone() })))
            .chain(self.oauth.iter().map(|p| format!("Sign in to {}", capital(p))))
            .chain(self.fs_paths.iter().map(|p| format!("Files in {p}")))
            .chain(self.uri_schemes.iter().map(|s| format!("Open {s}: links in their app")))
            .collect()
    }
}

pub fn method_allowed(method: &str, declared: &[String]) -> bool {
    declared.iter().any(|entry| entry == method || entry.strip_suffix('*').is_some_and(|prefix| prefix.ends_with('.') && method.starts_with(prefix) && !method[prefix.len()..].contains('.')))
}

pub fn scheme_allowed(url: &str, schemes: &[String]) -> bool {
    let Some((scheme, rest)) = url.split_once(':') else { return false };
    let valid = scheme.chars().next().is_some_and(|c| c.is_ascii_alphabetic()) && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c));
    if !valid {
        return false;
    }
    let scheme = scheme.to_ascii_lowercase();
    if scheme == "http" || scheme == "https" {
        return rest.starts_with("//") && rest.len() > 2 && !rest[2..].starts_with('/');
    }
    schemes.iter().any(|s| s.eq_ignore_ascii_case(&scheme))
}

fn normalize(path: &str) -> String {
    let mut out = String::new();
    for c in path.chars() {
        let c = if c == '\\' { '/' } else { c };
        if !(c == '/' && out.ends_with('/')) {
            out.push(c);
        }
    }
    out.to_lowercase()
}

fn glob_regex(glob: &str) -> regex::Regex {
    let mut source = String::from("^");
    let chars: Vec<char> = glob.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            '*' if chars.get(i + 1) == Some(&'*') => {
                if chars.get(i + 2) == Some(&'/') {
                    source.push_str("(?:.*/)?");
                    i += 2;
                } else {
                    source.push_str(".*");
                    i += 1;
                }
            }
            '*' => source.push_str("[^/]*"),
            '?' => source.push_str("[^/]"),
            c => source.push_str(&regex::escape(&c.to_string())),
        }
        i += 1;
    }
    source.push('$');
    regex::Regex::new(&source).unwrap_or_else(|_| regex::Regex::new("$^").expect("empty regex"))
}

pub fn interpolate(scope: &str, preferences: &Map<String, Value>) -> Option<String> {
    let pattern = regex::Regex::new(r"\{\{([a-zA-Z_][a-zA-Z0-9_]*)\}\}").expect("placeholder regex");
    let mut missing = false;
    let value = pattern.replace_all(scope, |caps: &regex::Captures| match preferences.get(&caps[1]).and_then(Value::as_str) {
        Some(text) if !text.trim().is_empty() => text.trim_end_matches(['\\', '/']).to_owned(),
        _ => {
            missing = true;
            String::new()
        }
    });
    (!missing).then(|| value.into_owned())
}

fn absolute_safe(path: &str) -> bool {
    let normalized = normalize(path);
    let bytes = normalized.as_bytes();
    let absolute = (bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/') || normalized.starts_with("//");
    absolute && !normalized.split('/').any(|s| s == "..")
}

pub fn path_allowed(path: &str, scopes: &[String], preferences: &Map<String, Value>) -> bool {
    if !absolute_safe(path) {
        return false;
    }
    let target = normalize(path);
    scopes.iter().any(|scope| {
        let Some(resolved) = interpolate(scope, preferences) else { return false };
        let glob = normalize(&resolved);
        glob_regex(&glob).is_match(&target) || (!glob.contains(['*', '?']) && glob_regex(&format!("{}/**", glob.trim_end_matches('/'))).is_match(&target))
    })
}

pub fn scope_root(scope: &str, preferences: &Map<String, Value>) -> Option<String> {
    let resolved = interpolate(scope, preferences)?.replace('\\', "/");
    let parts: Vec<&str> = resolved.split('/').collect();
    let end = parts.iter().position(|p| p.contains(['*', '?'])).unwrap_or(parts.len());
    Some(parts[..end].join("/"))
}

pub fn glob_matches(pattern: &str, path: &str) -> bool {
    glob_regex(&normalize(pattern)).is_match(&normalize(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn matches_methods_with_single_level_wildcards() {
        let declared = strings(&["storage.*", "http.fetch"]);
        assert!(method_allowed("storage.get", &declared));
        assert!(method_allowed("http.fetch", &declared));
        assert!(!method_allowed("storage.deep.get", &declared));
        assert!(!method_allowed("http.upload", &declared));
        assert!(!method_allowed("storagex.get", &declared));
    }

    #[test]
    fn allows_web_links_and_declared_schemes_only() {
        let schemes = strings(&["obsidian"]);
        assert!(scheme_allowed("https://example.com/x", &schemes));
        assert!(scheme_allowed("obsidian://open?vault=a", &schemes));
        assert!(!scheme_allowed("https:///etc", &schemes));
        assert!(!scheme_allowed("file:///C:/x", &schemes));
        assert!(!scheme_allowed("javascript:alert(1)", &schemes));
    }

    #[test]
    fn checks_paths_against_interpolated_scopes() {
        let prefs = json!({ "vault": "C:\\Notes\\" }).as_object().unwrap().clone();
        let scopes = strings(&["{{vault}}/**/*.md", "D:/Exact"]);
        assert!(path_allowed("C:\\Notes\\a\\b.md", &scopes, &prefs));
        assert!(path_allowed("c:/notes/top.md", &scopes, &prefs));
        assert!(!path_allowed("C:\\Notes\\a\\b.txt", &scopes, &prefs));
        assert!(!path_allowed("C:\\Notes\\..\\secret.md", &scopes, &prefs));
        assert!(path_allowed("D:\\Exact\\deep\\file.bin", &scopes, &prefs));
        assert!(!path_allowed("relative/x.md", &scopes, &prefs));
        assert!(!path_allowed("C:/Notes/a.md", &strings(&["{{missing}}/**"]), &prefs));
        assert_eq!(scope_root("{{vault}}/**/*.md", &prefs).as_deref(), Some("C:/Notes"));
        assert!(glob_matches("C:/Notes/**/*.md", "C:\\Notes\\x\\y.md"));
    }

    #[test]
    fn intersects_and_describes_grants() {
        let declared = Grants::of(&json!({ "nativeMethods": ["http.fetch", "storage.*"], "httpHosts": [".scdn.co"], "oauth": ["spotify"] }));
        let consented = Grants { native_methods: strings(&["http.fetch"]), ..Default::default() };
        assert_eq!(declared.intersect(Some(&consented)).native_methods, strings(&["http.fetch"]));
        assert!(declared.intersect(None).is_empty());
        let missing = declared.missing(Some(&consented));
        assert_eq!(missing.native_methods, strings(&["storage.*"]));
        assert_eq!(missing.describe(), strings(&["Use storage.*", "Connect to *.scdn.co", "Sign in to Spotify"]));
    }
}
