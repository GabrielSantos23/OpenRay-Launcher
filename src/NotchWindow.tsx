import { invoke } from "@tauri-apps/api/core";
import { emit, listen } from "@tauri-apps/api/event";
import type { UiTree } from "@flowkey-cli/native-sdk";
import { AppWindow, BatteryCharging, BatteryFull, BatteryLow, BatteryMedium, Check, ClipboardList, Crop, Music, Pause, Pipette, Play, Plus, ScanText, Settings, SkipBack, SkipForward, Volume2, VolumeX } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { addDays, CREATE_AT_EVENT, CREATE_EVENT_KEY, eventsOn, type CalendarEvent } from "./calendar";
import { entryTitle, type ClipboardEntry } from "./clipboardHistory";
import { formatTimer, remainingMs, type FocusSession } from "./focus";
import {
  formatClock,
  litSegments,
  mediaPosition,
  notchItems,
  notchSize,
  primaryAction,
  weekStrip,
  NOTCH_ACTION_EVENT,
  NOTCH_READY_EVENT,
  NOTCH_WIDGETS_EVENT,
  type NotchMedia,
  type NotchMode,
  type NotchWidget,
} from "./notch";

const OPEN_DELAY_MS = 90;
const CLOSE_DELAY_MS = 250;
const VOLUME_SHOW_MS = 1500;
const MEDIA_POLL_MS = 1000;
const BATTERY_POLL_MS = 60_000;
const VOLUME_SEGMENTS = 36;
const CLIPBOARD_SHOWN = 6;
const SHAPE_SETTLE_MS = 620;
const NOTCH_WINDOW = { width: 720, height: 224 };

type Tab = "nook" | "tray" | "widgets";

function run(promise: Promise<unknown>, label: string) {
  promise.catch((e) => console.error(`${label} failed`, e));
}

function useNow(ms: number) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), ms);
    return () => window.clearInterval(timer);
  }, [ms]);
  return now;
}

function IconButton({ label, onClick, children, className = "size-7" }: { label: string; onClick: () => void; children: ReactNode; className?: string }) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      onClick={onClick}
      className={`flex shrink-0 items-center justify-center rounded-full text-fg outline-offset-1 hover:bg-control focus-visible:outline-2 focus-visible:outline-accent ${className}`}
    >
      {children}
    </button>
  );
}

function Artwork({ media, className }: { media: NotchMedia; className: string }) {
  return media.artwork ? (
    <img src={media.artwork} alt="" draggable={false} className={`shrink-0 object-cover ${className}`} />
  ) : (
    <span aria-hidden="true" className={`flex shrink-0 items-center justify-center bg-control text-fg-60 ${className}`}>
      <Music className="size-1/2" />
    </span>
  );
}

function Equalizer() {
  return (
    <span aria-hidden="true" className="flex h-3 items-end gap-[2px]">
      {[0, 1, 2, 3].map((i) => (
        <span key={i} className={`notch-eq h-3 w-[3px] origin-bottom rounded-full bg-green notch-eq-${i}`} />
      ))}
    </span>
  );
}

