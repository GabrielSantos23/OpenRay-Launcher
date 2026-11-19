use super::command;
use crate::calendar::{Calendar, Event, NewEvent};
use crate::extension::{Action, Command, Dropdown, Empty, Extension, Field, FieldKind, Form, Host, Icon, Item, Load, Request, Section, Shortcut, Toast, Tone, View, ViewFactory};
use crate::icu;
use crate::theme;
use egui::{pos2, Color32, Key, Rect};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const ICON: Icon = Icon::Tile("calendar-days", Tone::Red);
const DAY: i64 = 86_400_000;
const HOUR: i64 = 3_600_000;
const SCHEDULE_DAYS: i64 = 30;
const DURATIONS: [(i64, &str); 6] = [(15, "15 minutes"), (30, "30 minutes"), (60, "1 hour"), (90, "1 hour 30 minutes"), (120, "2 hours"), (240, "4 hours")];
static VERSION: AtomicU64 = AtomicU64::new(0);

fn offset(ms: i64) -> i64 {
    icu::local_offset_minutes(ms as f64) as i64 * 60_000
}

fn from_local(local: i64) -> i64 {
    let first = local - offset(local);
    local - offset(first)
}

pub fn local_day(ms: i64) -> i64 {
    (ms + offset(ms)).div_euclid(DAY)
}

fn day_start(day: i64) -> i64 {
    from_local(day * DAY)
}

fn week_start(day: i64) -> i64 {
    day - (day + 3).rem_euclid(7)
}

fn civil(day: i64) -> (i64, i64, i64) {
    super::calculator::history::civil_from_days(day)
}

pub struct EventSection<'a> {
    pub key: (u8, i64),
    pub title: String,
    pub subtitle: Option<String>,
    pub events: Vec<&'a Event>,
}

pub fn sections_for<'a>(events: &'a [Event], now: i64, locale: &str) -> Vec<EventSection<'a>> {
    let today = local_day(now);
    let today_start = day_start(today);
    let next_week = week_start(today) + 7;
    let week_after = next_week + 7;
    let (now_year, now_month, _) = civil(today);
    let mut sections: Vec<EventSection> = Vec::new();
    for event in events {
        if event.end <= today_start && !(event.all_day && event.start >= today_start) {
            continue;
        }
        let day = local_day(event.start).max(today);
        let at = day_start(day) as f64;
        let (key, title, subtitle) = if day < next_week {
            let title = match day - today {
                0 => "Today".to_owned(),
                1 => "Tomorrow".to_owned(),
                _ => icu::format_skeleton(at, locale, "EEEE", None),
            };
            ((0, day), title, Some(icu::format_skeleton(at, locale, "dMMM", None)))
        } else if day < week_after {
            ((1, 0), "Next Week".to_owned(), None)
        } else {
            let (year, month, _) = civil(day);
            let name = icu::format_skeleton(at, locale, "MMMM", None);
            ((2, year * 100 + month), if (year, month) == (now_year, now_month) { format!("Rest of {name}") } else { name }, None)
        };
        match sections.iter_mut().find(|s| s.key == key) {
            Some(section) => section.events.push(event),
            None => sections.push(EventSection { key, title, subtitle, events: vec![event] }),
        }
    }
    sections.sort_by_key(|s| s.key);
    for section in &mut sections {
        section.events.sort_by_key(|e| e.start);
    }
    sections
}

fn clock(ms: i64, locale: &str) -> String {
    icu::format_skeleton(ms as f64, locale, "jjmm", None)
}

pub fn time_label(event: &Event, now: i64, locale: &str) -> String {
    let week = week_start(local_day(now));
    if event.start < day_start(week + 7) {
        return if event.all_day { "All day".into() } else { format!("{} – {}", clock(event.start, locale), clock(event.end, locale)) };
    }
    if event.start < day_start(week + 14) {
        return icu::format_skeleton(event.start as f64, locale, "EEEd", None);
    }
    let same_year = civil(local_day(event.start)).0 == civil(local_day(now)).0;
    icu::format_skeleton(event.start as f64, locale, if same_year { "dMMM" } else { "yMMMd" }, None)
}

