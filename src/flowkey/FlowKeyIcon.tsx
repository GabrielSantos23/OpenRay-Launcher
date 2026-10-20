import type { LucideProps } from "lucide-react";
import { lazy, Suspense, useState, type ComponentType } from "react";
import names from "virtual:lucide-names";
import { hostAllowed } from "./gate";
import type { InstalledExtension } from "./runtime";

export interface IconFields {
  icon?: string;
  iconName?: string;
  iconColor?: string;
  iconUri?: string;
  iconSvg?: string;
}

const LOCAL_IMAGE_ORIGINS = ["http://extimg.localhost/", "http://appicon.localhost/", "http://clipimg.localhost/"];
const IMAGE_FILE = /\.(png|jpe?g|gif|webp|ico|bmp|svg)$/i;
const COLOR = /^(#[0-9a-f]{3,8}|rgba?\([\d\s.,%]+\)|[a-z]+)$/i;

const NAMES = new Set(names);
const LOADED = new Map<string, ComponentType<LucideProps>>();

export function iconKey(name: string): string {
  return name
    .trim()
    .replace(/([a-z0-9])([A-Z])/g, "$1-$2")
    .replace(/([a-zA-Z])([0-9])/g, "$1-$2")
    .replace(/[_\s]+/g, "-")
    .toLowerCase();
}

export function lucideIcon(name: string): ComponentType<LucideProps> | null {
  const key = iconKey(name);
  if (!NAMES.has(key)) return null;
  let icon = LOADED.get(key);
  if (!icon) {
    const Lazy = lazy(() => import("./lucideLoaders").then((m) => m.loadIcon(key)));
    icon = (props: LucideProps) => (
      <Suspense fallback={<span className={props.className} />}>
        <Lazy {...props} />
      </Suspense>
    );
    LOADED.set(key, icon);
  }
  return icon;
}

export function safeColor(color?: string): string | undefined {
  return color && COLOR.test(color.trim()) ? color.trim() : undefined;
}

export function resolveImage(src: string | undefined, extension: InstalledExtension): string | null {
  if (!src) return null;
  if (/^data:image\/(png|jpe?g|gif|webp|bmp|svg\+xml)[;,]/i.test(src)) return src;
  if (LOCAL_IMAGE_ORIGINS.some((origin) => src.startsWith(origin)) || src.startsWith(extension.baseUrl)) return src;
  if (IMAGE_FILE.test(src) && !src.includes(":") && !src.includes("..") && !src.startsWith("/")) return `${extension.baseUrl}${src.split("/").map(encodeURIComponent).join("/")}`;
  try {
    const url = new URL(src);
    return hostAllowed(url, extension.manifest.httpHosts ?? []) ? url.href : null;
  } catch {
    return null;
  }
}

export function FlowKeyIcon({ fields, extension, size = "row", title }: { fields: IconFields; extension: InstalledExtension; size?: "row" | "small" | "tile" | "large"; title?: string }) {
  const [failed, setFailed] = useState(false);
  const box = { small: "size-4 text-[13px]", row: "size-[22px] text-[17px]", tile: "size-full text-[44px]", large: "size-12 text-[36px]" }[size];
  const glyph = { small: "size-4", row: "size-[18px]", tile: "size-1/2", large: "size-9" }[size];
  const color = safeColor(fields.iconColor);
  const uri = resolveImage(fields.iconUri ?? (fields.icon && IMAGE_FILE.test(fields.icon) ? fields.icon : undefined), extension);
  if (uri && !failed) {
    return <img src={uri} alt="" draggable={false} loading="lazy" onError={() => setFailed(true)} className={`${box} shrink-0 rounded-[4px] object-cover`} />;
  }
  if (fields.iconSvg) {
    const mask = `url("data:image/svg+xml;utf8,${encodeURIComponent(fields.iconSvg)}") center / contain no-repeat`;
    return (
      <span aria-hidden="true" className={`${box} flex shrink-0 items-center justify-center`}>
        <span style={{ mask, WebkitMask: mask, background: color ?? "currentColor" }} className={`${glyph} text-fg`} />
      </span>
    );
  }
  const name = fields.iconName ?? fields.icon;
  const Lucide = name ? lucideIcon(name) : null;
  if (Lucide) {
    return (
      <span aria-hidden="true" className={`${box} flex shrink-0 items-center justify-center text-fg`}>
        <Lucide style={color ? { color } : undefined} className={glyph} strokeWidth={2} />
      </span>
    );
  }
  if (fields.icon && !IMAGE_FILE.test(fields.icon)) {
    return (
      <span aria-hidden="true" className={`${box} flex shrink-0 items-center justify-center leading-none`}>
        {fields.icon}
      </span>
    );
  }
  return (
    <span aria-hidden="true" className={`${box} flex shrink-0 items-center justify-center rounded-[5px] bg-key-bg text-11 font-semibold text-fg-60`}>
      {(title ?? "").trim().charAt(0).toUpperCase()}
    </span>
  );
}

const ACTION_ICONS: [RegExp, string][] = [
  [/^(play|resume)\b/i, "play"],
  [/^pause\b/i, "pause"],
  [/^dislike\b|^unlike\b|remove from liked/i, "heart-off"],
  [/^like\b/i, "heart"],
  [/^next\b|^skip\b/i, "forward"],
  [/^previous\b/i, "rewind"],
  [/radio/i, "radio"],
  [/^copy (url|link)/i, "link"],
  [/embed/i, "code"],
  [/^copy\b/i, "copy"],
  [/^paste\b/i, "clipboard-paste"],
  [/search (the )?web|in browser|on the web/i, "globe"],
  [/^open\b/i, "external-link"],
  [/refresh|reload/i, "refresh-cw"],
  [/search again/i, "search"],
  [/restart|retry|again\b/i, "rotate-ccw"],
  [/remove from library/i, "minus"],
  [/^(remove|delete|uninstall|clear)\b/i, "trash-2"],
  [/add to queue/i, "list-end"],
  [/playlist/i, "list-plus"],
  [/^(add|create|new|save)\b/i, "plus"],
  [/album|grid/i, "layout-grid"],
  [/popular/i, "list-music"],
  [/^show\b|details|list\b/i, "list"],
  [/connect|device/i, "smartphone"],
  [/library/i, "library"],
  [/search/i, "search"],
  [/lyrics/i, "mic-vocal"],
  [/settings|preferences|configure/i, "settings"],
  [/download/i, "download"],
  [/share/i, "share-2"],
  [/^(go|back)\b/i, "arrow-left"],
];

export function actionIcon(title: string, name?: string) {
  const explicit = name ? lucideIcon(name) : null;
  if (explicit) return explicit;
  const match = ACTION_ICONS.find(([pattern]) => pattern.test(title.trim()));
  return match ? lucideIcon(match[1]) : null;
}