function NowPlaying({ media }: { media: NotchMedia | null }) {
  const now = useNow(1000);
  if (!media?.title) {
    return (
      <div className="flex min-w-0 items-center gap-3">
        <span aria-hidden="true" className="flex size-[76px] shrink-0 items-center justify-center rounded-xl bg-control text-fg-40">
          <Music className="size-7" />
        </span>
        <div className="flex min-w-0 flex-col gap-1">
          <span className="text-13 font-medium text-fg">Nothing playing</span>
          <span className="text-12 text-fg-60">Start music or a video and it shows up here.</span>
        </div>
      </div>
    );
  }
  const position = mediaPosition(media, now);
  return (
    <div className="flex min-w-0 items-center gap-3">
      <div className="relative shrink-0">
        <Artwork media={media} className="size-[76px] rounded-xl" />
      </div>
      <div className="flex min-w-0 flex-1 flex-col">
        <span className="truncate text-13 font-semibold text-fg">{media.title}</span>
        <span className="truncate text-12 text-fg-60">{media.artist || media.album || " "}</span>
        {media.source && <span className="truncate text-11 text-fg-40">{media.source}</span>}
        <div className="mt-1 flex items-center gap-1">
          <IconButton label="Previous track" onClick={() => run(invoke("flowkey_media_control", { command: "previous" }), "previous")}>
            <SkipBack className="size-4 fill-current" />
          </IconButton>
          <IconButton label={media.playing ? "Pause" : "Play"} onClick={() => run(invoke("flowkey_media_control", { command: "playPause" }), "play/pause")}>
            {media.playing ? <Pause className="size-4 fill-current" /> : <Play className="size-4 fill-current" />}
          </IconButton>
          <IconButton label="Next track" onClick={() => run(invoke("flowkey_media_control", { command: "next" }), "next")}>
            <SkipForward className="size-4 fill-current" />
          </IconButton>
        </div>
        {position !== null && media.durationMs ? (
          <div className="flex items-center gap-2 text-11 text-fg-40 tabular-nums">
            <span>{formatClock(position)}</span>
            <span className="h-1 min-w-0 flex-1 overflow-hidden rounded-full bg-control">
              <span className="block h-full rounded-full bg-fg-60" style={{ width: `${(position / media.durationMs) * 100}%` }} />
            </span>
            <span>{formatClock(media.durationMs)}</span>
          </div>
        ) : null}
      </div>
    </div>
  );
}

function CalendarStrip({ focus, expanded }: { focus: FocusSession | null; expanded: boolean }) {
  const now = useNow(1000);
  const today = new Date(now);
  const todayKey = today.toDateString();
  const [offset, setOffset] = useState(0);
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const month = new Intl.DateTimeFormat(undefined, { month: "short" }).format(today);
  const selected = addDays(today, offset);

  useEffect(() => {
    if (!expanded) {
      setOffset(0);
      return;
    }
    const base = new Date(todayKey);
    run(invoke<CalendarEvent[]>("calendar_events", { start: addDays(base, -3).getTime(), end: addDays(base, 4).getTime() }).then(setEvents), "calendar_events");
  }, [expanded, todayKey]);

  const dayEvents = eventsOn(events, selected).slice(0, focus ? 1 : 2);
  const time = new Intl.DateTimeFormat(undefined, { hour: "2-digit", minute: "2-digit" });
  const create = async () => {
    await emit(CREATE_AT_EVENT, selected.getTime());
    await emit("run-command", CREATE_EVENT_KEY);
  };

  return (
    <div className="flex min-w-0 flex-col gap-3">
      <div className="flex items-end gap-2">
        <span className="text-18 leading-none font-semibold text-fg">{month}</span>
        <ol className="flex flex-1 justify-between" aria-label="This week">
          {weekStrip(today).map((d, i) => (
            <li key={d.key}>
              <button
                type="button"
                aria-current={d.today ? "date" : undefined}
                aria-pressed={offset === i - 3}
                onClick={() => setOffset(i - 3)}
                className={`flex w-6 flex-col items-center gap-0.5 rounded-md py-0.5 outline-offset-1 hover:bg-control focus-visible:outline-2 focus-visible:outline-accent ${offset === i - 3 ? "bg-control" : ""}`}
              >
                <span className={`text-[9px] uppercase ${d.today ? "text-fg" : "text-fg-40"}`}>{d.weekday}</span>
                <span className={`text-12 tabular-nums ${d.today ? "font-semibold text-accent" : "text-fg-60"}`}>{d.day}</span>
              </button>
            </li>
          ))}
        </ol>
      </div>
      {focus && (
        <div className="flex items-center gap-2 rounded-xl bg-control px-3 py-2">
          <div className="flex min-w-0 flex-1 flex-col">
            <span className="truncate text-12 font-medium text-fg">{focus.title || "Focus"}</span>
            <span className="text-11 text-fg-60 tabular-nums">
              {focus.status === "paused" ? "Paused · " : ""}
              {focus.durationMs ? `${formatTimer(remainingMs(focus, now))} left` : "No time limit"}
            </span>
          </div>
          <IconButton
            label={focus.status === "paused" ? "Resume focus" : "Pause focus"}
            onClick={() => run(invoke(focus.status === "paused" ? "focus_resume" : "focus_pause", focus.status === "paused" ? {} : { durationMs: null }), "focus")}
          >
            {focus.status === "paused" ? <Play className="size-3.5 fill-current" /> : <Pause className="size-3.5 fill-current" />}
          </IconButton>
          <IconButton label="Complete focus session" onClick={() => run(invoke("focus_complete"), "focus_complete")}>
            <Check className="size-3.5" />
          </IconButton>
        </div>
      )}
      <div className="flex min-w-0 items-start gap-2">
        {dayEvents.length ? (
          <ul className="flex min-w-0 flex-1 flex-col gap-0.5" aria-label="Events">
            {dayEvents.map((e) => (
              <li key={`${e.calendarId}:${e.id}:${e.start}`} className="flex min-w-0 items-center gap-2 text-12">
                <span aria-hidden="true" className="size-1.5 shrink-0 rounded-full bg-accent" />
                <span className="shrink-0 text-fg-60 tabular-nums">{e.allDay ? "All day" : time.format(e.start)}</span>
                <span className="truncate text-fg">{e.title || "Untitled event"}</span>
              </li>
            ))}
          </ul>
        ) : (
          <span className="min-w-0 flex-1 truncate text-12 text-fg-40">
            {offset === 0 ? "Nothing for today" : `Nothing for ${new Intl.DateTimeFormat(undefined, { weekday: "long" }).format(selected)}`}
          </span>
        )}
        <IconButton label="Create event" onClick={() => run(create(), "create event")} className="-my-1 size-6">
          <Plus className="size-3.5" />
        </IconButton>
      </div>
    </div>
  );
}