pub fn format_duration(ms: i64) -> String {
    let minutes = (ms as f64 / 60_000.0).round() as i64;
    let (hours, rest) = (minutes / 60, minutes % 60);
    let parts: Vec<String> = [(hours, "hour"), (rest, "minute")].iter().filter(|(n, _)| *n != 0).map(|(n, unit)| format!("{n} {unit}{}", if *n == 1 { "" } else { "s" })).collect();
    if parts.is_empty() {
        "0 minutes".into()
    } else {
        parts.join(" ")
    }
}

pub fn when_text(event: &Event, locale: &str) -> String {
    let date = |ms: i64| icu::format_skeleton(ms as f64, locale, "yMMMd", None);
    if event.all_day {
        let last = event.end - 1;
        return if local_day(event.start) == local_day(last) { format!("{} (All day)", date(event.start)) } else { format!("{} – {}", date(event.start), date(last)) };
    }
    let duration = format!("(Duration: {})", format_duration(event.end - event.start));
    if local_day(event.start) == local_day(event.end) {
        format!("{}, {} – {} {duration}", date(event.start), clock(event.start, locale), clock(event.end, locale))
    } else {
        format!("{} at {} – {} at {} {duration}", date(event.start), clock(event.start, locale), date(event.end), clock(event.end, locale))
    }
}

