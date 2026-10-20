import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { Overlay, useAnchorRect } from "./Dropdown";
import { hotkeyFromEvent, hotkeyLabels } from "./settings";

export interface HotkeyConflict {
  name: string;
  overwritable: boolean;
}

interface HotkeyRecorderProps {
  value: string;
  label: string;
  findConflict: (hotkey: string) => HotkeyConflict | null;
  onSave: (hotkey: string, overwrite: boolean) => Promise<void>;
  onClear?: () => Promise<void>;
  variant: "button" | "cell";
}

type Result = { state: "idle" } | { state: "conflict"; hotkey: string; conflict: HotkeyConflict } | { state: "invalid"; labels: string[]; message?: string };

const POPOVER_WIDTH = 320;
const POPOVER_HEIGHT = 170;
const SAVED_FLASH_MS = 200;
const MODIFIER_KEYS = new Set(["Control", "Alt", "Shift", "Meta"]);

function heldModifiers(event: { ctrlKey: boolean; altKey: boolean; shiftKey: boolean; metaKey: boolean }): string[] {
  return [event.ctrlKey && "Ctrl", event.altKey && "Alt", event.shiftKey && "Shift", event.metaKey && "Win"].filter((m): m is string => !!m);
}

function KeyCap({ children, variant = "default" }: { children: ReactNode; variant?: "default" | "hint" | "conflict" }) {
  return (
    <kbd
      className={`flex h-6 min-w-6 items-center justify-center rounded-[4px] px-1.5 font-sans text-12 font-medium text-fg shadow-[inset_0_-0.5px_0_0_rgb(0_0_0/80%),inset_0_0.5px_0_0_rgb(255_255_255/30%)] ${
        variant === "conflict" ? "bg-red/10" : "bg-control"
      } ${variant === "hint" ? "opacity-50" : ""}`}
    >
      {children}
    </kbd>
  );
}

function Message({ children, tone = "secondary" }: { children: ReactNode; tone?: "secondary" | "tertiary" | "red" }) {
  const color = tone === "red" ? "text-red" : tone === "tertiary" ? "text-fg-40" : "text-fg-60";
  return <span className={`flex max-w-full items-center gap-1 truncate px-3 text-center ${color}`}>{children}</span>;
}

function Keys({ labels, variant }: { labels: string[]; variant?: "default" | "hint" | "conflict" }) {
  return (
    <div className="flex min-h-6 items-center justify-center gap-1">
      {labels.map((label, i) => (
        <KeyCap key={`${label}-${i}`} variant={variant}>
          {label}
        </KeyCap>
      ))}
    </div>
  );
}

