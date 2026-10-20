import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, PhysicalSize } from "@tauri-apps/api/window";
import { useEffect, useRef } from "react";
import { showHUD } from "./toast";

export function PinWindow() {
  const window_ = getCurrentWindow();
  const label = window_.label;
  const base = useRef<{ w: number; h: number } | null>(null);
  const zoom = useRef(1);

  useEffect(() => {
    window_.innerSize().then((size) => (base.current = { w: size.width, h: size.height }));
  }, [window_]);

  const close = () => invoke("pin_close", { label });

  return (
    <div
      role="img"
      aria-label="Pinned screenshot. Drag to move, scroll to zoom, Ctrl+C to copy, Esc or double-click to close."
      tabIndex={-1}
      ref={(el) => el?.focus()}
      data-tauri-drag-region
      onDoubleClick={close}
      onKeyDown={(event) => {
        if (event.key === "Escape") close();
        else if (event.ctrlKey && event.key.toLowerCase() === "c") invoke("pin_copy", { label }).then(() => showHUD({ variant: "success", title: "Copied Image to Clipboard" }));
      }}
      onWheel={(event) => {
        if (!base.current) return;
        zoom.current = Math.min(4, Math.max(0.2, zoom.current * (event.deltaY < 0 ? 1.1 : 1 / 1.1)));
        window_.setSize(new PhysicalSize(Math.round(base.current.w * zoom.current), Math.round(base.current.h * zoom.current)));
      }}
      className="fixed inset-0 cursor-move overflow-hidden bg-focus-overlay outline-none"
    >
      <img
        src={convertFileSrc(`pin/${label}`, "capture")}
        alt=""
        draggable={false}
        onLoad={() => invoke("pin_shown", { label })}
        className="pointer-events-none size-full object-fill"
      />
    </div>
  );
}
