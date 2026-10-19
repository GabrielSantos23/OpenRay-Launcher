import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { CircleCheck, CirclePause, CirclePlay, Ellipsis, GripVertical, Target, X, type LucideIcon } from "lucide-react";
import { useEffect, useState } from "react";
import { elapsedFraction, FOCUS_ICON, formatTimer, pauseRemainingMs, remainingMs, timerStep, type FocusSession } from "./focus";
import { GlyphTile } from "./icons";

function useFocusSession() {
  const [session, setSession] = useState<FocusSession | null>(null);
  useEffect(() => {
    invoke<FocusSession | null>("focus_get").then(setSession);
    const unlisten = listen<FocusSession | null>("focus-session-changed", (event) => setSession(event.payload));
    return () => {
      unlisten.then((off) => off());
    };
  }, []);
  return session;
}

function useNow(stepMs: number) {
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), Math.min(stepMs, 1000));
    return () => window.clearInterval(timer);
  }, [stepMs]);
  return now;
}

function PanelButton({ icon: Icon, text, label, onClick }: { icon: LucideIcon; text?: string; label?: string; onClick: (rect: DOMRect) => void }) {
  return (
    <button
      type="button"
      tabIndex={-1}
      aria-label={label}
      onPointerDown={(e) => e.preventDefault()}
      onClick={(e) => onClick(e.currentTarget.getBoundingClientRect())}
      className={`flex h-6 shrink-0 cursor-default items-center gap-1.5 rounded-[6px] bg-control text-12 font-medium text-fg hover:bg-control-hover ${text ? "pr-1.5 pl-2" : "w-6 justify-center"}`}
    >
      {text}
      <Icon aria-hidden="true" className="size-3.5" />
    </button>
  );
}

function menu(name: "pause" | "more", rect: DOMRect) {
  invoke("focus_panel_menu", { menu: name, x: rect.left, y: rect.top - 6 }).catch((e) => console.error("focus menu failed", e));
}

export function FocusPanel() {
  const session = useFocusSession();
  const [hovered, setHovered] = useState(false);
  useEffect(() => {
    const unlisten = listen<boolean>("focus-panel-hover", (event) => setHovered(event.payload));
    return () => {
      unlisten.then((off) => off());
    };
  }, []);
  const now = useNow(session ? timerStep(remainingMs(session, Date.now())) : 1000);
  if (!session) return null;
  const paused = session.status === "paused";
  const pauseLeft = pauseRemainingMs(session, now);
  const shown = pauseLeft ?? remainingMs(session, now);
  const step = timerStep(shown);
  const noLimit = session.durationMs === null && (pauseLeft === null || pauseLeft <= 0);
  const progress = elapsedFraction(session, now);
  const run = (command: string) => invoke(command).catch((e) => console.error(command, e));

  return (
    <main data-hovered={hovered} className="group relative flex h-screen w-screen items-center overflow-hidden bg-panel text-13 text-fg select-none">
      <div
        aria-hidden="true"
        style={{ width: `${Math.round(progress * 100)}%` }}
        className={`absolute inset-y-0 left-0 transition-[width,background-color] duration-200 ease-in-out ${paused ? "bg-yellow/28" : "bg-green/28"}`}
      />
      <div className="absolute inset-0 flex items-center gap-2 px-2 transition-opacity duration-150 group-data-[hovered=true]:opacity-0">
        {noLimit ? (
          <GlyphTile spec={FOCUS_ICON} />
        ) : (
          <span className="flex h-6 shrink-0 items-center rounded-[6px] bg-control px-3 font-mono text-12 text-fg-60">{formatTimer(shown <= 0 ? 0 : Math.ceil(shown / step) * step)}</span>
        )}
        <span className="truncate">{session.title}</span>
      </div>
      <div className="absolute inset-0 flex items-center gap-1 px-2 opacity-0 transition-opacity duration-150 group-data-[hovered=true]:opacity-100">
        <PanelButton icon={paused ? CirclePlay : CirclePause} text={paused ? "Resume" : "Pause"} onClick={(rect) => (paused ? run("focus_resume") : menu("pause", rect))} />
        <PanelButton icon={CircleCheck} text="Complete" onClick={() => run("focus_complete")} />
        <PanelButton icon={Ellipsis} label="More" onClick={(rect) => menu("more", rect)} />
        <div
          role="button"
          tabIndex={-1}
          aria-label="Drag Focus Panel"
          title="Drag to move, double-click to reset"
          onMouseDown={(e) => {
            e.preventDefault();
            if (e.button === 0 && e.detail === 1) void getCurrentWindow().startDragging();
          }}
          onDoubleClick={() => run("focus_panel_reset")}
          className="ml-auto flex h-6 w-5 shrink-0 cursor-grab items-center justify-center text-fg-60"
        >
          <GripVertical aria-hidden="true" className="size-4" />
        </div>
      </div>
    </main>
  );
}