pub fn event_details(event: &Event, calendar: Option<&Calendar>, locale: &str) -> String {
    [event.title.clone(), when_text(event, locale), event.location.clone(), calendar.map(|c| format!("Calendar: {}", c.title)).unwrap_or_default(), event.link.clone(), event.notes.clone()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join("\n")
}

pub fn default_start(now: i64) -> i64 {
    let local = now + offset(now);
    from_local((local.div_euclid(HOUR) + 1) * HOUR)
}

pub fn to_input(ms: i64, all_day: bool) -> String {
    let local = ms + offset(ms);
    crate::date_picker::format(local.div_euclid(DAY), local.rem_euclid(DAY) / 60_000, !all_day)
}

pub fn from_input(text: &str) -> Option<i64> {
    crate::date_picker::parse(text).map(|(day, minutes)| from_local(day * DAY + minutes * 60_000))
}

pub struct Draft {
    pub title: String,
    pub start: i64,
    pub end: i64,
    pub all_day: bool,
    pub calendar_id: String,
    pub location: String,
    pub notes: String,
}

pub fn draft_to_event(draft: &Draft) -> NewEvent {
    let (start, end) = if draft.all_day { (day_start(local_day(draft.start)), day_start(local_day(draft.end.max(draft.start)) + 1)) } else { (draft.start, draft.end) };
    NewEvent {
        title: draft.title.trim().to_owned(),
        start,
        end,
        all_day: draft.all_day,
        calendar_id: Some(draft.calendar_id.clone()).filter(|c| !c.is_empty()),
        location: draft.location.clone(),
        notes: draft.notes.clone(),
    }
}

fn color(hex: &str) -> Option<Color32> {
    Color32::from_hex(hex).ok()
}

struct CreateEvent {
    form: Form,
    start: i64,
    end: i64,
    all_day: bool,
}

impl CreateEvent {
    fn factory() -> ViewFactory {
        Rc::new(|_| {
            let start = default_start(crate::store::now_ms() as i64);
            let calendars = crate::calendar::list().unwrap_or_default();
            let mut options = vec![(String::new(), "Default Calendar".to_string(), Icon::Glyph("calendar-days"))];
            options.extend(calendars.iter().map(|c| (c.id.clone(), c.title.clone(), color(&c.color).map_or(Icon::Glyph("calendar-days"), Icon::Dot))));
            let fields = vec![
                Field::new("Title", FieldKind::Text, "").placeholder("Event title"),
                Field::new("Calendar", FieldKind::Choice(options), ""),
                Field::new("Start", FieldKind::Date { time: true }, to_input(start, false)).placeholder(to_input(start, false)),
                Field::new("End", FieldKind::Date { time: true }, to_input(start + HOUR, false)).placeholder(to_input(start + HOUR, false)),
                Field::new("All Day", FieldKind::Toggle, "false"),
                Field::new("Location", FieldKind::Text, "").placeholder("Add location"),
                Field::new("Notes", FieldKind::Area(80.0), "").placeholder("Add notes"),
            ];
            let mut view = CreateEvent { form: Form { title: "Create Event".into(), submit: "Create Event", icon: ICON, fields, focus: Some(0) }, start, end: start + HOUR, all_day: false };
            view.sync();
            Box::new(view)
        })
    }

    fn field(&mut self, label: &str) -> Option<&mut Field> {
        self.form.fields.iter_mut().find(|f| f.label == label)
    }

    fn value(&self, label: &str) -> String {
        self.form.fields.iter().find(|f| f.label == label).map(|f| f.value.clone()).unwrap_or_default()
    }

    fn sync(&mut self) {
        let (start, end, all_day) = (self.start, self.end, self.all_day);
        if let Some(f) = self.field("Start") {
            f.value = to_input(start, all_day);
            f.placeholder = to_input(start, all_day);
            f.kind = FieldKind::Date { time: !all_day };
            f.error = None;
        }
        if let Some(f) = self.field("End") {
            f.value = to_input(end, all_day);
            f.placeholder = to_input(end, all_day);
            f.kind = FieldKind::Date { time: !all_day };
            f.error = None;
        }
        let duration = end - start;
        let has = self.form.fields.iter().any(|f| f.label == "Duration");
        if all_day && has {
            self.form.fields.retain(|f| f.label != "Duration");
        } else if !all_day {
            let mut options: Vec<(String, String, Icon)> = DURATIONS.iter().map(|(m, l)| ((m * 60_000).to_string(), (*l).to_owned(), Icon::Glyph("clock"))).collect();
            if !options.iter().any(|o| o.0 == duration.to_string()) {
                options.push((duration.to_string(), format_duration(duration), Icon::Glyph("clock")));
            }
            let field = Field::new("Duration", FieldKind::Choice(options), duration.to_string());
            match self.form.fields.iter().position(|f| f.label == "Duration") {
                Some(i) => self.form.fields[i] = field,
                None => {
                    let at = self.form.fields.iter().position(|f| f.label == "All Day").unwrap_or(self.form.fields.len());
                    self.form.fields.insert(at, field);
                }
            }
        }
    }
}

impl View for CreateEvent {
    fn placeholder(&self) -> String {
        String::new()
    }

    fn footer(&self) -> (String, Icon) {
        ("Create Event".into(), ICON)
    }

    fn sections(&mut self) -> Vec<Section> {
        Vec::new()
    }

    fn form(&mut self) -> Option<&mut Form> {
        Some(&mut self.form)
    }

    fn form_changed(&mut self, index: usize) {
        let Some(label) = self.form.fields.get(index).map(|f| f.label) else { return };
        match label {
            "Start" => {
                if let Some(start) = from_input(&self.value("Start")) {
                    let duration = if self.end > self.start { self.end - self.start } else { HOUR };
                    self.start = start;
                    self.end = start + if self.all_day { 0 } else { duration };
                    let text = to_input(self.end, self.all_day);
                    if let Some(f) = self.field("End") {
                        f.value = text;
                    }
                }
            }
            "End" => {
                if let Some(end) = from_input(&self.value("End")) {
                    self.end = end;
                    if self.all_day {
                        self.start = self.start.min(end);
                    } else if end <= self.start {
                        self.start = end - HOUR;
                    } else {
                        return self.sync_duration();
                    }
                    let (start, all_day) = (self.start, self.all_day);
                    if let Some(f) = self.field("Start") {
                        f.value = to_input(start, all_day);
                    }
                    self.sync_duration();
                }
            }
            "Duration" => {
                if let Ok(duration) = self.value("Duration").parse::<i64>() {
                    self.end = self.start + duration;
                    self.sync();
                }
            }
            "All Day" => {
                self.all_day = self.value("All Day") == "true";
                self.end = if self.all_day { self.start } else { self.start + HOUR };
                self.sync();
            }
            _ => {}
        }
    }

    fn submit(&mut self, host: &mut Host) -> Result<bool, String> {
        let title = self.value("Title");
        let example = format!("Enter a date like {}", to_input(self.start, self.all_day));
        let format = example.as_str();
        let start = from_input(&self.value("Start"));
        let end = from_input(&self.value("End"));
        let errors = [("Title", title.trim().is_empty().then_some("Title is required")), ("Start", start.is_none().then_some(format)), ("End", end.is_none().then_some(format))];
        for (label, error) in errors {
            if let Some(f) = self.field(label) {
                f.error = error.map(String::from);
            }
        }
        if let Some(i) = self.form.fields.iter().position(|f| f.error.is_some()) {
            self.form.focus = Some(i);
            return Ok(false);
        }
        let draft = Draft { title, start: start.unwrap_or(self.start), end: end.unwrap_or(self.end), all_day: self.all_day, calendar_id: self.value("Calendar"), location: self.value("Location"), notes: self.value("Notes") };
        crate::calendar::create(draft_to_event(&draft)).map_err(|e| format!("Failed to create event: {e}"))?;
        VERSION.fetch_add(1, Ordering::SeqCst);
        host.toast = Some(Toast::Success("Created event".into()));
        Ok(true)
    }
}

impl CreateEvent {
    fn sync_duration(&mut self) {
        if !self.all_day {
            self.sync();
        }
    }
}

type Schedule = Arc<(Vec<Calendar>, Vec<Event>)>;

fn load_schedule(ctx: &egui::Context) -> Load<Schedule> {
    Load::start(ctx, || {
        let today = day_start(local_day(crate::store::now_ms() as i64));
        let calendars = crate::calendar::list().map_err(|e| e.message())?;
        let events = crate::calendar::events(today, day_start(local_day(today) + SCHEDULE_DAYS)).map_err(|e| e.message())?;
        Ok(Arc::new((calendars, events)))
    })
}

fn create_action() -> Action {
    Action::new("create", "Create Event", |host: &mut Host| {
        host.requests.push(Request::Push(CreateEvent::factory()));
        Ok(())
    })
    .icon("calendar-plus")
    .shortcut(Shortcut::key(Key::N).ctrl())
    .keep_open()
}

fn copy(text: String, title: &'static str) -> impl Fn(&mut Host) -> Result<(), String> {
    move |host| {
        host.copy_and_close(&text, title);
        Ok(())
    }
}

fn event_actions(event: &Event, calendar: Option<&Calendar>, locale: &str) -> Vec<Vec<Action>> {
    let mut open = vec![Action::new("open", "Open in Calendar", |_| crate::calendar::open()).icon("calendar-days")];
    if !event.link.is_empty() {
        let link = event.link.clone();
        open.push(Action::new("open-link", "Open Link", move |_| crate::apps::open_url(&link, None)).icon("external-link").shortcut(Shortcut::key(Key::L).ctrl().shift()));
    }
    let mut copies = vec![
        Action::new("copy-details", "Copy Event Details", copy(event_details(event, calendar, locale), "Copied event details")).icon("clipboard").shortcut(Shortcut::key(Key::C).ctrl().shift()),
        Action::new("copy-title", "Copy Event Title", copy(event.title.clone(), "Copied event title")).icon("clipboard").shortcut(Shortcut::key(Key::C).ctrl().alt()),
    ];
    if !event.link.is_empty() {
        copies.push(Action::new("copy-url", "Copy Event URL", copy(event.link.clone(), "Copied event URL")).icon("link").shortcut(Shortcut::key(Key::C).alt().shift()));
    }
    let message = if event.instance_start.is_some() { "Only this occurrence will be deleted.".to_owned() } else { format!("\"{}\" will be deleted from {}.", event.title, calendar.map_or("your calendar", |c| c.title.as_str())) };
    let (calendar_id, id, instance) = (event.calendar_id.clone(), event.id.clone(), event.instance_start);
    let delete = Action::new("delete", "Delete Event", move |host| {
        crate::calendar::delete(&calendar_id, &id, instance)?;
        VERSION.fetch_add(1, Ordering::SeqCst);
        host.toast = Some(Toast::Success("Deleted event".into()));
        Ok(())
    })
    .icon("trash-2")
    .destructive()
    .keep_open()
    .shortcut(Shortcut::key(Key::D).ctrl().shift())
    .confirm("Delete Event?", &message, "Delete");
    vec![open, vec![create_action()], copies, vec![delete]]
}

struct MySchedule {
    ctx: egui::Context,
    load: Load<Schedule>,
    version: u64,
    query: String,
    filter: usize,
    locale: String,
}

impl MySchedule {
    fn schedule(&self) -> Option<Result<Schedule, String>> {
        self.load.get()
    }

    fn visible<'a>(&self, schedule: &'a Schedule) -> Vec<EventSection<'a>> {
        let wanted = self.query.trim().to_lowercase();
        let calendar = self.filter.checked_sub(1).and_then(|i| schedule.0.get(i)).map(|c| c.id.clone());
        let events: Vec<Event> = schedule.1.iter().filter(|e| calendar.as_ref().is_none_or(|c| *c == e.calendar_id) && e.title.to_lowercase().contains(&wanted)).cloned().collect();
        let ids: Vec<String> = events.iter().map(|e| format!("{}:{}:{}", e.calendar_id, e.id, e.start)).collect();
        sections_for(&schedule.1, crate::store::now_ms() as i64, &self.locale)
            .into_iter()
            .map(|mut s| {
                s.events.retain(|e| ids.contains(&format!("{}:{}:{}", e.calendar_id, e.id, e.start)));
                s
            })
            .filter(|s| !s.events.is_empty())
            .collect()
    }
}

