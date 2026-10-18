import { useState } from "react";
import type { IconSpec, IconTone } from "./extension-api";

const TONES: Record<IconTone, string> = {
  blue: "from-icon-blue-top to-icon-blue-bottom",
  orange: "from-icon-orange-top to-icon-orange-bottom",
  red: "from-icon-red-top to-icon-red-bottom",
  yellow: "from-icon-yellow-top to-icon-yellow-bottom",
  purple: "from-icon-purple-top to-icon-purple-bottom",
  green: "from-icon-green-top to-icon-green-bottom",
  pink: "from-icon-pink-top to-icon-pink-bottom",
  slate: "from-icon-slate-top to-icon-slate-bottom",
};

export function OpenrayGlyph({ className, strokeWidth = 2.5 }: { className?: string; strokeWidth?: number }) {
  return (
    <svg viewBox="0 0 24 24" className={className} aria-hidden="true">
      <circle cx="12" cy="12" r="8" fill="none" stroke="currentColor" strokeWidth={strokeWidth} />
      <path d="M12 12 L21 3" stroke="currentColor" strokeWidth={strokeWidth} strokeLinecap="round" />
    </svg>
  );
}

export function GlyphTile({ spec, small = false }: { spec: IconSpec; small?: boolean }) {
  const Glyph = spec.glyph;
  return (
    <span
      aria-hidden="true"
      className={`flex shrink-0 items-center justify-center bg-linear-to-b/oklch text-white shadow-[inset_0_0.5px_0_rgb(255_255_255/35%),inset_0_0_0_0.5px_rgb(0_0_0/20%)] ${TONES[spec.tone]} ${
        small ? "size-4 rounded-[4px]" : "size-[22px] rounded-[6px]"
      }`}
    >
      <Glyph strokeWidth={2.25} className={small ? "size-2.5" : "size-3.5"} />
    </span>
  );
}

export function CommandIcon({ title, icon, fallback, small = false }: { title: string; icon?: string | IconSpec; fallback?: IconSpec; small?: boolean }) {
  const [failed, setFailed] = useState(false);
  const size = small ? "size-4" : "size-[22px]";
  if (icon && typeof icon !== "string") return <GlyphTile spec={icon} small={small} />;
  if (icon && !failed) {
    return <img src={icon} alt="" loading="lazy" draggable={false} onError={() => setFailed(true)} className={`${size} shrink-0 rounded-[4px]`} />;
  }
  if (fallback) return <GlyphTile spec={fallback} small={small} />;
  return (
    <span aria-hidden="true" className={`flex ${size} shrink-0 items-center justify-center rounded-[5px] bg-key-bg text-11 font-semibold text-fg-60`}>
      {title.trim().charAt(0).toUpperCase()}
    </span>
  );
}
