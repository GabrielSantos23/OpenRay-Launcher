use super::command;
use crate::contacts::{Labeled, Person};
use crate::extension::{Action, Command, Empty, Extension, Host, Icon, Item, Load, Section, Shortcut, Tone, View};
use crate::theme;
use egui::{pos2, vec2, Key, Rect, UiBuilder};
use std::rc::Rc;
use std::sync::Arc;
use windows::Win32::Globalization::{NormalizeString, NormalizationD};

const ICON: Icon = Icon::Tile("user", Tone::Slate);

pub fn fold(text: &str) -> String {
    let src: Vec<u16> = text.encode_utf16().collect();
    let decomposed = (|| unsafe {
        let n = NormalizeString(NormalizationD, &src, None);
        if n <= 0 {
            return None;
        }
        let mut buffer = vec![0u16; n as usize];
        let n = NormalizeString(NormalizationD, &src, Some(&mut buffer));
        (n > 0).then(|| String::from_utf16_lossy(&buffer[..n as usize]))
    })();
    decomposed.unwrap_or_else(|| text.to_owned()).chars().filter(|c| !('\u{300}'..='\u{36f}').contains(c)).collect::<String>().to_lowercase()
}

pub fn display_name(person: &Person) -> String {
    [&person.name, &person.organization].into_iter().find(|s| !s.is_empty()).cloned().unwrap_or_else(|| "No Name".into())
}

pub fn initials(person: &Person) -> String {
    let source = if person.name.is_empty() { &person.organization } else { &person.name };
    let words: Vec<&str> = source.split_whitespace().collect();
    let first = |w: &str| w.chars().next().map(String::from).unwrap_or_default();
    match words.as_slice() {
        [] => String::new(),
        [one] => first(one),
        [head, .., last] => first(head) + &first(last),
    }
    .to_uppercase()
}

fn digits(text: &str) -> String {
    text.chars().filter(char::is_ascii_digit).collect()
}

fn phone_matches(phone: &str, term: &str) -> bool {
    let wanted = digits(term);
    if wanted.is_empty() {
        return false;
    }
    let have = digits(phone);
    have.contains(&wanted) || (wanted.starts_with('0') && wanted.len() > 1 && have.contains(&wanted[1..]))
}

pub fn matches(person: &Person, query: &str) -> bool {
    let folded = fold(query.trim());
    let (name, organization) = (fold(&person.name), fold(&person.organization));
    let emails: Vec<String> = person.email_addresses.iter().map(|e| fold(&e.value)).collect();
    let addresses: Vec<String> = person.postal_addresses.iter().map(|a| fold(&a.value)).collect();
    folded.split_whitespace().all(|t| name.contains(t) || organization.contains(t) || emails.iter().any(|e| e.contains(t)) || addresses.iter().any(|a| a.contains(t)) || person.phone_numbers.iter().any(|p| phone_matches(&p.value, t)))
}

pub fn sections_for<'a>(people: &'a [Person], query: &str) -> Vec<(String, Vec<&'a Person>)> {
    let mut groups: Vec<(String, Vec<&Person>)> = Vec::new();
    for person in people.iter().filter(|p| matches(p, query)) {
        let source = if !person.family_name.is_empty() { person.family_name.clone() } else { person.name.split(' ').next_back().filter(|w| !w.is_empty()).map(String::from).unwrap_or_else(|| person.organization.clone()) };
        let key = fold(&source).chars().next().map(|c| c.to_ascii_uppercase()).filter(char::is_ascii_uppercase).map_or("#".to_string(), String::from);
        match groups.iter_mut().find(|g| g.0 == key) {
            Some(group) => group.1.push(person),
            None => groups.push((key, vec![person])),
        }
    }
    groups.sort_by(|a, b| (a.0 == "#").cmp(&(b.0 == "#")).then(a.0.cmp(&b.0)));
    for group in &mut groups {
        group.1.sort_by_key(|p| display_name(p).to_lowercase());
    }
    groups
}