fn item_id(section: &EventSection, event: &Event) -> String {
    format!("{}.{}:{}:{}:{}", section.key.0, section.key.1, event.calendar_id, event.id, event.start)
}

impl View for MySchedule {
    fn placeholder(&self) -> String {
        "Filter by title…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("My Schedule".into(), ICON)
    }

    fn filters(&self) -> bool {
        false
    }

    fn set_query(&mut self, query: &str) {
        self.query = query.to_owned();
    }

    fn loading(&self) -> bool {
        self.schedule().is_none()
    }

    fn dropdown(&self) -> Option<Dropdown> {
        let Some(Ok(schedule)) = self.schedule() else { return None };
        if schedule.0.len() < 2 {
            return None;
        }
        let options = std::iter::once(("All Calendars".to_string(), Icon::Glyph("calendar-days"))).chain(schedule.0.iter().map(|c| (c.title.clone(), color(&c.color).map_or(Icon::Glyph("calendar-days"), Icon::Dot)))).collect();
        Some(Dropdown { label: "Filter by Calendar", options, selected: self.filter })
    }

    fn select(&mut self, index: usize) {
        self.filter = index;
    }

    fn empty(&self) -> Empty {
        match self.schedule() {
            Some(Err(e)) => Empty { title: "Couldn't read your calendars".into(), description: Some(e) },
            Some(Ok(s)) if s.0.is_empty() => Empty { title: "No calendars found".into(), description: Some("Add an account with a calendar in Windows Settings > Accounts > Email & accounts.".into()) },
            _ if !self.query.is_empty() || self.filter != 0 => Empty { title: "No Events".into(), description: Some("No events match this filter.".into()) },
            _ => Empty { title: "No Events".into(), description: Some("Your schedule is clear.".into()) },
        }
    }