interface ToastPayload {
  title: string;
  message?: string;
  snooze?: boolean;
}

const TOAST_MS = 6000;

export function FocusToast() {
  const [toast, setToast] = useState<ToastPayload | null>(null);
  useEffect(() => {
    const unlisten = listen<ToastPayload>("focus-toast", (event) => setToast(event.payload));
    return () => {
      unlisten.then((off) => off());
    };
  }, []);
  useEffect(() => {
    if (!toast || toast.snooze !== undefined) return;
    const timer = window.setTimeout(() => invoke("focus_toast_hide"), TOAST_MS);
    return () => window.clearTimeout(timer);
  }, [toast]);
  if (!toast) return null;
  return (
    <main role="status" className="relative flex h-screen w-screen items-center gap-3 bg-panel pr-9 pl-4 text-fg select-none">
      <GlyphTile spec={FOCUS_ICON} />
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <h1 className="truncate text-13 font-semibold">{toast.title}</h1>
        {toast.message && <p className="truncate text-12 text-fg-60">{toast.message}</p>}
      </div>
      {toast.snooze && (
        <button
          type="button"
          onClick={() => invoke("focus_snooze").catch((e) => console.error("snooze failed", e))}
          className="h-7 shrink-0 rounded-[6px] bg-control px-3 text-12 font-medium hover:bg-control-hover"
        >
          Snooze
        </button>
      )}
      <button
        type="button"
        aria-label="Close"
        onClick={() => invoke("focus_toast_hide")}
        className="absolute top-2 right-2 flex size-5 items-center justify-center rounded text-fg-60 hover:bg-control hover:text-fg"
      >
        <X aria-hidden="true" className="size-3.5" />
      </button>
    </main>
  );
}

interface BlockPayload {
  key: string;
  name: string;
  website: boolean;
  snoozeMs: number | null;
}

export function FocusBlock() {
  const [block, setBlock] = useState<BlockPayload | null>(null);
  useEffect(() => {
    invoke<BlockPayload | null>("focus_block_info").then((info) => info && setBlock(info));
    const unlisten = listen<BlockPayload>("focus-block", (event) => setBlock(event.payload));
    return () => {
      unlisten.then((off) => off());
    };
  }, []);
  if (!block) return <main className="h-screen w-screen bg-focus-overlay" />;
  const minutes = block.snoozeMs ? Math.round(block.snoozeMs / 60_000) : 0;
  const snooze = () => invoke("focus_snooze").catch((e) => console.error("snooze failed", e));
  return (
    <main className="flex h-screen w-screen flex-col items-center justify-center bg-focus-overlay text-fg select-none">
      <Target aria-hidden="true" className="size-4 text-fg-60" />
      <h1 className="mt-3 max-w-[80%] truncate text-15 font-semibold">{block.name} has been blocked by Openray</h1>
      <div className="mt-4 flex gap-2">
        {minutes > 0 && (
          <button type="button" autoFocus onClick={snooze} className="h-[30px] min-w-[180px] rounded-[6px] bg-control px-4 text-12 font-medium hover:bg-control-hover">
            Snooze for {minutes} {minutes === 1 ? "minute" : "minutes"}
          </button>
        )}
        {block.website && (
          <button
            type="button"
            onClick={() => invoke("focus_close_tab").catch((e) => console.error("close tab failed", e))}
            className="h-[30px] rounded-[6px] bg-control px-4 text-12 font-medium hover:bg-control-hover"
          >
            Close Tab
          </button>
        )}
      </div>
      <p className="mt-5 text-11 text-fg-60">
        {block.website ? "Switching focus to another window will hide this page." : "Switching focus to another window will close this app."}
      </p>
    </main>
  );
}
