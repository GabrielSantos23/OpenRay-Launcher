export interface CalendarInfo {
  id: string;
  title: string;
  source: string;
  color: string;
}

export interface CalendarEvent {
  id: string;
  calendarId: string;
  title: string;
  start: number;
  end: number;
  allDay: boolean;
  location: string;
  notes: string;
  link: string;
  organizer: string;
  attendees: number;
  instanceStart: number | null;
  canceled: boolean;
}

export interface EventSection {
  id: string;
  title: string;
  subtitle?: string;
  events: CalendarEvent[];
}

export const CREATE_EVENT_KEY = "calendar/createEvent";
export const CREATE_AT_EVENT = "calendar-create-at";
export const SCHEDULE_DAYS = 30;

export const DURATIONS = [
  { value: String(15 * 60_000), label: "15 minutes" },
  { value: String(30 * 60_000), label: "30 minutes" },
  { value: String(60 * 60_000), label: "1 hour" },
  { value: String(90 * 60_000), label: "1 hour 30 minutes" },
  { value: String(120 * 60_000), label: "2 hours" },
  { value: String(240 * 60_000), label: "4 hours" },
];

export function startOfDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

export function addDays(date: Date, days: number): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate() + days);
}

function startOfWeek(date: Date): Date {
  return addDays(startOfDay(date), -((date.getDay() + 6) % 7));
}

function sameDay(a: Date, b: Date): boolean {
  return a.toDateString() === b.toDateString();
}

export function sectionsFor(events: CalendarEvent[], now: Date, locale?: string): EventSection[] {
  const today = startOfDay(now);
  const nextWeek = addDays(startOfWeek(now), 7);
  const weekAfter = addDays(nextWeek, 7);
  const weekday = new Intl.DateTimeFormat(locale, { weekday: "long" });
  const dayMonth = new Intl.DateTimeFormat(locale, { day: "numeric", month: "short" });
  const month = new Intl.DateTimeFormat(locale, { month: "long" });
  const map = new Map<string, EventSection>();
  for (const event of events) {
    if (event.end <= today.getTime() && !(event.allDay && event.start >= today.getTime())) continue;
    const day = new Date(Math.max(startOfDay(new Date(event.start)).getTime(), today.getTime()));
    let key: string;
    let section: Omit<EventSection, "events">;
    if (day < nextWeek) {
      key = `0.${day.getTime()}`;
      const title = sameDay(day, today) ? "Today" : sameDay(day, addDays(today, 1)) ? "Tomorrow" : weekday.format(day);
      section = { id: key, title, subtitle: dayMonth.format(day) };
    } else if (day < weekAfter) {
      key = "1.week";
      section = { id: key, title: "Next Week" };
    } else {
      key = `2.${day.getFullYear() * 100 + day.getMonth()}`;
      const name = month.format(day);
      section = { id: key, title: day.getMonth() === now.getMonth() && day.getFullYear() === now.getFullYear() ? `Rest of ${name}` : name };
    }
    if (!map.has(key)) map.set(key, { ...section, events: [] });
    map.get(key)!.events.push(event);
  }
  return [...map.values()]
    .sort((a, b) => a.id.localeCompare(b.id, undefined, { numeric: true }))
    .map((s) => ({ ...s, events: s.events.sort((a, b) => a.start - b.start) }));
}

export function timeLabel(event: CalendarEvent, now: Date, locale?: string): string {
  const start = new Date(event.start);
  const time = new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit" });
  if (start < addDays(startOfWeek(now), 7)) return event.allDay ? "All day" : `${time.format(start)} – ${time.format(new Date(event.end))}`;
  if (start < addDays(startOfWeek(now), 14)) return new Intl.DateTimeFormat(locale, { weekday: "short", day: "numeric" }).format(start);
  const sameYear = start.getFullYear() === now.getFullYear();
  return new Intl.DateTimeFormat(locale, { day: "numeric", month: "short", year: sameYear ? undefined : "numeric" }).format(start);
}

export function formatDuration(ms: number): string {
  const minutes = Math.round(ms / 60_000);
  const hours = Math.floor(minutes / 60);
  const rest = minutes % 60;
  const parts = [hours ? `${hours} ${hours === 1 ? "hour" : "hours"}` : "", rest ? `${rest} ${rest === 1 ? "minute" : "minutes"}` : ""].filter(Boolean);
  return parts.join(" ") || "0 minutes";
}

export function whenText(event: CalendarEvent, locale?: string): string {
  const date = new Intl.DateTimeFormat(locale, { day: "numeric", month: "short", year: "numeric" });
  const time = new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit" });
  const start = new Date(event.start);
  const end = new Date(event.end);
  if (event.allDay) {
    const last = new Date(event.end - 1);
    return sameDay(start, last) ? `${date.format(start)} (All day)` : `${date.format(start)} – ${date.format(last)}`;
  }
  const duration = `(Duration: ${formatDuration(event.end - event.start)})`;
  return sameDay(start, end)
    ? `${date.format(start)}, ${time.format(start)} – ${time.format(end)} ${duration}`
    : `${date.format(start)} at ${time.format(start)} – ${date.format(end)} at ${time.format(end)} ${duration}`;
}

export function eventDetails(event: CalendarEvent, calendar?: CalendarInfo, locale?: string): string {
  return [event.title, whenText(event, locale), event.location, calendar && `Calendar: ${calendar.title}`, event.link, event.notes].filter(Boolean).join("\n");
}

export function eventsOn(events: CalendarEvent[], day: Date): CalendarEvent[] {
  const start = startOfDay(day).getTime();
  const end = addDays(day, 1).getTime();
  return events.filter((e) => e.start < end && e.end > start).sort((a, b) => Number(b.allDay) - Number(a.allDay) || a.start - b.start);
}

export function defaultStart(now: Date, day?: Date): Date {
  const next = new Date(now.getFullYear(), now.getMonth(), now.getDate(), now.getHours() + 1);
  if (!day || sameDay(day, now)) return next;
  return new Date(day.getFullYear(), day.getMonth(), day.getDate(), 9);
}

export function toLocalInput(ms: number, allDay: boolean): string {
  const d = new Date(ms);
  const pad = (n: number) => String(n).padStart(2, "0");
  const date = `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
  return allDay ? date : `${date}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export function fromLocalInput(value: string): number | null {
  const ms = new Date(value.includes("T") ? value : `${value}T00:00`).getTime();
  return Number.isNaN(ms) ? null : ms;
}

export interface EventDraft {
  title: string;
  start: number;
  end: number;
  allDay: boolean;
  calendarId: string;
  location: string;
  notes: string;
}

export function draftToEvent(draft: EventDraft) {
  if (!draft.allDay) return { ...draft, title: draft.title.trim(), calendarId: draft.calendarId || null };
  const start = startOfDay(new Date(draft.start));
  const end = addDays(new Date(Math.max(draft.end, draft.start)), 1);
  return { ...draft, title: draft.title.trim(), start: start.getTime(), end: startOfDay(end).getTime(), calendarId: draft.calendarId || null };
}