    fn empty_actions(&self) -> Vec<Vec<Action>> {
        vec![vec![create_action()]]
    }

    fn has_detail(&self) -> bool {
        self.schedule().is_some_and(|s| s.is_ok_and(|s| !self.visible(&s).is_empty()))
    }

    fn detail(&mut self, ui: &mut egui::Ui, rect: Rect, id: &str) {
        let Some(Ok(schedule)) = self.schedule() else { return };
        let sections = self.visible(&schedule);
        let Some(event) = sections.iter().flat_map(|s| s.events.iter().map(move |e| (item_id(s, e), *e))).find(|(i, _)| i == id).map(|(_, e)| e) else { return };
        let calendar = schedule.0.iter().find(|c| c.id == event.calendar_id);
        let mut rows = vec![("When".to_string(), when_text(event, &self.locale))];
        for (label, value) in [("Calendar", calendar.map(|c| c.title.clone()).unwrap_or_default()), ("Location", event.location.clone()), ("Organizer", event.organizer.clone()), ("Attendees", if event.attendees > 0 { event.attendees.to_string() } else { String::new() })] {
            if !value.is_empty() {
                rows.push((label.into(), value));
            }
        }
        let info = crate::root::metadata(ui, rect, &rows);
        let area = Rect::from_min_max(rect.min, pos2(rect.right(), info.top())).shrink(16.0);
        let painter = ui.painter().with_clip_rect(area);
        let title = if event.title.is_empty() { "Untitled event" } else { &event.title };
        let mut job = egui::text::LayoutJob::single_section(title.to_owned(), egui::TextFormat { font_id: theme::weighted(15.0, theme::SEMIBOLD), color: theme::FG(), strikethrough: if event.canceled { egui::Stroke::new(1.0, theme::FG()) } else { egui::Stroke::NONE }, ..Default::default() });
        job.wrap.max_width = area.width();
        let galley = ui.ctx().fonts_mut(|f| f.layout_job(job));
        let mut y = area.top() + galley.size().y + 8.0;
        painter.galley(area.min, galley, theme::FG());
        if !event.link.is_empty() {
            let link = crate::root::wrapped(ui, &event.link, theme::font(13.0), theme::FG_60(), area.width());
            y += link.size().y + 8.0;
            painter.galley(pos2(area.left(), y - link.size().y - 8.0), link, theme::FG_60());
        }
        let (notes, color) = if event.notes.is_empty() { ("No notes", theme::FG_40()) } else { (event.notes.as_str(), theme::FG()) };
        painter.galley(pos2(area.left(), y), crate::root::wrapped(ui, notes, theme::font(13.0), color, area.width()), color);
    }