const CAPTURES = [
  { label: "Region", glyph: Crop, run: () => invoke("capture_start", { mode: "region" }) },
  { label: "Window", glyph: AppWindow, run: () => invoke("capture_direct", { kind: "window" }) },
  { label: "Text", glyph: ScanText, run: () => invoke("capture_start", { mode: "ocr" }) },
  { label: "Color", glyph: Pipette, run: () => invoke("capture_start", { mode: "color" }) },
];

function Captures() {
  return (
    <div className="grid grid-cols-2 gap-1.5" aria-label="Capture">
      {CAPTURES.map(({ label, glyph: Glyph, run: start }) => (
        <button
          key={label}
          type="button"
          onClick={() => run(start(), `capture ${label}`)}
          className="flex h-[42px] flex-col items-center justify-center gap-0.5 rounded-xl bg-control text-11 text-fg-60 outline-offset-1 hover:bg-control-hover hover:text-fg focus-visible:outline-2 focus-visible:outline-accent"
        >
          <Glyph aria-hidden="true" className="size-4 text-fg" />
          {label}
        </button>
      ))}
    </div>
  );
}

function Tray({ entries }: { entries: ClipboardEntry[] | null }) {
  if (entries && !entries.length) return <p className="py-6 text-center text-12 text-fg-60">Copy something and it appears here, ready to paste.</p>;
  return (
    <ul className="grid grid-cols-3 gap-1.5" aria-label="Recent clipboard items">
      {(entries ?? []).slice(0, CLIPBOARD_SHOWN).map((entry) => (
        <li key={entry.id} className="min-w-0">
          <button
            type="button"
            title={`Paste ${entryTitle(entry)}`}
            onClick={() => run(invoke("clipboard_paste", { id: entry.id }), "clipboard_paste")}
            className="flex h-[52px] w-full min-w-0 flex-col items-start justify-center gap-0.5 rounded-xl bg-control px-3 text-left outline-offset-1 hover:bg-control-hover focus-visible:outline-2 focus-visible:outline-accent"
          >
            <span className="w-full truncate text-12 text-fg">{entryTitle(entry)}</span>
            <span className="w-full truncate text-11 text-fg-40">{entry.sourceName || "Clipboard"}</span>
          </button>
        </li>
      ))}
    </ul>
  );
}

