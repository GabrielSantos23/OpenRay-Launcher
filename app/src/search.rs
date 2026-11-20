use std::collections::HashMap;

pub type Frecency = HashMap<String, f64>;

const SUGGESTION_LIMIT: usize = 8;

pub trait Searchable {
    fn key(&self) -> &str;
    fn title(&self) -> &str;
    fn keywords(&self) -> &[String];
    fn kind(&self) -> &str;
}

impl Searchable for &crate::extension::Command {
    fn key(&self) -> &str {
        &self.key
    }
    fn title(&self) -> &str {
        &self.title
    }
    fn keywords(&self) -> &[String] {
        &self.keywords
    }
    fn kind(&self) -> &str {
        self.kind
    }
}

fn words(text: &str) -> Vec<String> {
    let mut split = String::new();
    let mut prev_lower = false;
    for c in text.chars() {
        if prev_lower && c.is_uppercase() {
            split.push(' ');
        }
        prev_lower = c.is_lowercase();
        split.push(c);
    }
    split.to_lowercase().split(|c: char| c.is_whitespace() || "-_.()".contains(c)).filter(|w| !w.is_empty()).map(str::to_owned).collect()
}

fn is_subsequence(query: &str, text: &str) -> bool {
    let mut q = query.chars().peekable();
    for c in text.chars() {
        if q.peek() == Some(&c) {
            q.next();
        }
    }
    q.peek().is_none()
}

fn score_text(query: &str, text: &str) -> u32 {
    let lower = text.to_lowercase();
    if lower.starts_with(query) {
        return 100;
    }
    let parts = words(text);
    if parts.iter().any(|w| w.starts_with(query)) {
        return 80;
    }
    if parts.iter().filter_map(|w| w.chars().next()).collect::<String>().starts_with(query) {
        return 70;
    }
    if lower.contains(query) {
        return 50;
    }
    if is_subsequence(query, &lower) {
        return 20;
    }
    0
}

pub fn match_score(query: &str, title: &str, keywords: &[String]) -> u32 {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return 1;
    }
    let keyword = keywords.iter().map(|k| score_text(&q, k)).max().unwrap_or(0);
    let whole = score_text(&q, title).max(if keyword >= 50 { keyword - 30 } else { 0 });
    if whole > 0 {
        return whole;
    }
    let tokens: Vec<&str> = q.split_whitespace().collect();
    if tokens.len() < 2 {
        return 0;
    }
    let haystack: Vec<String> = std::iter::once(title).chain(keywords.iter().map(String::as_str)).flat_map(words).collect();
    if tokens.iter().all(|t| haystack.iter().any(|w| w.starts_with(t))) {
        60
    } else {
        0
    }
}

fn by_frecency_then_title<T: Searchable>(frecency: &Frecency) -> impl Fn(&&T, &&T) -> std::cmp::Ordering + '_ {
    move |a, b| {
        let fa = frecency.get(a.key()).copied().unwrap_or(0.0);
        let fb = frecency.get(b.key()).copied().unwrap_or(0.0);
        fb.total_cmp(&fa).then_with(|| a.title().to_lowercase().cmp(&b.title().to_lowercase()))
    }
}

pub struct Section<'a, T> {
    pub title: &'static str,
    pub items: Vec<&'a T>,
}

fn translated_score(query: &str, title: &str) -> u32 {
    match crate::i18n::tr(title) {
        std::borrow::Cow::Owned(shown) => match_score(query, &crate::extensions::calculator::math::unaccent(&shown), &[shown.clone()]),
        std::borrow::Cow::Borrowed(_) => 0,
    }
}

fn alias_score(query: &str, alias: Option<&String>) -> u32 {
    let q = query.trim().to_lowercase();
    match alias {
        Some(a) if !a.is_empty() && a.to_lowercase().starts_with(&q) => 110,
        _ => 0,
    }
}

