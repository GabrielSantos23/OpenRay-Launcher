import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { CalendarDays, CalendarPlus, Clipboard, ExternalLink, Link, Trash2 } from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useState } from "react";
import { fail } from "../ActionHost";
import {
  addDays,
  CREATE_AT_EVENT,
  defaultStart,
  draftToEvent,
  DURATIONS,
  eventDetails,
  formatDuration,
  fromLocalInput,
  SCHEDULE_DAYS,
  sectionsFor,
  startOfDay,
  timeLabel,
  toLocalInput,
  whenText,
  type CalendarEvent,
  type CalendarInfo,
  type EventDraft,
} from "../calendar";
import { Switch } from "../controls";
import { Dropdown, type DropdownOption } from "../Dropdown";
import { defineExtension, type Action, type IconSpec, type Item, type ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { ListView, Metadata, type ListSection } from "../ListView";
import { copyAndClose } from "../output";
import { showToast } from "../toast";
import { field, FormShell, label } from "./focus";

const ICON: IconSpec = { glyph: CalendarDays, tone: "red" };
const ALL = "__all__";
const HOUR_MS = 3_600_000;

let pendingStart: number | null = null;
listen<number>(CREATE_AT_EVENT, (e) => {
  pendingStart = e.payload;
}).catch(() => {});

function Dot({ color }: { color?: string }) {
  return <span aria-hidden="true" className="size-2 shrink-0 rounded-full bg-fg-40" style={color ? { backgroundColor: color } : undefined} />;
}

function footer(title: string) {
  return (
    <span className="flex min-w-0 items-center gap-2 text-fg-60">
      <GlyphTile spec={ICON} small />
      <span className="truncate">{title}</span>
    </span>
  );
}

function EventDetail({ event, calendar }: { event: CalendarEvent; calendar?: CalendarInfo }) {
  const rows = [
    { label: "When", value: whenText(event) },
    ...(calendar ? [{ label: "Calendar", value: <><Dot color={calendar.color} />{calendar.title}</> }] : []),
    ...(event.location ? [{ label: "Location", value: event.location }] : []),
    ...(event.organizer ? [{ label: "Organizer", value: event.organizer }] : []),
    ...(event.attendees ? [{ label: "Attendees", value: event.attendees }] : []),
  ];
  return (
    <>
      <div className="flex min-h-0 flex-1 flex-col gap-2 overflow-y-auto p-4">
        <h2 className={`text-15 font-semibold text-fg ${event.canceled ? "line-through" : ""}`}>{event.title || "Untitled event"}</h2>
        {event.link && <p className="truncate text-13 text-fg-60">{event.link}</p>}
        {event.notes ? <p className="text-13 whitespace-pre-wrap text-fg">{event.notes}</p> : <p className="text-13 text-fg-40">No notes</p>}
      </div>
      <Metadata rows={rows} />
    </>
  );
}

function eventActions(event: CalendarEvent, calendar: CalendarInfo | undefined, create: () => void, reload: () => void): Action[][] {
  return [
    [
      { id: "open", title: "Open in Calendar", icon: CalendarDays, run: () => invoke<void>("calendar_open") },
      ...(event.link ? [{ id: "open-link", title: "Open Link", icon: ExternalLink, shortcut: { key: "l", ctrl: true, shift: true }, run: () => invoke<void>("open_url", { url: event.link, appPath: null }) }] : []),
    ],
    [{ id: "create", title: "Create Event", icon: CalendarPlus, shortcut: { key: "n", ctrl: true }, keepOpen: true, run: create }],
    [
      { id: "copy-details", title: "Copy Event Details", icon: Clipboard, shortcut: { key: "c", ctrl: true, shift: true }, run: () => copyAndClose(eventDetails(event, calendar), "Copied event details") },
      { id: "copy-title", title: "Copy Event Title", icon: Clipboard, shortcut: { key: "c", ctrl: true, alt: true }, run: () => copyAndClose(event.title, "Copied event title") },
      ...(event.link ? [{ id: "copy-url", title: "Copy Event URL", icon: Link, shortcut: { key: "c", alt: true, shift: true }, run: () => copyAndClose(event.link, "Copied event URL") }] : []),
    ],
    [
      {
        id: "delete",
        title: "Delete Event",
        icon: Trash2,
        destructive: true,
        keepOpen: true,
        shortcut: { key: "d", ctrl: true, shift: true },
        confirm: { title: "Delete Event?", message: event.instanceStart ? "Only this occurrence will be deleted." : `"${event.title}" will be deleted from ${calendar?.title ?? "your calendar"}.`, confirmTitle: "Delete" },
        run: async () => {
          await invoke("calendar_delete", { calendarId: event.calendarId, id: event.id, instanceStart: event.instanceStart });
          showToast({ variant: "success", title: "Deleted event" });
          reload();
        },
      },
    ],
  ];
}

function MySchedule({ pop }: ViewProps) {
  const [calendars, setCalendars] = useState<CalendarInfo[]>([]);
  const [events, setEvents] = useState<CalendarEvent[] | null>(null);
  const [error, setError] = useState<string>();
  const [text, setText] = useState("");
  const [filter, setFilter] = useState(ALL);
  const [creating, setCreating] = useState(false);

  const reload = useCallback(() => {
    const today = startOfDay(new Date());
    Promise.all([invoke<CalendarInfo[]>("calendar_list"), invoke<CalendarEvent[]>("calendar_events", { start: today.getTime(), end: addDays(today, SCHEDULE_DAYS).getTime() })])
      .then(([c, e]) => {
        setCalendars(c);
        setEvents(e);
        setError(undefined);
      })
      .catch((e) => {
        setEvents([]);
        setError(String(e));
      });
  }, []);

  useEffect(() => {
    reload();
    window.addEventListener("focus", reload);
    return () => window.removeEventListener("focus", reload);
  }, [reload]);

  const byId = useMemo(() => new Map(calendars.map((c) => [c.id, c])), [calendars]);
  const create = useCallback(() => setCreating(true), []);

  const { sections, eventByItem } = useMemo(() => {
    const now = new Date();
    const wanted = text.trim().toLowerCase();
    const visible = (events ?? []).filter((e) => (filter === ALL || e.calendarId === filter) && e.title.toLowerCase().includes(wanted));
    const eventByItem = new Map<string, CalendarEvent>();
    const sections = sectionsFor(visible, now).map<ListSection>((s) => ({
      title: s.title,
      subtitle: s.subtitle,
      items: s.events.map<Item>((e) => {
        const id = `${s.id}:${e.calendarId}:${e.id}:${e.start}`;
        eventByItem.set(id, e);
        return {
          id,
          title: e.title || "Untitled event",
          icon: (
            <span className={`flex shrink-0 items-center gap-2.5 ${e.canceled || e.end < now.getTime() ? "opacity-50" : ""}`}>
              <Dot color={byId.get(e.calendarId)?.color} />
              <span className="w-[104px] truncate text-fg-60 tabular-nums">{timeLabel(e, now)}</span>
            </span>
          ),
          keywords: [e.location],
          actions: eventActions(e, byId.get(e.calendarId), create, reload),
        };
      }),
    }));
    return { sections, eventByItem };
  }, [events, filter, text, byId, create, reload]);

  const options = useMemo<DropdownOption<string>[]>(
    () => [{ value: ALL, label: "All Calendars", icon: <CalendarDays aria-hidden="true" className="size-4 shrink-0" /> }, ...calendars.map((c) => ({ value: c.id, label: c.title, icon: <Dot color={c.color} /> }))],
    [calendars],
  );

  if (creating) return <CreateEvent pop={() => { setCreating(false); reload(); }} />;

  const empty = error
    ? { text: "Couldn't read your calendars", description: error }
    : calendars.length === 0 && events
      ? { text: "No calendars found", description: "Add an account with a calendar in Windows Settings > Accounts > Email & accounts." }
      : text || filter !== ALL
        ? { text: "No Events", description: "No events match this filter." }
        : { text: "No Events", description: "Your schedule is clear." };

  return (
    <ListView
      placeholder="Filter by title…"
      sections={sections}
      onBack={pop}
      searchText={text}
      onSearchTextChange={setText}
      filtering={false}
      footerTitle="My Schedule"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={events === null}
      emptyText={empty.text}
      emptyDescription={empty.description}
      emptyActions={[[{ id: "create", title: "Create Event", icon: CalendarPlus, shortcut: { key: "n", ctrl: true }, keepOpen: true, run: create }]]}
      accessory={
        calendars.length > 1 ? (
          <div className="w-[197px] shrink-0">
            <span id="calendar-filter" className="sr-only">
              Filter by Calendar
            </span>
            <Dropdown size="field" labelledBy="calendar-filter" value={filter} options={options} onChange={setFilter} />
          </div>
        ) : undefined
      }
      detail={
        sections.length
          ? (item) => {
              const event = eventByItem.get(item.id);
              return event ? <EventDetail event={event} calendar={byId.get(event.calendarId)} /> : null;
            }
          : undefined
      }
    />
  );
}

function initialDraft(): EventDraft {
  const start = (pendingStart !== null ? defaultStart(new Date(), new Date(pendingStart)) : defaultStart(new Date())).getTime();
  pendingStart = null;
  return { title: "", start, end: start + HOUR_MS, allDay: false, calendarId: "", location: "", notes: "" };
}

function CreateEvent({ pop }: ViewProps) {
  const [draft, setDraft] = useState(initialDraft);
  const [calendars, setCalendars] = useState<CalendarInfo[]>([]);
  const [error, setError] = useState<string>();
  const [saving, setSaving] = useState(false);
  const ids = useId();

  useEffect(() => {
    invoke<CalendarInfo[]>("calendar_list").then(setCalendars).catch((e) => fail("Couldn't read your calendars", e));
  }, []);

  const set = (patch: Partial<EventDraft>) => setDraft((d) => ({ ...d, ...patch }));
  const duration = draft.end - draft.start;

  function setStart(start: number | null) {
    if (start !== null) set({ start, end: start + (duration > 0 ? duration : HOUR_MS) });
  }

  function setEnd(end: number | null) {
    if (end === null) return;
    if (draft.allDay) set({ end, start: Math.min(draft.start, end) });
    else set({ end, start: end <= draft.start ? end - HOUR_MS : draft.start });
  }

  async function save() {
    if (saving) return;
    if (!draft.title.trim()) {
      setError("Title is required");
      document.getElementById(`${ids}-title`)?.focus();
      return;
    }
    setSaving(true);
    try {
      await invoke("calendar_create", { event: draftToEvent(draft) });
      showToast({ variant: "success", title: "Created event" });
      pop();
    } catch (e) {
      fail("Failed to create event", e);
    } finally {
      setSaving(false);
    }
  }

  const calendarOptions: DropdownOption<string>[] = [{ value: "", label: "Default Calendar" }, ...calendars.map((c) => ({ value: c.id, label: c.title, icon: <Dot color={c.color} /> }))];
  const durationOptions = DURATIONS.some((d) => Number(d.value) === duration) ? DURATIONS : [...DURATIONS, { value: String(duration), label: formatDuration(duration) }];
  const dateType = draft.allDay ? "date" : "datetime-local";

  return (
    <FormShell title="Create Event" primaryTitle="Create Event" onSave={save} onBack={pop} footer={footer("Create Event")}>
      <label htmlFor={`${ids}-title`} className={label}>
        Title
      </label>
      <div className="flex flex-col gap-1.5">
        <input
          id={`${ids}-title`}
          autoFocus
          value={draft.title}
          onChange={(e) => {
            set({ title: e.target.value });
            setError(undefined);
          }}
          placeholder="Event title"
          aria-invalid={!!error}
          aria-describedby={error ? `${ids}-title-error` : undefined}
          className={`${field} ${error ? "border-red" : ""}`}
        />
        {error && (
          <p id={`${ids}-title-error`} role="alert" className="text-12 text-red">
            {error}
          </p>
        )}
      </div>
      <span id={`${ids}-calendar`} className={label}>
        Calendar
      </span>
      <Dropdown size="field" labelledBy={`${ids}-calendar`} value={draft.calendarId} options={calendarOptions} onChange={(calendarId) => set({ calendarId })} />
      <label htmlFor={`${ids}-start`} className={label}>
        Start
      </label>
      <input id={`${ids}-start`} type={dateType} value={toLocalInput(draft.start, draft.allDay)} onChange={(e) => setStart(fromLocalInput(e.target.value))} className={field} />
      <label htmlFor={`${ids}-end`} className={label}>
        End
      </label>
      <input id={`${ids}-end`} type={dateType} value={toLocalInput(draft.end, draft.allDay)} onChange={(e) => setEnd(fromLocalInput(e.target.value))} className={field} />
      {!draft.allDay && (
        <>
          <span id={`${ids}-duration`} className={label}>
            Duration
          </span>
          <Dropdown size="field" labelledBy={`${ids}-duration`} value={String(duration)} options={durationOptions} onChange={(value) => set({ end: draft.start + Number(value) })} />
        </>
      )}
      <span id={`${ids}-allday`} className={label}>
        All Day
      </span>
      <div className="flex h-[34px] items-center">
        <Switch checked={draft.allDay} labelledBy={`${ids}-allday`} onChange={(allDay) => set({ allDay, end: allDay ? draft.start : draft.start + HOUR_MS })} />
      </div>
      <label htmlFor={`${ids}-location`} className={label}>
        Location
      </label>
      <input id={`${ids}-location`} value={draft.location} onChange={(e) => set({ location: e.target.value })} placeholder="Add location" className={field} />
      <label htmlFor={`${ids}-notes`} className={label}>
        Notes
      </label>
      <textarea
        id={`${ids}-notes`}
        value={draft.notes}
        onChange={(e) => set({ notes: e.target.value })}
        placeholder="Add notes"
        rows={3}
        className={`${field} h-auto resize-none py-2`}
      />
    </FormShell>
  );
}

export default defineExtension({
  id: "calendar",
  title: "Calendar",
  icon: ICON,
  commands: [
    {
      id: "mySchedule",
      title: "My Schedule",
      subtitle: "Calendar",
      keywords: ["calendar", "agenda", "events", "meetings", "schedule"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: MySchedule,
    },
    {
      id: "createEvent",
      title: "Create Event",
      icon: { glyph: CalendarPlus, tone: ICON.tone },
      subtitle: "Calendar",
      keywords: ["calendar", "event", "meeting", "new", "add"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: CreateEvent,
    },
  ],
});