function WidgetBody({ widget }: { widget: NotchWidget }) {
  if (widget.error) return <p className="text-11 text-red">{widget.error}</p>;
  if (!widget.tree) return <p className="text-11 text-fg-40">Loading…</p>;
  const tree: UiTree = widget.tree;
  if (tree.type === "detail") {
    return (
      <div className="flex min-w-0 flex-col">
        <span className="truncate text-12 text-fg">{tree.title}</span>
        {tree.subtitle && <span className="truncate text-11 text-fg-60">{tree.subtitle}</span>}
        {tree.fields.slice(0, 2).map((f) => (
          <span key={f.label} className="truncate text-11 text-fg-40">
            {f.label}: {f.value}
          </span>
        ))}
      </div>
    );
  }
  const items = notchItems(tree).slice(0, 3);
  if (!items.length) return <p className="text-11 text-fg-40">{tree.type !== "form" && tree.emptyView?.title ? tree.emptyView.title : "Nothing to show"}</p>;
  return (
    <ul className="flex min-w-0 flex-col">
      {items.map((item) => {
        const action = primaryAction(item);
        return (
          <li key={item.id} className="min-w-0">
            <button
              type="button"
              disabled={!action}
              title={action ? `${action.title}: ${item.title}` : item.title}
              onClick={() => action && run(emit(NOTCH_ACTION_EVENT, { key: widget.key, itemId: item.id, actionId: action.id }), "notch action")}
              className="flex w-full min-w-0 flex-col rounded-md px-1.5 py-0.5 text-left outline-offset-1 enabled:hover:bg-control focus-visible:outline-2 focus-visible:outline-accent"
            >
              <span className="w-full truncate text-12 text-fg">{item.title}</span>
              {item.subtitle && <span className="w-full truncate text-11 text-fg-40">{item.subtitle}</span>}
            </button>
          </li>
        );
      })}
    </ul>
  );
}

function Widgets({ widgets }: { widgets: NotchWidget[] }) {
  return (
    <div className="grid grid-cols-3 gap-1.5">
      {widgets.slice(0, 3).map((widget) => (
        <section key={widget.key} aria-label={widget.title} className="flex min-w-0 flex-col gap-1 rounded-xl bg-control p-2">
          <h2 className="truncate px-1.5 text-11 font-medium text-fg-60">{widget.title}</h2>
          <WidgetBody widget={widget} />
        </section>
      ))}
    </div>
  );
}

function BatteryChip({ battery }: { battery: { percent: number; charging: boolean } | null }) {
  if (!battery) return null;
  const Glyph = battery.charging ? BatteryCharging : battery.percent > 60 ? BatteryFull : battery.percent > 20 ? BatteryMedium : BatteryLow;
  return (
    <span className={`flex items-center gap-1 text-11 tabular-nums ${battery.percent <= 20 && !battery.charging ? "text-red" : "text-fg-60"}`}>
      <Glyph aria-hidden="true" className="size-4" />
      {battery.percent}%<span className="sr-only">{battery.charging ? " charging" : " battery"}</span>
    </span>
  );
}

