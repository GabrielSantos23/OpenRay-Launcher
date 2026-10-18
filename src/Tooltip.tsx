import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { Keys } from "./ActionPanel";
import { shortcutLabels } from "./actions";
import type { Shortcut } from "./extension-api";

interface TooltipContent {
  title: string;
  keys?: string[];
}

const OPEN_DELAY_MS = 500;
const INSTANT_WINDOW_MS = 400;

export function Tooltip() {
  const [tip, setTip] = useState<TooltipContent | null>(null);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unlisten = listen<TooltipContent>("tooltip-show", (event) => setTip({ ...event.payload }));
    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  useLayoutEffect(() => {
    const rect = ref.current?.getBoundingClientRect();
    if (tip && rect) invoke("tooltip_present", { width: Math.ceil(rect.width), height: Math.ceil(rect.height) });
  }, [tip]);

  if (!tip) return null;
  return (
    <div ref={ref} role="tooltip" className="inline-flex h-7 items-center gap-2 bg-panel px-2 text-12 whitespace-nowrap text-fg">
      {tip.title}
      {tip.keys && <Keys labels={tip.keys} />}
    </div>
  );
}

let lastClosed = 0;

export function TipButton({
  label,
  shortcut,
  onClick,
  children,
  className,
  active = false,
}: {
  label: string;
  shortcut?: Shortcut;
  onClick: () => void;
  children: ReactNode;
  className: string;
  active?: boolean;
}) {
  const timer = useRef(0);
  const shown = useRef(false);

  function open(target: HTMLElement) {
    const r = target.getBoundingClientRect();
    shown.current = true;
    invoke("tooltip_show", { tooltip: { title: label, keys: shortcut && shortcutLabels(shortcut) }, rect: [r.left, r.top, r.width, r.height] });
  }

  function close() {
    window.clearTimeout(timer.current);
    if (!shown.current) return;
    shown.current = false;
    lastClosed = Date.now();
    invoke("tooltip_hide");
  }

  useEffect(() => close, []);

  return (
    <button
      type="button"
      tabIndex={-1}
      aria-label={label}
      aria-pressed={active || undefined}
      onMouseEnter={(e) => {
        const target = e.currentTarget;
        const delay = Date.now() - lastClosed < INSTANT_WINDOW_MS ? 0 : OPEN_DELAY_MS;
        timer.current = window.setTimeout(() => open(target), delay);
      }}
      onMouseLeave={close}
      onMouseDown={(e) => e.preventDefault()}
      onClick={() => {
        close();
        onClick();
      }}
      className={className}
    >
      {children}
    </button>
  );
}
