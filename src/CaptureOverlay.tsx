import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { clampToScreen, MODE_HINTS, measure, rectFrom, targetAt, type CaptureStart, type Rect } from "./capture";
import { showHUD } from "./toast";

const ZOOM_PIXELS = 15;
const LOUPE = 120;

export function CaptureOverlay() {
  const [start, setStart] = useState<CaptureStart | null>(null);
  const [cursor, setCursor] = useState<[number, number]>([0, 0]);
  const [anchor, setAnchor] = useState<[number, number] | null>(null);
  const [ruler, setRuler] = useState<Rect | null>(null);
  const [color, setColor] = useState("#000000");
  const dpr = window.devicePixelRatio || 1;

  useEffect(() => {
    const begin = (payload: CaptureStart) => {
      setStart(payload);
      setCursor(payload.cursor);
      setAnchor(null);
      setRuler(null);
    };
    const unlisten = listen<CaptureStart>("capture-start", (event) => begin(event.payload));
    invoke<CaptureStart | null>("capture_pending").then((payload) => payload && begin(payload));
    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  useEffect(() => {
    if (!start) return;
    let live = true;
    invoke<string | null>("capture_pixel", { x: cursor[0], y: cursor[1] }).then((hex) => live && setColor(hex ?? ""));
    return () => {
      live = false;
    };
  }, [cursor, start]);

  if (!start) return null;

  const mode = start.mode;
  const frameUrl = `${convertFileSrc("frame", "capture")}?n=${start.nonce}`;
  const css = (value: number) => value / dpr;
  const point = (event: React.MouseEvent): [number, number] => [Math.round(event.clientX * dpr), Math.round(event.clientY * dpr)];
  const selection = anchor ? rectFrom(anchor, cursor) : ruler;
  const hovered = !anchor && !ruler && mode !== "color" && mode !== "ruler" ? targetAt(cursor, start.windows, start.monitors) : null;
  const shown = selection ?? hovered;
  const monitor = targetAt(cursor, [], start.monitors) ?? { x: 0, y: 0, w: start.width, h: start.height };

  function close() {
    setStart(null);
    setAnchor(null);
  }

  function cancel() {
    invoke("capture_cancel").catch(() => {});
    close();
  }

  function finish(rect: Rect | null) {
    const clamped = rect && clampToScreen(rect, start!.width, start!.height);
    if (!clamped) return cancel();
    close();
    invoke("capture_finish", { rect: clamped }).catch((e) => showHUD({ variant: "failure", title: String(e) }));
  }

  function onMouseDown(event: React.MouseEvent) {
    event.preventDefault();
    if (event.button === 2) return cancel();
    if (event.button !== 0) return;
    const at = point(event);
    setCursor(at);
    if (mode === "color") return finish({ x: at[0], y: at[1], w: 1, h: 1 });
    setRuler(null);
    setAnchor(at);
  }

  function onMouseUp(event: React.MouseEvent) {
    if (!anchor || event.button !== 0) return;
    const rect = rectFrom(anchor, point(event));
    setAnchor(null);
    if (mode === "ruler") return setRuler(rect);
    if (rect.w < 4 && rect.h < 4) return finish(targetAt(point(event), start!.windows, start!.monitors));
    finish(rect);
  }

  const loupeLeft = css(cursor[0]) + 20 + LOUPE > css(start.width) ? css(cursor[0]) - 20 - LOUPE : css(cursor[0]) + 20;
  const loupeTop = css(cursor[1]) + 20 + LOUPE + 44 > css(start.height) ? css(cursor[1]) - 20 - LOUPE - 44 : css(cursor[1]) + 20;
  const label = shown && (mode === "ruler" ? measure(shown) : `${shown.w} × ${shown.h}`);

  return (
    <div
      role="application"
      aria-label={`Openray capture: ${MODE_HINTS[mode]}`}
      tabIndex={-1}
      ref={(el) => el?.focus()}
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          cancel();
        } else if (event.key === "Enter" && mode === "ruler" && ruler) {
          event.preventDefault();
          invoke("copy_text", { text: `${ruler.w}x${ruler.h}` }).then(() => showHUD({ variant: "success", title: `Copied ${ruler.w}x${ruler.h}` }));
          cancel();
        }
      }}
      onMouseDown={onMouseDown}
      onMouseMove={(event) => setCursor(point(event))}
      onMouseUp={onMouseUp}
      onContextMenu={(event) => event.preventDefault()}
      className="fixed inset-0 cursor-crosshair overflow-hidden bg-focus-overlay outline-none select-none"
    >
      <img
        key={start.nonce}
        src={frameUrl}
        alt=""
        draggable={false}
        onLoad={() => invoke("capture_ready")}
        onError={() => {
          showHUD({ variant: "failure", title: "Could not start the capture" });
          cancel();
        }}
        className="pointer-events-none absolute inset-0 size-full"
      />
      {shown ? (
        <div
          className="pointer-events-none absolute border border-accent shadow-[0_0_0_100000px_var(--color-capture-dim)]"
          style={{ left: css(shown.x), top: css(shown.y), width: css(shown.w), height: css(shown.h) }}
        />
      ) : (
        <div className="pointer-events-none absolute inset-0 bg-capture-dim" />
      )}
      {mode !== "color" && !anchor && (
        <>
          <div className="pointer-events-none absolute inset-y-0 w-px bg-accent/50" style={{ left: css(cursor[0]) }} />
          <div className="pointer-events-none absolute inset-x-0 h-px bg-accent/50" style={{ top: css(cursor[1]) }} />
        </>
      )}
      {shown && label && (
        <span
          className="pointer-events-none absolute rounded-[5px] bg-panel px-1.5 py-0.5 text-12 font-medium whitespace-nowrap text-fg tabular-nums"
          style={{ left: css(shown.x), top: css(shown.y) > 28 ? css(shown.y) - 26 : css(shown.y + shown.h) + 6 }}
        >
          {label}
        </span>
      )}
      <div
        className="pointer-events-none absolute flex flex-col overflow-hidden rounded-[8px] border border-panel-border bg-panel shadow-panel"
        style={{ left: loupeLeft, top: loupeTop, width: LOUPE }}
      >
        <span className="relative block" style={{ width: LOUPE, height: LOUPE }}>
          <span
            className="block size-full bg-no-repeat [image-rendering:pixelated]"
            style={{
              backgroundImage: `url(${frameUrl})`,
              backgroundSize: `${start.width * (LOUPE / ZOOM_PIXELS)}px ${start.height * (LOUPE / ZOOM_PIXELS)}px`,
              backgroundPosition: `${-(cursor[0] - 7) * (LOUPE / ZOOM_PIXELS)}px ${-(cursor[1] - 7) * (LOUPE / ZOOM_PIXELS)}px`,
            }}
          />
          <span className="absolute border border-fg" style={{ left: (LOUPE / ZOOM_PIXELS) * 7, top: (LOUPE / ZOOM_PIXELS) * 7, width: LOUPE / ZOOM_PIXELS, height: LOUPE / ZOOM_PIXELS }} />
        </span>
        <span className="flex items-center justify-between gap-1 px-2 py-1 text-11 text-fg-60 tabular-nums">
          <span>
            {cursor[0]}, {cursor[1]}
          </span>
          {color && (
            <span className="flex items-center gap-1 font-medium text-fg">
              <span className="size-2.5 rounded-[2px] border border-panel-border" style={{ background: color }} />
              {color}
            </span>
          )}
        </span>
      </div>
      <div
        className="pointer-events-none absolute -translate-x-1/2 rounded-[8px] border border-panel-border bg-panel px-3 py-1.5 text-13 text-fg shadow-panel"
        style={{ left: css(monitor.x + monitor.w / 2), top: css(monitor.y) + 16 }}
      >
        {MODE_HINTS[mode]}
        <span className="ml-2 text-fg-60">{mode === "ruler" ? "Enter to copy · Esc to close" : "Esc to cancel"}</span>
      </div>
    </div>
  );
}