function VolumeView({ volume }: { volume: { percent: number; muted: boolean } }) {
  const lit = volume.muted ? 0 : litSegments(volume.percent, VOLUME_SEGMENTS);
  return (
    <div role="status" aria-label={volume.muted ? "Volume muted" : `Volume ${volume.percent}%`} className="flex h-full items-center gap-3 px-5">
      {volume.muted ? <VolumeX aria-hidden="true" className="size-5 shrink-0 text-fg" /> : <Volume2 aria-hidden="true" className="size-5 shrink-0 text-fg" />}
      <span aria-hidden="true" className="flex h-4 flex-1 items-center gap-[2px]">
        {Array.from({ length: VOLUME_SEGMENTS }, (_, i) => (
          <span key={i} className={`h-full flex-1 rounded-full ${i < lit ? "bg-fg" : "bg-control"}`} />
        ))}
      </span>
      <span className="w-10 shrink-0 text-right text-12 font-medium text-fg tabular-nums">{volume.muted ? "Muted" : `${volume.percent}%`}</span>
    </div>
  );
}

export function NotchWindow() {
  const [hovered, setHovered] = useState(false);
  const [expanded, setExpanded] = useState(false);
  const [tab, setTab] = useState<Tab>("nook");
  const [media, setMedia] = useState<NotchMedia | null>(null);
  const [volume, setVolume] = useState<{ percent: number; muted: boolean; at: number } | null>(null);
  const [focus, setFocus] = useState<FocusSession | null>(null);
  const [battery, setBattery] = useState<{ percent: number; charging: boolean } | null>(null);
  const [clipboard, setClipboard] = useState<ClipboardEntry[] | null>(null);
  const [widgets, setWidgets] = useState<NotchWidget[]>([]);
  const volumeTimer = useRef(0);

  useEffect(() => {
    const offs = [
      listen<boolean>("notch-hover", (e) => setHovered(e.payload)),
      listen<{ percent: number; muted: boolean }>("notch-volume", (e) => {
        window.clearTimeout(volumeTimer.current);
        setVolume({ ...e.payload, at: Date.now() });
        volumeTimer.current = window.setTimeout(() => setVolume(null), VOLUME_SHOW_MS);
      }),
      listen<FocusSession | null>("focus-session-changed", (e) => setFocus(e.payload)),
      listen<NotchWidget[]>(NOTCH_WIDGETS_EVENT, (e) => setWidgets(e.payload)),
    ];
    run(emit(NOTCH_READY_EVENT), "notch ready");
    run(invoke<FocusSession | null>("focus_get").then(setFocus), "focus_get");
    const pollMedia = () => run(invoke<NotchMedia>("notch_media").then(setMedia), "notch_media");
    const pollBattery = () => run(invoke<{ percent: number; charging: boolean } | null>("notch_battery").then(setBattery), "notch_battery");
    pollMedia();
    pollBattery();
    const timers = [window.setInterval(pollMedia, MEDIA_POLL_MS), window.setInterval(pollBattery, BATTERY_POLL_MS)];
    return () => {
      offs.forEach((off) => off.then((f) => f()));
      timers.forEach((t) => window.clearInterval(t));
    };
  }, []);

  useEffect(() => {
    const timer = window.setTimeout(() => setExpanded(hovered), hovered ? OPEN_DELAY_MS : CLOSE_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [hovered]);

  useEffect(() => {
    if (expanded) run(invoke<ClipboardEntry[]>("clipboard_list").then(setClipboard), "clipboard_list");
  }, [expanded, tab]);

  useEffect(() => {
    if (tab === "widgets" && !widgets.length) setTab("nook");
  }, [tab, widgets.length]);

  const mode: NotchMode = expanded ? "expanded" : volume ? "volume" : "collapsed";
  const playing = !!media?.playing && !!media.title;
  const size = notchSize(mode, playing);
  const collapsedSize = notchSize("collapsed", playing);
  const previous = useRef(size);
  const growing = size.width * size.height >= previous.current.width * previous.current.height;

  useEffect(() => {
    previous.current = size;
    const settle = () => invoke("notch_shape", { width: size.width, height: size.height, regionWidth: size.width + 2, regionHeight: size.height + 2 });
    run(invoke("notch_shape", { width: size.width, height: size.height, regionWidth: NOTCH_WINDOW.width, regionHeight: NOTCH_WINDOW.height }), "notch_shape");
    const timer = window.setTimeout(() => run(settle(), "notch_shape"), SHAPE_SETTLE_MS);
    return () => window.clearTimeout(timer);
  }, [size.width, size.height]);

  const tabs: { id: Tab; label: string; glyph: ReactNode }[] = [
    { id: "nook", label: "Nook", glyph: <Music aria-hidden="true" className="size-3.5" /> },
    { id: "tray", label: "Tray", glyph: <ClipboardList aria-hidden="true" className="size-3.5" /> },
    ...(widgets.length ? [{ id: "widgets" as Tab, label: "Widgets", glyph: <span aria-hidden="true" className="size-1.5 rounded-full bg-accent" /> }] : []),
  ];
  const expandedSize = notchSize("expanded", playing);
  const volumeSize = notchSize("volume", playing);

  return (
    <div className="notch-surface flex h-screen w-screen justify-center text-fg select-none">
      <div
        className="notch-shape relative shrink-0 overflow-hidden bg-notch"
        data-closing={growing ? undefined : ""}
        style={{ width: size.width, height: size.height, borderBottomLeftRadius: size.radius, borderBottomRightRadius: size.radius }}
      >
        <div
          className="notch-layer flex items-center justify-between px-3"
          data-shown={mode === "collapsed" ? "" : undefined}
          aria-hidden={mode !== "collapsed"}
          style={{ width: collapsedSize.width, height: collapsedSize.height }}
        >
          {playing && media && (
            <>
              <Artwork media={media} className="size-5 rounded-[5px]" />
              <Equalizer />
            </>
          )}
        </div>
        <div className="notch-layer" data-shown={mode === "volume" ? "" : undefined} aria-hidden={mode !== "volume"} style={{ width: volumeSize.width, height: volumeSize.height }}>
          {volume && <VolumeView volume={volume} />}
        </div>
        <div
          className="notch-layer flex flex-col gap-3 px-4 pt-2.5 pb-4"
          data-shown={mode === "expanded" ? "" : undefined}
          aria-hidden={mode !== "expanded"}
          inert={mode !== "expanded"}
          style={{ width: expandedSize.width, height: expandedSize.height }}
        >
          <header className="flex h-7 shrink-0 items-center justify-between">
            <div role="tablist" aria-label="Notch sections" className="flex items-center gap-1">
              {tabs.map((t) => (
                <button
                  key={t.id}
                  type="button"
                  role="tab"
                  aria-selected={tab === t.id}
                  onClick={() => setTab(t.id)}
                  className={`flex h-7 items-center gap-1.5 rounded-full px-3 text-12 font-medium outline-offset-1 transition-colors duration-150 focus-visible:outline-2 focus-visible:outline-accent ${tab === t.id ? "bg-control text-fg" : "text-fg-60 hover:text-fg"}`}
                >
                  {t.glyph}
                  {t.label}
                </button>
              ))}
            </div>
            <div className="flex items-center gap-2">
              <BatteryChip battery={battery} />
              <IconButton label="Open Openray settings" onClick={() => run(invoke("open_settings"), "open_settings")}>
                <Settings className="size-4" />
              </IconButton>
            </div>
          </header>
          <div key={tab} role="tabpanel" aria-label={tabs.find((t) => t.id === tab)?.label} className="notch-panel min-h-0 flex-1">
            {tab === "nook" && (
              <div className="grid h-full grid-cols-[1.35fr_1fr_0.75fr] items-center gap-5">
                <NowPlaying media={media} />
                <CalendarStrip focus={focus} expanded={expanded} />
                <Captures />
              </div>
            )}
            {tab === "tray" && <Tray entries={clipboard} />}
            {tab === "widgets" && <Widgets widgets={widgets} />}
          </div>
        </div>
      </div>
    </div>
  );
}
