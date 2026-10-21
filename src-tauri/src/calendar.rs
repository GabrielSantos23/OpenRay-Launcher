use serde::{Deserialize, Serialize};
use windows::core::{HSTRING, PCWSTR};
use windows::ApplicationModel::Appointments::{
    Appointment, AppointmentCalendar, AppointmentManager, AppointmentProperties, AppointmentStore, AppointmentStoreAccessType, FindAppointmentsOptions,
};
use windows::Foundation::{DateTime, TimeSpan};
use windows::Win32::System::Registry::{RegCloseKey, RegOpenKeyExW, HKEY, HKEY_CLASSES_ROOT, KEY_READ};

const EPOCH_TICKS: i64 = 116_444_736_000_000_000;
const TICKS_PER_MS: i64 = 10_000;
const CALENDAR_SCHEMES: [&str; 2] = ["outlookcal", "ms-outlook"];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Calendar {
    id: String,
    title: String,
    source: String,
    color: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    id: String,
    calendar_id: String,
    title: String,
    start: i64,
    end: i64,
    all_day: bool,
    location: String,
    notes: String,
    link: String,
    organizer: String,
    attendees: u32,
    instance_start: Option<i64>,
    canceled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewEvent {
    title: String,
    start: i64,
    end: i64,
    all_day: bool,
    calendar_id: Option<String>,
    location: String,
    notes: String,
}

fn to_ms(time: DateTime) -> i64 {
    (time.UniversalTime - EPOCH_TICKS) / TICKS_PER_MS
}

fn from_ms(ms: i64) -> DateTime {
    DateTime { UniversalTime: ms * TICKS_PER_MS + EPOCH_TICKS }
}

fn hex(r: u8, g: u8, b: u8) -> String {
    format!("#{r:02X}{g:02X}{b:02X}")
}

fn store() -> windows::core::Result<AppointmentStore> {
    AppointmentManager::RequestStoreAsync(AppointmentStoreAccessType::AllCalendarsReadWrite)?.get()
}

fn visible(store: &AppointmentStore) -> windows::core::Result<Vec<AppointmentCalendar>> {
    Ok(store.FindAppointmentCalendarsAsync()?.get()?.into_iter().filter(|c| !c.IsHidden().unwrap_or(false)).collect())
}

fn list() -> windows::core::Result<Vec<Calendar>> {
    visible(&store()?)?
        .into_iter()
        .map(|c| {
            let color = c.DisplayColor()?;
            Ok(Calendar { id: c.LocalId()?.to_string(), title: c.DisplayName()?.to_string(), source: c.SourceDisplayName()?.to_string(), color: hex(color.R, color.G, color.B) })
        })
        .collect()
}

fn event(a: &Appointment) -> windows::core::Result<Event> {
    let start = to_ms(a.StartTime()?);
    let link = a.OnlineMeetingLink().map(|s| s.to_string()).unwrap_or_default();
    let uri = a.Uri().and_then(|u| u.AbsoluteUri()).map(|s| s.to_string()).unwrap_or_default();
    Ok(Event {
        id: a.LocalId()?.to_string(),
        calendar_id: a.CalendarId()?.to_string(),
        title: a.Subject()?.to_string(),
        start,
        end: start + a.Duration()?.Duration / TICKS_PER_MS,
        all_day: a.AllDay()?,
        location: a.Location()?.to_string(),
        notes: a.Details()?.to_string(),
        link: if link.is_empty() { uri } else { link },
        organizer: a.Organizer().and_then(|o| o.DisplayName()).map(|s| s.to_string()).unwrap_or_default(),
        attendees: a.Invitees().and_then(|i| i.Size()).unwrap_or(0),
        instance_start: a.OriginalStartTime().and_then(|t| t.Value()).ok().map(to_ms),
        canceled: a.IsCanceledMeeting().unwrap_or(false),
    })
}

fn events(start: i64, end: i64) -> windows::core::Result<Vec<Event>> {
    let store = store()?;
    let options = FindAppointmentsOptions::new()?;
    let fetch = options.FetchProperties()?;
    for name in [
        AppointmentProperties::Subject()?,
        AppointmentProperties::StartTime()?,
        AppointmentProperties::Duration()?,
        AppointmentProperties::AllDay()?,
        AppointmentProperties::Location()?,
        AppointmentProperties::Details()?,
        AppointmentProperties::Uri()?,
        AppointmentProperties::OnlineMeetingLink()?,
        AppointmentProperties::Organizer()?,
        AppointmentProperties::Invitees()?,
        AppointmentProperties::OriginalStartTime()?,
        AppointmentProperties::IsCanceledMeeting()?,
    ] {
        fetch.Append(&name)?;
    }
    let ids = options.CalendarIds()?;
    for calendar in visible(&store)? {
        ids.Append(&calendar.LocalId()?)?;
    }
    if ids.Size()? == 0 {
        return Ok(Vec::new());
    }
    let found = store.FindAppointmentsAsyncWithOptions(from_ms(start), TimeSpan { Duration: (end - start).max(0) * TICKS_PER_MS }, &options)?.get()?;
    let mut events: Vec<Event> = found.into_iter().filter_map(|a| event(&a).ok()).collect();
    events.sort_by_key(|e| e.start);
    Ok(events)
}

fn target(store: &AppointmentStore, id: Option<&str>) -> Result<AppointmentCalendar, String> {
    let calendars = visible(store).map_err(|e| e.to_string())?;
    match id.filter(|id| !id.is_empty()) {
        Some(id) => calendars.into_iter().find(|c| c.LocalId().is_ok_and(|l| l == id)).ok_or_else(|| "The calendar is no longer available".into()),
        None => calendars.into_iter().next().ok_or_else(|| "No calendar is set up in Windows. Add an account in Settings > Accounts > Email & accounts.".into()),
    }
}

fn create(input: NewEvent) -> Result<String, String> {
    if input.title.trim().is_empty() {
        return Err("Title is required".into());
    }
    if input.end <= input.start {
        return Err("The event must end after it starts".into());
    }
    let store = store().map_err(|e| e.to_string())?;
    let calendar = target(&store, input.calendar_id.as_deref())?;
    let save = || -> windows::core::Result<String> {
        let a = Appointment::new()?;
        a.SetSubject(&HSTRING::from(input.title.trim()))?;
        a.SetStartTime(from_ms(input.start))?;
        a.SetDuration(TimeSpan { Duration: (input.end - input.start) * TICKS_PER_MS })?;
        a.SetAllDay(input.all_day)?;
        a.SetLocation(&HSTRING::from(input.location.as_str()))?;
        a.SetDetails(&HSTRING::from(input.notes.as_str()))?;
        calendar.SaveAppointmentAsync(&a)?.get()?;
        Ok(a.LocalId()?.to_string())
    };
    save().map_err(|e| e.message())
}

fn delete(calendar_id: &str, id: &str, instance_start: Option<i64>) -> Result<(), String> {
    let store = store().map_err(|e| e.to_string())?;
    let calendar = target(&store, Some(calendar_id))?;
    let id = HSTRING::from(id);
    let result = match instance_start {
        Some(start) => calendar.DeleteAppointmentInstanceAsync(&id, from_ms(start)).and_then(|op| op.get()),
        None => calendar.DeleteAppointmentAsync(&id).and_then(|op| op.get()),
    };
    result.map_err(|e| e.message())
}

fn registered(scheme: &str) -> bool {
    let mut key = HKEY::default();
    let name = HSTRING::from(scheme);
    let found = unsafe { RegOpenKeyExW(HKEY_CLASSES_ROOT, PCWSTR(name.as_ptr()), None, KEY_READ, &mut key) }.is_ok();
    if found {
        unsafe {
            let _ = RegCloseKey(key);
        }
    }
    found
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn calendar_list() -> Result<Vec<Calendar>, String> {
    blocking(|| list().map_err(|e| e.message())).await
}

#[tauri::command]
pub async fn calendar_events(start: i64, end: i64) -> Result<Vec<Event>, String> {
    blocking(move || events(start, end).map_err(|e| e.message())).await
}

#[tauri::command]
pub async fn calendar_create(event: NewEvent) -> Result<String, String> {
    blocking(move || create(event)).await
}

#[tauri::command]
pub async fn calendar_delete(calendar_id: String, id: String, instance_start: Option<i64>) -> Result<(), String> {
    blocking(move || delete(&calendar_id, &id, instance_start)).await
}

#[tauri::command]
pub fn calendar_open() -> Result<(), String> {
    let scheme = CALENDAR_SCHEMES.into_iter().find(|s| registered(s)).ok_or("No calendar app is installed")?;
    crate::apps::open_url(&format!("{scheme}:"), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_between_unix_milliseconds_and_windows_ticks() {
        assert_eq!(from_ms(0).UniversalTime, EPOCH_TICKS);
        assert_eq!(to_ms(from_ms(1_761_048_000_000)), 1_761_048_000_000);
    }

    #[test]
    fn formats_calendar_colors_as_hex() {
        assert_eq!(hex(0xC1, 0xE9, 0xFE), "#C1E9FE");
        assert_eq!(hex(0, 10, 255), "#000AFF");
    }
}