    fn sections(&mut self) -> Vec<Section> {
        let version = VERSION.load(Ordering::SeqCst);
        if version != self.version {
            if self.version != u64::MAX {
                self.load = load_schedule(&self.ctx);
            }
            self.version = version;
        }
        let Some(Ok(schedule)) = self.schedule() else { return Vec::new() };
        let now = crate::store::now_ms() as i64;
        self.visible(&schedule)
            .iter()
            .map(|s| Section {
                title: match &s.subtitle {
                    Some(sub) => format!("{}  {sub}", s.title),
                    None => s.title.clone(),
                },
                items: s
                    .events
                    .iter()
                    .map(|e| {
                        let calendar = schedule.0.iter().find(|c| c.id == e.calendar_id);
                        Item {
                            id: item_id(s, e),
                            title: if e.title.is_empty() { "Untitled event".into() } else { e.title.clone() },
                            subtitle: None,
                            icon: Icon::Time(calendar.and_then(|c| color(&c.color)), time_label(e, now, &self.locale), e.canceled || e.end < now),
                            accessories: Vec::new(),
                            keywords: vec![e.location.clone()],
                            actions: event_actions(e, calendar, &self.locale),
                        }
                    })
                    .collect(),
            })
            .collect()
    }
}

pub struct CalendarExtension;

