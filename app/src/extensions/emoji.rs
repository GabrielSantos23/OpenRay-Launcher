use super::command;
use crate::extension::{frontmost_app, paste_title, Action, Command, Dropdown, Empty, Extension, Icon, Item, Section, Shortcut, Tone, View};
use crate::search::match_score;
use egui::Key;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;

const ICON: Icon = Icon::Tile("smile", Tone::Yellow);
const FREQUENT_LIMIT: usize = 16;
const CATEGORIES: [(&str, &str, &str); 8] = [
    ("smileys_people", "Smileys & People", "smile"),
    ("animals_nature", "Animals & Nature", "paw-print"),
    ("food_drink", "Food & Drink", "utensils-crossed"),
    ("activity", "Activity", "gamepad-2"),
    ("travel_places", "Travel & Places", "plane"),
    ("objects", "Objects", "lightbulb"),
    ("symbols", "Symbols", "hash"),
    ("flags", "Flags", "flag"),
];
static VERSION: AtomicU64 = AtomicU64::new(0);

pub struct Emoji {
    pub emoji: &'static str,
    pub name: &'static str,
    pub category: &'static str,
}

pub static CATALOG: LazyLock<Vec<Emoji>> = LazyLock::new(|| {
    include_str!("../../assets/emoji/emoji.tsv")
        .lines()
        .filter_map(|line| {
            let mut parts = line.split('\t');
            Some(Emoji { emoji: parts.next()?, name: parts.next()?, category: parts.next()? })
        })
        .collect()
});

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Usage {
    pub count: u64,
    pub last: u64,
}

fn path() -> std::path::PathBuf {
    crate::data_dir().join("emoji.json")
}

fn load() -> HashMap<String, Usage> {
    crate::store::read_json(&path())
}

fn record(emoji: &str) {
    let mut usage = load();
    let entry = usage.entry(emoji.to_owned()).or_insert(Usage { count: 0, last: 0 });
    entry.count += 1;
    entry.last = crate::store::now_ms();
    if crate::store::write_json(&path(), &usage).is_ok() {
        VERSION.fetch_add(1, Ordering::SeqCst);
    }
}

pub fn frequent(usage: &HashMap<String, Usage>, limit: usize) -> Vec<String> {
    let mut list: Vec<(&String, &Usage)> = usage.iter().collect();
    list.sort_by(|a, b| b.1.count.cmp(&a.1.count).then(b.1.last.cmp(&a.1.last)));
    list.into_iter().take(limit).map(|(e, _)| e.clone()).collect()
}

pub fn title_case(name: &str) -> String {
    let mut upper = true;
    name.chars()
        .map(|c| {
            let out: String = if upper && c.is_alphabetic() { c.to_uppercase().collect() } else { c.to_string() };
            upper = c == ' ' || c == '-';
            out
        })
        .collect()
}

struct SearchEmoji {
    query: String,
    filter: usize,
    usage: HashMap<String, Usage>,
    version: u64,
    app: Option<String>,
    cache: Option<((String, usize, u64), Vec<Section>)>,
}

impl SearchEmoji {
    fn item(&self, emoji: &Emoji, section: &str) -> Item {
        let (pasted, copied) = (emoji.emoji, emoji.emoji);
        Item {
            id: format!("{section}:{}", emoji.emoji),
            title: title_case(emoji.name),
            subtitle: None,
            icon: Icon::Emoji(emoji.emoji.into()),
            accessories: Vec::new(),
            keywords: vec![emoji.name.into()],
            actions: vec![vec![
                Action::new("paste", paste_title(&self.app), move |host| {
                    record(pasted);
                    host.paste(pasted)
                })
                .icon("clipboard-paste"),
                Action::new("copy", "Copy to Clipboard", move |host| {
                    record(copied);
                    host.copy_and_close(copied, "Copied Emoji to Clipboard");
                    Ok(())
                })
                .icon("clipboard")
                .shortcut(Shortcut::key(Key::Enter).ctrl()),
            ]],
        }
    }

    fn category(&self) -> Option<&'static str> {
        self.filter.checked_sub(2).map(|i| CATEGORIES[i].0)
    }
}

