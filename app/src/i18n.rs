use crate::store::Language;
use regex::Regex;
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, OnceLock};

static PT: AtomicBool = AtomicBool::new(false);

struct Dictionary {
    exact: HashMap<&'static str, &'static str>,
    templates: Vec<(Regex, Vec<&'static str>, &'static str)>,
}

static PORTUGUESE: LazyLock<Dictionary> = LazyLock::new(|| parse(include_str!("../assets/i18n/pt.json")));
static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{(\w+)\}").unwrap());

fn parse(json: &'static str) -> Dictionary {
    let map: HashMap<String, String> = serde_json::from_str(json).expect("valid pt.json");
    let mut exact = HashMap::new();
    let mut templates = Vec::new();
    for (key, value) in map {
        let (key, value): (&'static str, &'static str) = (Box::leak(key.into_boxed_str()), Box::leak(value.into_boxed_str()));
        if !PLACEHOLDER.is_match(key) {
            exact.insert(key, value);
            continue;
        }
        let mut pattern = String::from("^");
        let mut names = Vec::new();
        let mut last = 0;
        for caps in PLACEHOLDER.captures_iter(key) {
            let whole = caps.get(0).unwrap();
            pattern.push_str(&regex::escape(&key[last..whole.start()]));
            pattern.push_str("(.+?)");
            names.push(caps.get(1).unwrap().as_str());
            last = whole.end();
        }
        pattern.push_str(&regex::escape(&key[last..]));
        pattern.push('$');
        if let Ok(regex) = Regex::new(&pattern) {
            templates.push((regex, names, value));
        }
    }
    let literal = |key: &str| PLACEHOLDER.replace_all(key, "").chars().count();
    templates.sort_by_key(|(regex, _, _)| std::cmp::Reverse(literal(regex.as_str())));
    Dictionary { exact, templates }
}

fn system_is_portuguese() -> bool {
    static PT_SYSTEM: OnceLock<bool> = OnceLock::new();
    *PT_SYSTEM.get_or_init(|| crate::icu::display_locale().to_lowercase().starts_with("pt"))
}

pub fn sync(language: Language) {
    let pt = match language {
        Language::Pt => true,
        Language::En => false,
        Language::System => system_is_portuguese(),
    };
    PT.store(pt, Ordering::Relaxed);
}

pub fn is_portuguese() -> bool {
    PT.load(Ordering::Relaxed)
}

pub fn tr(text: &str) -> Cow<'_, str> {
    if !is_portuguese() || text.is_empty() {
        return Cow::Borrowed(text);
    }
    translate(&PORTUGUESE, text).map_or(Cow::Borrowed(text), Cow::Owned)
}

fn translate(dictionary: &Dictionary, text: &str) -> Option<String> {
    if let Some(found) = dictionary.exact.get(text) {
        return Some((*found).to_owned());
    }
    if text.len() > 160 || text.contains('\n') {
        return None;
    }
    thread_local! {
        static CACHE: RefCell<HashMap<String, Option<String>>> = RefCell::new(HashMap::new());
    }
    if let Some(hit) = CACHE.with(|c| c.borrow().get(text).cloned()) {
        return hit;
    }
    let result = dictionary.templates.iter().find_map(|(regex, names, target)| {
        let caps = regex.captures(text)?;
        let mut out = (*target).to_owned();
        for (i, name) in names.iter().enumerate() {
            let value = caps.get(i + 1)?.as_str();
            let value = dictionary.exact.get(value).copied().unwrap_or(value);
            out = out.replace(&format!("{{{name}}}"), value);
        }
        Some(out)
    });
    CACHE.with(|c| {
        let mut cache = c.borrow_mut();
        if cache.len() > 4096 {
            cache.clear();
        }
        cache.insert(text.to_owned(), result.clone());
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_exact_strings_and_templates() {
        let d = &*PORTUGUESE;
        assert_eq!(translate(d, "Suggestions").as_deref(), Some("Sugestões"));
        assert_eq!(translate(d, "Use “weather in paris” with...").as_deref(), Some("Usar “weather in paris” com..."));
        assert_eq!(translate(d, "Updated 5 minutes ago").as_deref(), Some("Atualizado há 5 minutos"));
        assert_eq!(translate(d, "My own clipboard text"), None);
    }
}