impl Extension for CalendarExtension {
    fn id(&self) -> &'static str {
        "calendar"
    }

    fn commands(&self) -> Vec<Command> {
        let mut schedule = command(self.id(), "mySchedule", "My Schedule", ICON, "Command", "Open Command", |_| Ok(()));
        schedule.keywords = ["calendar", "agenda", "events", "meetings", "schedule", "calendário", "eventos", "compromissos"].map(String::from).to_vec();
        schedule.view = Some(Rc::new(|ctx| Box::new(MySchedule { ctx: ctx.clone(), load: load_schedule(ctx), version: u64::MAX, query: String::new(), filter: 0, locale: icu::display_locale() })));
        let mut create = command(self.id(), "createEvent", "Create Event", Icon::Tile("calendar-plus", Tone::Red), "Command", "Open Command", |_| Ok(()));
        create.keywords = ["calendar", "event", "meeting", "new", "add", "calendário", "evento", "criar evento"].map(String::from).to_vec();
        create.view = Some(CreateEvent::factory());
        let mut commands = vec![schedule, create];
        for cmd in &mut commands {
            cmd.subtitle = Some("Calendar".into());
        }
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::calculator::history::days_from_civil;

    fn at(y: i64, m: i64, d: i64, h: i64, min: i64) -> i64 {
        from_local(days_from_civil(y, m, d) * DAY + h * HOUR + min * 60_000)
    }

    fn event(id: &str, start: i64, end: i64, all_day: bool) -> Event {
        Event { id: id.into(), calendar_id: "c".into(), title: id.into(), start, end, all_day, location: String::new(), notes: String::new(), link: String::new(), organizer: String::new(), attendees: 0, instance_start: None, canceled: false }
    }

    fn now() -> i64 {
        at(2026, 10, 21, 10, 30)
    }

    #[test]
    fn groups_this_week_by_day_then_next_week_then_by_month() {
        let events = [
            event("later", at(2026, 12, 3, 9, 0), at(2026, 12, 3, 10, 0), false),
            event("nov", at(2026, 11, 5, 9, 0), at(2026, 11, 5, 10, 0), false),
            event("next", at(2026, 10, 27, 9, 0), at(2026, 10, 27, 10, 0), false),
            event("fri", at(2026, 10, 23, 9, 0), at(2026, 10, 23, 10, 0), false),
            event("tomorrow", at(2026, 10, 22, 9, 0), at(2026, 10, 22, 10, 0), false),
            event("today-late", at(2026, 10, 21, 16, 0), at(2026, 10, 21, 17, 0), false),
            event("today", at(2026, 10, 21, 9, 0), at(2026, 10, 21, 11, 0), false),
            event("past", at(2026, 10, 20, 9, 0), at(2026, 10, 20, 10, 0), false),
        ];
        let sections = sections_for(&events, now(), "en-GB");
        assert_eq!(sections.iter().map(|s| s.title.as_str()).collect::<Vec<_>>(), ["Today", "Tomorrow", "Friday", "Next Week", "November", "December"]);
        assert_eq!(sections[0].subtitle.as_deref(), Some("21 Oct"));
        assert_eq!(sections[0].events.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["today", "today-late"]);
    }

    #[test]
    fn puts_ongoing_multi_day_events_under_today() {
        assert_eq!(sections_for(&[event("trip", at(2026, 10, 19, 9, 0), at(2026, 10, 24, 9, 0), false)], now(), "en-GB")[0].title, "Today");
        assert_eq!(sections_for(&[event("late", at(2026, 10, 20, 9, 0), at(2026, 10, 20, 10, 0), false)], at(2026, 10, 1, 0, 0), "en-GB")[0].title, "Rest of October");
    }

    #[test]
    fn labels_times_and_describes_events() {
        assert_eq!(time_label(&event("a", at(2026, 10, 21, 9, 0), at(2026, 10, 21, 10, 0), false), now(), "en-GB"), "09:00 – 10:00");
        assert_eq!(time_label(&event("b", at(2026, 10, 22, 0, 0), at(2026, 10, 23, 0, 0), true), now(), "en-GB"), "All day");
        assert_eq!(time_label(&event("c", at(2026, 10, 28, 9, 0), at(2026, 10, 28, 10, 0), false), now(), "en-GB"), "Wed 28");
        assert_eq!(time_label(&event("d", at(2027, 1, 5, 9, 0), at(2027, 1, 5, 10, 0), false), now(), "en-GB"), "5 Jan 2027");
        assert_eq!(when_text(&event("a", at(2026, 10, 21, 9, 0), at(2026, 10, 21, 10, 30), false), "en-GB"), "21 Oct 2026, 09:00 – 10:30 (Duration: 1 hour 30 minutes)");
        assert_eq!(when_text(&event("b", at(2026, 10, 21, 0, 0), at(2026, 10, 22, 0, 0), true), "en-GB"), "21 Oct 2026 (All day)");
        assert_eq!(format_duration(45 * 60_000), "45 minutes");
    }

    #[test]
    fn starts_at_the_next_hour_and_round_trips_inputs() {
        assert_eq!(default_start(now()), at(2026, 10, 21, 11, 0));
        assert_eq!(to_input(at(2026, 10, 21, 9, 5), false), "2026-10-21 09:05");
        assert_eq!(to_input(at(2026, 10, 21, 9, 5), true), "2026-10-21");
        assert_eq!(from_input("2026-10-21 09:05"), Some(at(2026, 10, 21, 9, 5)));
        assert_eq!(from_input("2026-10-21T09:05"), Some(at(2026, 10, 21, 9, 5)));
        assert_eq!(from_input("2026-10-21"), Some(at(2026, 10, 21, 0, 0)));
        assert_eq!(from_input(""), None);
        assert_eq!(from_input("2026-13-01"), None);
        assert_eq!(from_input("2026-02-30 10:00"), None);
    }

    #[test]
    fn makes_all_day_drafts_span_whole_days() {
        let draft = Draft { title: " Trip ".into(), start: at(2026, 10, 21, 9, 0), end: at(2026, 10, 23, 9, 0), all_day: true, calendar_id: String::new(), location: String::new(), notes: String::new() };
        let event = draft_to_event(&draft);
        assert_eq!((event.title.as_str(), event.start, event.end, event.calendar_id), ("Trip", at(2026, 10, 21, 0, 0), at(2026, 10, 24, 0, 0), None));
    }
}