pub fn search<'a, T: Searchable>(commands: &'a [T], query: &str, frecency: &Frecency, favorites: &[String], aliases: &HashMap<String, String>) -> Vec<Section<'a, T>> {
    let order = by_frecency_then_title::<T>(frecency);
    if query.trim().is_empty() {
        let mut used: Vec<&T> = commands.iter().filter(|c| !favorites.iter().any(|f| f == c.key()) && frecency.get(c.key()).copied().unwrap_or(0.0) > 0.0).collect();
        used.sort_by(&order);
        used.truncate(SUGGESTION_LIMIT);
        let pinned: Vec<&T> = favorites.iter().filter_map(|key| commands.iter().find(|c| c.key() == key)).collect();
        let shown = |c: &T| pinned.iter().chain(&used).any(|s| s.key() == c.key());
        let mut extension_commands: Vec<&T> = commands.iter().filter(|c| c.kind() == "Command" && !shown(c)).collect();
        extension_commands.sort_by(|a, b| a.title().to_lowercase().cmp(&b.title().to_lowercase()));
        let sections: Vec<Section<T>> = [("Favorites", pinned), ("Suggestions", used), ("Commands", extension_commands)]
            .into_iter()
            .filter(|(_, items)| !items.is_empty())
            .map(|(title, items)| Section { title, items })
            .collect();
        if !sections.is_empty() {
            return sections;
        }
        let mut all: Vec<&T> = commands.iter().collect();
        all.sort_by(&order);
        return vec![Section { title: "Results", items: all }];
    }
    let mut scored: Vec<(u32, &T)> = commands
        .iter()
        .map(|c| (alias_score(query, aliases.get(c.key())).max(match_score(query, c.title(), c.keywords())).max(translated_score(query, c.title())), c))
        .filter(|(score, _)| *score > 0)
        .collect();
    scored.sort_by(|(sa, a), (sb, b)| sb.cmp(sa).then_with(|| order(a, b)));
    let items: Vec<&T> = scored.into_iter().map(|(_, c)| c).collect();
    if items.is_empty() {
        Vec::new()
    } else {
        vec![Section { title: "Results", items }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Cmd {
        key: String,
        title: String,
        keywords: Vec<String>,
        kind: &'static str,
    }

    impl Searchable for Cmd {
        fn key(&self) -> &str {
            &self.key
        }
        fn title(&self) -> &str {
            &self.title
        }
        fn keywords(&self) -> &[String] {
            &self.keywords
        }
        fn kind(&self) -> &str {
            self.kind
        }
    }

    fn cmd(title: &str, keywords: &[&str]) -> Cmd {
        Cmd { key: format!("test/{title}"), title: title.into(), keywords: keywords.iter().map(|k| k.to_string()).collect(), kind: "Application" }
    }

    fn all() -> Vec<Cmd> {
        vec![cmd("Visual Studio Code", &[]), cmd("Visual Studio 2022", &[]), cmd("Notepad", &[]), cmd("Notion", &[]), cmd("Windows Terminal", &["wt", "console"])]
    }

    fn titles<T: Searchable>(section: &Section<T>) -> Vec<String> {
        section.items.iter().map(|c| c.title().to_string()).collect()
    }

    fn kw(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn match_score_ranks_match_kinds() {
        assert_eq!(match_score("vis", "Visual Studio Code", &[]), 100);
        assert_eq!(match_score("stu", "Visual Studio Code", &[]), 80);
        assert_eq!(match_score("vsc", "Visual Studio Code", &[]), 70);
        assert_eq!(match_score("sual", "Visual Studio Code", &[]), 50);
        assert_eq!(match_score("vlcd", "Visual Studio Code", &[]), 20);
        assert_eq!(match_score("xyz", "Visual Studio Code", &[]), 0);
        assert_eq!(match_score("  NOTE ", "Notepad", &[]), 100);
        assert_eq!(match_score("console", "Windows Terminal", &kw(&["wt", "console"])), 70);
        assert!(match_score("energia", "Configurações de energia", &[]) > match_score("energia", "Bluetooth", &kw(&["energia"])));
        assert_eq!(match_score("bx", "Volume", &kw(&["box"])), 0);
        assert_eq!(match_score("code vis", "Visual Studio Code", &[]), 60);
        assert_eq!(match_score("code xyz", "Visual Studio Code", &[]), 0);
        assert_eq!(match_score("store", "GitHubStore", &[]), 80);
        assert_eq!(match_score("", "Anything", &[]), 1);
    }

    #[test]
    fn filters_orders_and_uses_aliases_and_frecency() {
        let all = all();
        let none = HashMap::new();
        assert_eq!(titles(&search(&all, "no", &none, &[], &HashMap::new())[0]), ["Notepad", "Notion", "Windows Terminal"]);
        let aliases = HashMap::from([("test/Windows Terminal".to_string(), "nope".to_string())]);
        assert_eq!(titles(&search(&all, "no", &none, &[], &aliases)[0]), ["Windows Terminal", "Notepad", "Notion"]);
        let frecency = HashMap::from([("test/Notion".to_string(), 5.0)]);
        assert_eq!(titles(&search(&all, "no", &frecency, &[], &HashMap::new())[0]), ["Notion", "Notepad", "Windows Terminal"]);
        let frecency = HashMap::from([("test/Windows Terminal".to_string(), 100.0)]);
        assert!(search(&all, "vis", &frecency, &[], &HashMap::new())[0].items[0].title.starts_with("Visual"));
        assert!(search(&all, "zzzz", &none, &[], &HashMap::new()).is_empty());
    }

    #[test]
    fn empty_query_sections() {
        let all = all();
        let frecency = HashMap::from([("test/Notepad".to_string(), 2.0), ("test/Visual Studio 2022".to_string(), 9.0)]);
        let sections = search(&all, "", &frecency, &[], &HashMap::new());
        assert_eq!((sections.len(), sections[0].title), (1, "Suggestions"));
        assert_eq!(titles(&sections[0]), ["Visual Studio 2022", "Notepad"]);
        let favorites = ["test/Windows Terminal".to_string(), "test/Visual Studio 2022".to_string(), "gone/app".to_string()];
        let sections = search(&all, "", &frecency, &favorites, &HashMap::new());
        assert_eq!(sections.iter().map(|s| (s.title, titles(s))).collect::<Vec<_>>(), [("Favorites", vec!["Windows Terminal".to_string(), "Visual Studio 2022".to_string()]), ("Suggestions", vec!["Notepad".to_string()])]);
        let sections = search(&all, "", &HashMap::new(), &[], &HashMap::new());
        assert_eq!(sections[0].title, "Results");
        assert_eq!(titles(&sections[0]), ["Notepad", "Notion", "Visual Studio 2022", "Visual Studio Code", "Windows Terminal"]);
        let many: Vec<Cmd> = (0..12).map(|i| cmd(&format!("App {i}"), &[])).collect();
        let frecency: Frecency = many.iter().enumerate().map(|(i, c)| (c.key.clone(), i as f64 + 1.0)).collect();
        assert_eq!(search(&many, "", &frecency, &[], &HashMap::new())[0].items.len(), 8);
    }

    #[test]
    fn extension_commands_listed_below_suggestions() {
        let mut list = all();
        list.push(Cmd { kind: "Command", ..cmd("Settings", &[]) });
        list.push(Cmd { kind: "Command", ..cmd("Clipboard History", &[]) });
        let frecency = HashMap::from([("test/Notepad".to_string(), 2.0), ("test/Settings".to_string(), 1.0)]);
        let sections = search(&list, "", &frecency, &[], &HashMap::new());
        assert_eq!(sections.iter().map(|s| (s.title, titles(s))).collect::<Vec<_>>(), [("Suggestions", vec!["Notepad".to_string(), "Settings".to_string()]), ("Commands", vec!["Clipboard History".to_string()])]);
    }
}