fn multi(items: &[Labeled], title: &'static str, id: &'static str, icon: &'static str, shortcut: Shortcut, run: impl Fn(&str, &mut Host) -> Result<(), String> + 'static) -> Vec<Action> {
    let run = Rc::new(run);
    match items {
        [] => Vec::new(),
        [one] => {
            let value = one.value.clone();
            vec![Action::new(id, title, move |host| run(&value, host)).icon(icon).shortcut(shortcut)]
        }
        many => {
            let options: Vec<(String, &'static str)> = many.iter().map(|i| (i.value.clone(), i.label)).collect();
            vec![Action::new(id, format!("{title}…"), |_| Ok(())).icon(icon).shortcut(shortcut).keep_open().submenu(move || {
                options
                    .iter()
                    .map(|(value, label)| {
                        let (run, value) = (run.clone(), value.clone());
                        Action::new(id, value.clone(), move |host| run(&value, host)).icon(icon).accessory(*label)
                    })
                    .collect()
            })]
        }
    }
}

fn open(scheme: &'static str) -> impl Fn(&str, &mut Host) -> Result<(), String> {
    move |value, _| {
        let target = if scheme == "tel" { value.chars().filter(|c| c.is_ascii_digit() || *c == '+').collect() } else { value.to_owned() };
        crate::apps::open_url(&format!("{scheme}:{target}"), None)
    }
}

fn copied(what: &'static str) -> impl Fn(&str, &mut Host) -> Result<(), String> {
    move |value, host| {
        host.copy_and_close(value, &format!("Copied {what}"));
        Ok(())
    }
}

fn person_actions(person: &Person) -> Vec<Vec<Action>> {
    let mut first = multi(&person.email_addresses, "Send Email", "send-email", "mail", Shortcut::key(Key::E).ctrl().shift(), open("mailto"));
    first.extend(multi(&person.phone_numbers, "Call via Phone", "call-via-phone", "phone", Shortcut::key(Key::P).ctrl().shift(), open("tel")));
    let mut second = Vec::new();
    if !person.name.is_empty() {
        let name = person.name.clone();
        second.push(Action::new("copy-name", "Copy Full Name", move |host| copied("name")(&name, host)).icon("clipboard").shortcut(Shortcut::key(Key::C).ctrl().shift()));
    }
    second.extend(multi(&person.phone_numbers, "Copy Phone Number", "copy-phone-number", "phone", Shortcut::key(Key::C).ctrl().alt(), copied("phone number")));
    second.extend(multi(&person.email_addresses, "Copy Email Address", "copy-email-address", "mail", Shortcut::key(Key::C).alt().shift(), copied("email address")));
    second.extend(multi(&person.postal_addresses, "Copy Postal Address", "copy-postal-address", "map-pin", Shortcut::key(Key::C).ctrl().alt().shift(), copied("postal address")));
    vec![first, second]
}

struct SearchContacts {
    load: Load<Arc<Vec<Person>>>,
    query: String,
}

impl SearchContacts {
    fn people(&self) -> Option<Result<Arc<Vec<Person>>, String>> {
        self.load.get()
    }
}

impl View for SearchContacts {
    fn placeholder(&self) -> String {
        "Search contacts…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Search Contacts".into(), ICON)
    }

    fn filters(&self) -> bool {
        false
    }

    fn set_query(&mut self, query: &str) {
        self.query = query.to_owned();
    }

    fn loading(&self) -> bool {
        self.people().is_none()
    }

    fn has_detail(&self) -> bool {
        self.people().is_some_and(|p| p.is_ok_and(|list| !sections_for(&list, &self.query).is_empty()))
    }

    fn empty(&self) -> Empty {
        match self.people() {
            Some(Err(e)) => Empty { title: "Couldn't read your contacts".into(), description: Some(e) },
            Some(Ok(list)) if list.is_empty() => Empty { title: "No contacts found".into(), description: Some("Windows has no contacts yet. Add an account with contacts in Settings > Accounts > Email & accounts.".into()) },
            _ => Empty { title: "No Contacts Found".into(), description: Some("Try another name, email or phone number.".into()) },
        }
    }

    fn detail(&mut self, ui: &mut egui::Ui, rect: Rect, id: &str) {
        let Some(Ok(people)) = self.people() else { return };
        let Some(person) = people.iter().find(|p| p.id == id) else { return };
        let rows: Vec<(String, String)> = person.phone_numbers.iter().chain(&person.email_addresses).chain(&person.postal_addresses).map(|l| (l.label.to_string(), l.value.clone())).collect();
        let top = if rows.is_empty() { rect.bottom() } else { crate::root::metadata(ui, rect, &rows).top() };
        let area = Rect::from_min_max(rect.min, pos2(rect.right(), top));
        let name = display_name(person);
        let subtitle = (!person.organization.is_empty() && !person.name.is_empty()).then(|| person.organization.clone());
        let block = 96.0 + 12.0 + 20.0 + if subtitle.is_some() { 20.0 } else { 0.0 };
        let avatar = Rect::from_center_size(pos2(area.center().x, area.center().y - block / 2.0 + 48.0), vec2(96.0, 96.0));
        crate::icons::command_icon(&mut ui.new_child(UiBuilder::new().max_rect(avatar)), &Icon::Initials(initials(person)), &name, 96.0);
        let title = ui.painter().text(pos2(area.center().x, avatar.bottom() + 12.0), egui::Align2::CENTER_TOP, &name, theme::weighted(15.0, theme::SEMIBOLD), theme::FG);
        if let Some(org) = subtitle {
            ui.painter().text(pos2(area.center().x, title.bottom() + 2.0), egui::Align2::CENTER_TOP, org, theme::font(13.0), theme::FG_60);
        }
    }

    fn sections(&mut self) -> Vec<Section> {
        let Some(Ok(people)) = self.people() else { return Vec::new() };
        sections_for(&people, &self.query)
            .into_iter()
            .map(|(title, list)| Section {
                title,
                items: list
                    .into_iter()
                    .map(|p| Item {
                        id: p.id.clone(),
                        title: display_name(p),
                        subtitle: Some(p.organization.clone()).filter(|o| !o.is_empty() && !p.name.is_empty()),
                        icon: Icon::Initials(initials(p)),
                        accessories: Vec::new(),
                        keywords: Vec::new(),
                        actions: person_actions(p),
                    })
                    .collect(),
            })
            .collect()
    }
}

