export type CaptureMode = "region" | "color" | "ocr" | "qr" | "pin" | "ruler";

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface CaptureStart {
  mode: CaptureMode;
  width: number;
  height: number;
  windows: Rect[];
  monitors: Rect[];
  cursor: [number, number];
  nonce: number;
}

export const MODE_HINTS: Record<CaptureMode, string> = {
  region: "Drag to select an area, or click a window",
  color: "Click to copy the color under the cursor",
  ocr: "Select the text to copy",
  qr: "Select the QR code to read",
  pin: "Select the area to pin to the screen",
  ruler: "Drag to measure",
};

export function rectFrom(a: [number, number], b: [number, number]): Rect {
  const x = Math.min(a[0], b[0]);
  const y = Math.min(a[1], b[1]);
  return { x, y, w: Math.abs(a[0] - b[0]) + 1, h: Math.abs(a[1] - b[1]) + 1 };
}

export function targetAt(point: [number, number], windows: Rect[], monitors: Rect[]): Rect | null {
  const inside = (r: Rect) => point[0] >= r.x && point[1] >= r.y && point[0] < r.x + r.w && point[1] < r.y + r.h;
  return windows.find(inside) ?? monitors.find(inside) ?? null;
}

export function clampToScreen(rect: Rect, width: number, height: number): Rect | null {
  const x = Math.max(0, rect.x);
  const y = Math.max(0, rect.y);
  const right = Math.min(width, rect.x + rect.w);
  const bottom = Math.min(height, rect.y + rect.h);
  return right > x && bottom > y ? { x, y, w: right - x, h: bottom - y } : null;
}

export function measure(rect: Rect): string {
  return `${rect.w} × ${rect.h}  ·  ${Math.round(Math.hypot(rect.w, rect.h))} px`;
}
