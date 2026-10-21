import { describe, expect, it } from "vitest";
import { defaultStart, draftToEvent, eventsOn, formatDuration, fromLocalInput, sectionsFor, timeLabel, toLocalInput, whenText, type CalendarEvent } from "./calendar";

const at = (y: number, m: number, d: number, h = 0, min = 0) => new Date(y, m - 1, d, h, min).getTime();
const event = (id: string, start: number, end: number, extra: Partial<CalendarEvent> = {}): CalendarEvent => ({
  id,
  calendarId: "c",
  title: id,
  start,
  end,
  allDay: false,
  location: "",
  notes: "",
  link: "",
  organizer: "",
  attendees: 0,
  instanceStart: null,
  canceled: false,
  ...extra,
});

const now = new Date(2026, 9, 21, 10, 30);

describe("calendar", () => {
  it("groups this week by day, then next week, then by month", () => {
    const events = [
      event("later", at(2026, 12, 3, 9), at(2026, 12, 3, 10)),
      event("nov", at(2026, 11, 5, 9), at(2026, 11, 5, 10)),
      event("next", at(2026, 10, 27, 9), at(2026, 10, 27, 10)),
      event("fri", at(2026, 10, 23, 9), at(2026, 10, 23, 10)),
      event("tomorrow", at(2026, 10, 22, 9), at(2026, 10, 22, 10)),
      event("today-late", at(2026, 10, 21, 16), at(2026, 10, 21, 17)),
      event("today", at(2026, 10, 21, 9), at(2026, 10, 21, 11)),
      event("past", at(2026, 10, 20, 9), at(2026, 10, 20, 10)),
    ];
    const sections = sectionsFor(events, now, "en-GB");
    expect(sections.map((s) => s.title)).toEqual(["Today", "Tomorrow", "Friday", "Next Week", "November", "December"]);
    expect(sections[0].subtitle).toBe("21 Oct");
    expect(sections[0].events.map((e) => e.id)).toEqual(["today", "today-late"]);
  });

  it("puts ongoing multi-day events under today", () => {
    const sections = sectionsFor([event("trip", at(2026, 10, 19, 9), at(2026, 10, 24, 9))], now, "en-GB");
    expect(sections[0].title).toBe("Today");
    expect(sectionsFor([event("late", at(2026, 10, 20, 9), at(2026, 10, 20, 10))], new Date(2026, 9, 1), "en-GB")[0].title).toBe("Rest of October");
  });

  it("labels times by distance from this week", () => {
    expect(timeLabel(event("a", at(2026, 10, 21, 9), at(2026, 10, 21, 10)), now, "en-GB")).toBe("09:00 – 10:00");
    expect(timeLabel(event("b", at(2026, 10, 22), at(2026, 10, 23), { allDay: true }), now, "en-GB")).toBe("All day");
    expect(timeLabel(event("c", at(2026, 10, 28, 9), at(2026, 10, 28, 10)), now, "en-GB")).toBe("Wed 28");
    expect(timeLabel(event("d", at(2027, 1, 5, 9), at(2027, 1, 5, 10)), now, "en-GB")).toBe("5 Jan 2027");
  });

  it("describes when an event happens", () => {
    expect(whenText(event("a", at(2026, 10, 21, 9), at(2026, 10, 21, 10, 30)), "en-GB")).toBe("21 Oct 2026, 09:00 – 10:30 (Duration: 1 hour 30 minutes)");
    expect(whenText(event("b", at(2026, 10, 21), at(2026, 10, 22), { allDay: true }), "en-GB")).toBe("21 Oct 2026 (All day)");
    expect(formatDuration(45 * 60_000)).toBe("45 minutes");
  });

  it("finds the events of one day, all-day first", () => {
    const events = [event("late", at(2026, 10, 21, 18), at(2026, 10, 21, 19)), event("allday", at(2026, 10, 21), at(2026, 10, 22), { allDay: true }), event("other", at(2026, 10, 22, 9), at(2026, 10, 22, 10))];
    expect(eventsOn(events, new Date(2026, 9, 21)).map((e) => e.id)).toEqual(["allday", "late"]);
  });

  it("starts new events at the next hour today, or at 9 on another day", () => {
    expect(defaultStart(now)).toEqual(new Date(2026, 9, 21, 11));
    expect(defaultStart(now, new Date(2026, 9, 24))).toEqual(new Date(2026, 9, 24, 9));
  });

  it("round-trips local input values", () => {
    expect(toLocalInput(at(2026, 10, 21, 9, 5), false)).toBe("2026-10-21T09:05");
    expect(toLocalInput(at(2026, 10, 21, 9, 5), true)).toBe("2026-10-21");
    expect(fromLocalInput("2026-10-21T09:05")).toBe(at(2026, 10, 21, 9, 5));
    expect(fromLocalInput("2026-10-21")).toBe(at(2026, 10, 21));
    expect(fromLocalInput("")).toBeNull();
  });

  it("makes all-day drafts span whole days", () => {
    const draft = { title: " Trip ", start: at(2026, 10, 21, 9), end: at(2026, 10, 23, 9), allDay: true, calendarId: "", location: "", notes: "" };
    expect(draftToEvent(draft)).toMatchObject({ title: "Trip", start: at(2026, 10, 21), end: at(2026, 10, 24), calendarId: null });
  });
});