impl View for SearchEmoji {
    fn placeholder(&self) -> String {
        "Search Emoji & Symbols…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Search Emoji & Symbols".into(), ICON)
    }

    fn grid(&self) -> Option<usize> {
        Some(8)
    }

    fn filters(&self) -> bool {
        false
    }

    fn set_query(&mut self, query: &str) {
        self.query = query.to_owned();
    }

    fn empty(&self) -> Empty {
        Empty { title: "No emoji found".into(), description: None }
    }

    fn dropdown(&self) -> Option<Dropdown> {
        let options = [("All Categories", "layout-grid"), ("Frequently Used", "clock")].into_iter().chain(CATEGORIES.iter().map(|c| (c.1, c.2))).map(|(label, glyph)| (label.to_string(), Icon::Glyph(glyph))).collect();
        Some(Dropdown { label: "Filter by Category", options, selected: self.filter })
    }

    fn select(&mut self, index: usize) {
        self.filter = index;
    }

    fn sections(&mut self) -> Vec<Section> {
        let version = VERSION.load(Ordering::SeqCst);
        if version != self.version {
            self.version = version;
            self.usage = load();
        }
        let key = (self.query.clone(), self.filter, self.version);
        if let Some((_, sections)) = self.cache.as_ref().filter(|(cached, _)| *cached == key) {
            return sections.clone();
        }
        let sections = self.build();
        self.cache = Some((key, sections.clone()));
        sections
    }
}

impl SearchEmoji {
    fn build(&self) -> Vec<Section> {
        let category = self.category();
        if !self.query.trim().is_empty() {
            let mut scored: Vec<(u32, &Emoji)> = CATALOG.iter().filter(|e| category.is_none_or(|c| c == e.category)).map(|e| (match_score(&self.query, e.name, &[]), e)).filter(|m| m.0 > 0).collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            let items: Vec<Item> = scored.into_iter().map(|(_, e)| self.item(e, "results")).collect();
            return if items.is_empty() { Vec::new() } else { vec![Section { title: "Results".into(), items }] };
        }
        let by_emoji: HashMap<&str, &Emoji> = CATALOG.iter().map(|e| (e.emoji, e)).collect();
        let recent: Vec<Item> = frequent(&self.usage, FREQUENT_LIMIT).iter().filter_map(|e| by_emoji.get(e.as_str())).map(|e| self.item(e, "recent")).collect();
        if self.filter == 1 {
            return vec![Section { title: "Frequently Used".into(), items: recent }];
        }
        let mut sections = Vec::new();
        if self.filter == 0 && !recent.is_empty() {
            sections.push(Section { title: "Frequently Used".into(), items: recent });
        }
        for (value, label, _) in CATEGORIES.iter().filter(|c| category.is_none_or(|v| v == c.0)) {
            sections.push(Section { title: (*label).into(), items: CATALOG.iter().filter(|e| e.category == *value).map(|e| self.item(e, value)).collect() });
        }
        sections
    }
}

pub struct EmojiPicker;

impl Extension for EmojiPicker {
    fn id(&self) -> &'static str {
        "emoji-picker"
    }

    fn commands(&self) -> Vec<Command> {
        let mut search = command(self.id(), "searchEmoji", "Search Emoji & Symbols", ICON, "Command", "Open Command", |_| Ok(()));
        search.subtitle = Some("Emoji & Symbols".into());
        search.keywords = ["emoji", "symbols", "smiley", "emojis", "símbolos"].map(String::from).to_vec();
        search.view = Some(Rc::new(|_| Box::new(SearchEmoji { query: String::new(), filter: 0, usage: HashMap::new(), version: u64::MAX, app: frontmost_app(), cache: None })));
        vec![search]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_the_catalog_into_known_categories() {
        assert!(CATALOG.len() > 1800);
        assert!(CATALOG.iter().all(|e| CATEGORIES.iter().any(|c| c.0 == e.category)));
        assert_eq!(CATALOG[0].emoji, "😀");
    }

    #[test]
    fn orders_frequent_by_count_then_recency() {
        let usage = HashMap::from([("a".to_string(), Usage { count: 1, last: 5 }), ("b".to_string(), Usage { count: 3, last: 1 }), ("c".to_string(), Usage { count: 1, last: 9 })]);
        assert_eq!(frequent(&usage, 2), ["b", "c"]);
    }

    #[test]
    fn title_cases_words_and_hyphenated_parts() {
        assert_eq!(title_case("grinning face"), "Grinning Face");
        assert_eq!(title_case("t-rex"), "T-Rex");
        assert_eq!(title_case("flag: Brazil"), "Flag: Brazil");
    }
}