pub struct Contacts;

impl Extension for Contacts {
    fn id(&self) -> &'static str {
        "contacts"
    }

    fn commands(&self) -> Vec<Command> {
        let mut search = command(self.id(), "search", "Search Contacts", ICON, "Command", "Open Command", |_| Ok(()));
        search.subtitle = Some("Contacts".into());
        search.keywords = ["contacts", "people", "phone", "email", "address book", "contatos", "pessoas", "agenda"].map(String::from).to_vec();
        search.view = Some(Rc::new(|ctx| Box::new(SearchContacts { load: Load::start(ctx, || crate::contacts::all().map(Arc::new).map_err(|e| e.message())), query: String::new() })));
        vec![search]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn person(id: &str, name: &str, family: &str, organization: &str, phone: &str, email: &str) -> Person {
        let labeled = |label: &'static str, value: &str| if value.is_empty() { Vec::new() } else { vec![Labeled { label, value: value.into() }] };
        Person { id: id.into(), name: name.into(), given_name: String::new(), family_name: family.into(), organization: organization.into(), phone_numbers: labeled("Mobile", phone), email_addresses: labeled("Email", email), postal_addresses: Vec::new() }
    }

    #[test]
    fn names_and_initials() {
        assert_eq!(initials(&person("1", "Ana Maria Souza", "", "", "", "")), "AS");
        assert_eq!(initials(&person("1", "", "", "acme", "", "")), "A");
        assert_eq!(display_name(&person("1", "", "", "", "", "")), "No Name");
    }

    #[test]
    fn matches_accent_insensitive_names_emails_and_phones() {
        let p = person("1", "José Álvares", "Álvares", "", "+55 (11) 98765-4321", "jose@example.com");
        assert!(matches(&p, "jose alv"));
        assert!(matches(&p, "example"));
        assert!(matches(&p, "98765"));
        assert!(matches(&p, "011"));
        assert!(!matches(&p, "maria"));
    }

    #[test]
    fn groups_by_family_name_letter_with_symbols_last() {
        let people = [person("1", "Zed Alpha", "", "", "", ""), person("2", "Bob Brown", "Brown", "", "", ""), person("3", "", "", "123 Corp", "", ""), person("4", "Amy Ávila", "Ávila", "", "", "")];
        let groups = sections_for(&people, "");
        let titles: Vec<(&str, Vec<&str>)> = groups.iter().map(|(t, l)| (t.as_str(), l.iter().map(|p| p.id.as_str()).collect())).collect();
        assert_eq!(titles, [("A", vec!["4", "1"]), ("B", vec!["2"]), ("#", vec!["3"])]);
    }
}
