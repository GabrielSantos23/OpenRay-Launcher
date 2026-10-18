import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState, type ReactNode } from "react";
import { useSettings, type Settings } from "./settings";

export interface Theme {
  id: string;
  name: string;
  appearance: "dark" | "light";
  background: string;
  backgroundSecondary: string;
  foreground: string;
  accent: string;
  loader: string;
  red: string;
  yellow: string;
  green: string;
}

const ACCENT = "#4FA3F8";

export const THEMES: Theme[] = [
  { id: "bundled-raycast-dark", name: "Raycast Dark", appearance: "dark", background: "#000000", backgroundSecondary: "#000000", foreground: "#FFFFFF", accent: ACCENT, loader: "#FFFFFF", red: "#FF6363", yellow: "#FFC531", green: "#59D499" },
  { id: "bundled-raycast-light", name: "Raycast Light", appearance: "light", background: "#FFFFFF", backgroundSecondary: "#FFFFFF", foreground: "#000000", accent: "#138AF2", loader: "#000000", red: "#B12424", yellow: "#F8A300", green: "#006B4F" },
  { id: "bundled-sky", name: "Sky", appearance: "dark", background: "#0E1C3F", backgroundSecondary: "#173036", foreground: "#D7F0F4", accent: ACCENT, loader: "#47FFFF", red: "#ED5959", yellow: "#EDD559", green: "#47FF91" },
  { id: "bundled-sunset", name: "Sunset", appearance: "dark", background: "#451A08", backgroundSecondary: "#0A3943", foreground: "#FADDD1", accent: ACCENT, loader: "#47E0FF", red: "#ED5959", yellow: "#EDD559", green: "#59ED7A" },
  { id: "bundled-midnight", name: "Midnight", appearance: "dark", background: "#070034", backgroundSecondary: "#11005B", foreground: "#FAF0FF", accent: ACCENT, loader: "#F4C4FF", red: "#F84141", yellow: "#FEFF7F", green: "#63BA89" },
  { id: "bundled-dawn", name: "Dawn", appearance: "light", background: "#FFF1F7", backgroundSecondary: "#D0C1FF", foreground: "#120032", accent: ACCENT, loader: "#5A00FF", red: "#E14848", yellow: "#B7B400", green: "#00AC5D" },
  { id: "bundled-overcast", name: "Overcast", appearance: "light", background: "#D9F9FF", backgroundSecondary: "#C7E0FC", foreground: "#002369", accent: ACCENT, loader: "#0019FF", red: "#EE5050", yellow: "#EBA200", green: "#30B475" },
  { id: "bundled-noon", name: "Noon", appearance: "light", background: "#FFE7C6", backgroundSecondary: "#FCBFA5", foreground: "#1E003C", accent: ACCENT, loader: "#6100FF", red: "#DA4C4C", yellow: "#D49200", green: "#09B664" },
];

const BACKDROP = { dark: "#333333", light: "#cccccc" };
const PANEL_BACKDROP = { dark: "#333333", light: "#e6e6e6" };

const over = (color: string, alpha: number, base: string) => `color-mix(in srgb, ${color} ${alpha}%, ${base})`;

export function themeVariables(theme: Theme, systemAccent: string | null): Record<string, string> {
  const backdrop = BACKDROP[theme.appearance];
  return {
    "--color-backdrop": backdrop,
    "--color-bg": over(theme.background, 40, backdrop),
    "--color-bg-bottom": over(theme.backgroundSecondary, 40, backdrop),
    "--color-fg": theme.foreground,
    "--color-panel": over("#ffffff", 10, over(theme.backgroundSecondary, 40, PANEL_BACKDROP[theme.appearance])),
    "--color-loader": theme.loader,
    "--color-red": theme.red,
    "--color-yellow": theme.yellow,
    "--color-green": theme.green,
    "--color-accent": systemAccent ?? theme.accent,
  };
}

export function activeTheme(settings: Settings, systemDark: boolean): Theme {
  const dark = settings.appearance === "system" ? systemDark : settings.appearance === "dark";
  const id = dark ? settings.themeDarkId : settings.themeLightId;
  return THEMES.find((t) => t.id === id) ?? THEMES[dark ? 0 : 1];
}

function useSystemDark(): boolean {
  const [dark, setDark] = useState(() => window.matchMedia("(prefers-color-scheme: dark)").matches);
  useEffect(() => {
    const query = window.matchMedia("(prefers-color-scheme: dark)");
    const onChange = () => setDark(query.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);
  return dark;
}

export function ThemeRoot({ children }: { children: ReactNode }) {
  const [settings] = useSettings();
  const systemDark = useSystemDark();
  const [accent, setAccent] = useState<string | null>(null);

  useEffect(() => {
    invoke<string | null>("system_accent_color").then(setAccent, () => setAccent(null));
  }, []);

  useEffect(() => {
    if (!settings) return;
    const theme = activeTheme(settings, systemDark);
    const root = document.documentElement;
    for (const [name, value] of Object.entries(themeVariables(theme, accent))) root.style.setProperty(name, value);
    root.style.colorScheme = theme.appearance;
  }, [settings, systemDark, accent]);

  return children;
}