export function HotkeyRecorder({ value, label, findConflict, onSave, onClear, variant }: HotkeyRecorderProps) {
  const [open, setOpen] = useState(false);
  const [held, setHeld] = useState<string[]>([]);
  const [result, setResult] = useState<Result>({ state: "idle" });
  const [saved, setSaved] = useState(false);
  const [triggerRef, rect] = useAnchorRect<HTMLButtonElement>(open);
  const contentRef = useRef<HTMLDivElement>(null);
  const downCode = useRef("");
  const clearable = !!onClear && !!value;

  useEffect(() => {
    if (!open) return;
    setHeld([]);
    setResult({ state: "idle" });
    contentRef.current?.focus();
    invoke("hotkeys_suspend", { suspended: true });
    return () => {
      invoke("hotkeys_suspend", { suspended: false });
    };
  }, [open]);

  function close() {
    setOpen(false);
    triggerRef.current?.focus();
  }

  async function save(hotkey: string, overwrite: boolean) {
    try {
      await onSave(hotkey, overwrite);
      setResult({ state: "idle" });
      setSaved(true);
      window.setTimeout(() => setSaved(false), SAVED_FLASH_MS);
    } catch (e) {
      setResult({ state: "invalid", labels: hotkeyLabels(hotkey), message: String(e) });
    }
  }

  function onKeyDown(event: React.KeyboardEvent) {
    event.preventDefault();
    if (event.type === "keydown") downCode.current = event.code;
    event.stopPropagation();
    if (MODIFIER_KEYS.has(event.key)) {
      setHeld(heldModifiers(event));
      return;
    }
    const modifiers = heldModifiers(event);
    const standalone = modifiers.length === 0 && hotkeyFromEvent(event);
    if (modifiers.length === 0 && !standalone) {
      if (event.key === "Escape") close();
      else if (event.key === "Backspace" && clearable) onClear!().then(close);
      else if (event.key === "Enter" && result.state === "conflict" && result.conflict.overwritable) save(result.hotkey, true);
      else if (event.key === "Enter") close();
      else setResult({ state: "invalid", labels: [event.key.length === 1 ? event.key.toUpperCase() : event.key] });
      return;
    }
    const hotkey = hotkeyFromEvent(event);
    if (!hotkey) {
      setResult({ state: "invalid", labels: [...modifiers, event.key.length === 1 ? event.key.toUpperCase() : event.key] });
      return;
    }
    const conflict = findConflict(hotkey);
    if (conflict) setResult({ state: "conflict", hotkey, conflict });
    else save(hotkey, false);
  }

  let body: ReactNode;
  if (result.state === "conflict") {
    body = (
      <>
        <Message tone="red">
          Already used by <strong className="font-semibold">{result.conflict.name}</strong>
        </Message>
        <Keys labels={hotkeyLabels(result.hotkey)} variant="conflict" />
        <Message tone="tertiary">{result.conflict.overwritable ? "Overwrite it or record a new hotkey" : "Discard or record a new hotkey"}</Message>
      </>
    );
  } else if (result.state === "invalid") {
    body = (
      <>
        <Message tone="red">{result.message ?? "This key combination isn't supported"}</Message>
        <Keys labels={result.labels} variant="conflict" />
        <Message tone="tertiary">Discard or record a new hotkey</Message>
      </>
    );
  } else if (held.length > 0) {
    body = (
      <>
        <Message>{" "}</Message>
        <Keys labels={held} />
        <Message>{" "}</Message>
      </>
    );
  } else if (value) {
    body = (
      <>
        <Message>{clearable ? "Press Backspace to delete" : " "}</Message>
        <Keys labels={hotkeyLabels(value)} />
        <Message tone="tertiary">{clearable ? "or record a new hotkey" : "Record a new hotkey"}</Message>
      </>
    );
  } else {
    body = (
      <>
        <Message>e.g.</Message>
        <Keys labels={["Ctrl", "Alt", "A"]} variant="hint" />
        <Message tone="tertiary">Single Step</Message>
      </>
    );
  }

  const tint = saved ? "recorder-saved" : result.state === "idle" ? "" : "recorder-conflict";
  const above = rect ? rect.top >= POPOVER_HEIGHT + 12 : true;
  const left = rect ? Math.min(Math.max(8, rect.left + rect.width / 2 - POPOVER_WIDTH / 2), window.innerWidth - POPOVER_WIDTH - 8) : 0;
  const arrowLeft = rect ? rect.left + rect.width / 2 - left : 0;

  const trigger =
    variant === "button" ? (
      <button
        ref={triggerRef}
        type="button"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={`${label}: ${value || "none"}`}
        onClick={() => setOpen(true)}
        className={`h-6 rounded-[5px] bg-control px-3 font-medium text-fg outline-offset-2 hover:bg-control-hover ${open ? "outline-2 outline-accent" : ""}`}
      >
        {value ? hotkeyLabels(value).join(" ") : open ? "Recording…" : "Record Hotkey"}
      </button>
    ) : (
      <button
        ref={triggerRef}
        type="button"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={`${label}: ${value || "none"}`}
        onClick={() => setOpen(true)}
        className={`flex h-6 max-w-[145px] min-w-0 items-center gap-1.5 rounded-[5px] px-1.5 font-medium outline-offset-0 hover:shadow-[0_0_0_1px_var(--color-control-hover)] ${open ? "outline-2 outline-accent" : ""}`}
      >
        {value ? (
          <span className="flex gap-px">
            {hotkeyLabels(value).map((key, i) => (
              <kbd key={`${key}-${i}`} className="flex h-5 min-w-4 items-center justify-center rounded-[4px] bg-control px-0.5 font-sans text-12 text-fg">
                {key}
              </kbd>
            ))}
          </span>
        ) : (
          <span className="truncate text-fg-40">{open ? "Recording…" : "Record Hotkey"}</span>
        )}
      </button>
    );

  return (
    <span className="flex min-w-0 items-center gap-1">
      {trigger}
      {variant === "cell" && clearable && (
        <button
          type="button"
          aria-label={`Remove hotkey from ${label.replace(/^Hotkey for /, "")}`}
          title="Remove hotkey"
          onClick={() => onClear!()}
          className="flex size-4 shrink-0 items-center justify-center rounded text-fg-60 hover:text-fg"
        >
          <X className="size-3" />
        </button>
      )}
      {open && rect && (
        <>
          <Overlay onClose={close} />
          <div
            ref={contentRef}
            role="dialog"
            aria-label={`Record ${label}`}
            tabIndex={-1}
            onKeyDown={onKeyDown}
            onKeyUp={(e) => {
              if (e.code === "PrintScreen" && downCode.current !== "PrintScreen") onKeyDown(e);
              else setHeld(heldModifiers(e));
              downCode.current = "";
            }}
            style={{ left, width: POPOVER_WIDTH, height: POPOVER_HEIGHT, ...(above ? { bottom: window.innerHeight - rect.top + 10 } : { top: rect.bottom + 10 }) }}
            className="fixed z-40 rounded-panel border border-panel-border bg-panel text-13 shadow-panel outline-none"
          >
            <span
              aria-hidden="true"
              style={{ left: arrowLeft - 6 }}
              className={`absolute size-3 rotate-45 border-panel-border bg-panel ${above ? "-bottom-[7px] border-r border-b" : "-top-[7px] border-t border-l"}`}
            />
            <div className={`relative flex h-full flex-col items-center justify-center gap-3 overflow-hidden rounded-panel pt-2 ${tint}`}>{body}</div>
          </div>
        </>
      )}
    </span>
  );
}
