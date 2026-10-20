import { invoke } from "@tauri-apps/api/core";
import {
  Eye,
  Lock,
  Mic,
  LogOut,
  MicOff,
  Monitor,
  MonitorOff,
  Moon,
  MoonStar,
  Play,
  Power,
  RotateCcw,
  SkipBack,
  SkipForward,
  Speaker,
  Sparkles,
  SunMoon,
  Trash,
  Trash2,
  Volume1,
  Volume2,
  VolumeX,
  X,
  type LucideIcon,
} from "lucide-react";
import { defineExtension, type Command, type Confirmation } from "../extension-api";
import { showHUD, showToast } from "../toast";
import { VolumeMixer } from "./app-volume";
import { InputDevices, OutputDevices } from "./sound-devices";

interface Entry {
  id: string;
  title: string;
  glyph: LucideIcon;
  keywords?: string[];
  confirm?: Confirmation;
}

const ENTRIES: Entry[] = [
  { id: "lock-screen", title: "Lock Screen", glyph: Lock },
  { id: "log-out", title: "Log Out", glyph: LogOut, confirm: { title: "Log Out?", message: "Open apps will close. Unsaved work may be lost.", confirmTitle: "Log Out" } },
  { id: "restart", title: "Restart", glyph: RotateCcw, keywords: ["reboot"], confirm: { title: "Restart Your Computer?", message: "Open apps will close. Unsaved work may be lost.", confirmTitle: "Restart" } },
  { id: "shut-down", title: "Shut Down", glyph: Power, confirm: { title: "Shut Down Your Computer?", message: "Open apps will close. Unsaved work may be lost.", confirmTitle: "Shut Down" } },
  { id: "sleep", title: "Sleep", glyph: Moon },
  { id: "hibernate", title: "Hibernate", glyph: MoonStar },
  { id: "sleep-displays", title: "Sleep Displays", glyph: MonitorOff },
  { id: "show-screen-saver", title: "Show Screen Saver", glyph: Sparkles },
  { id: "show-desktop", title: "Show Desktop", glyph: Monitor },
  { id: "open-recycle-bin", title: "Open Recycle Bin", glyph: Trash2, keywords: ["trash"] },
  {
    id: "empty-recycle-bin",
    title: "Empty Recycle Bin",
    glyph: Trash,
    keywords: ["trash"],
    confirm: { title: "Empty Recycle Bin?", message: "Everything in the Recycle Bin will be deleted permanently.", confirmTitle: "Empty Recycle Bin" },
  },
  { id: "quit-active-app", title: "Quit the Active App", glyph: X, keywords: ["close"] },
  {
    id: "quit-all-apps",
    title: "Quit All Apps",
    glyph: X,
    keywords: ["close"],
    confirm: { title: "Quit All Apps?", message: "Every open app window will be asked to close.", confirmTitle: "Quit All Apps" },
  },
  {
    id: "quit-all-apps-except-frontmost",
    title: "Quit All Apps Except Frontmost",
    glyph: X,
    keywords: ["close"],
    confirm: { title: "Quit All Other Apps?", message: "Every open app window except the active app's will be asked to close.", confirmTitle: "Quit Apps" },
  },
  { id: "turn-volume-up", title: "Turn Volume Up", glyph: Volume2, keywords: ["sound", "audio"] },
  { id: "turn-volume-down", title: "Turn Volume Down", glyph: Volume1, keywords: ["sound", "audio"] },
  { id: "toggle-mute", title: "Toggle Mute", glyph: VolumeX, keywords: ["sound", "audio", "mute"] },
  ...[0, 25, 50, 75, 100].map((percent) => ({ id: `set-volume-${percent}`, title: `Set Volume to ${percent}%`, glyph: percent ? Volume2 : VolumeX, keywords: ["sound", "audio"] })),
  { id: "toggle-microphone-mute", title: "Toggle Microphone Mute", glyph: MicOff, keywords: ["mute mic", "mute microphone", "unmute microphone", "mic mute"] },
  { id: "play-pause", title: "Play / Pause", glyph: Play, keywords: ["music", "media"] },
  { id: "next-track", title: "Next Track", glyph: SkipForward, keywords: ["music", "media"] },
  { id: "previous-track", title: "Previous Track", glyph: SkipBack, keywords: ["music", "media"] },
  { id: "toggle-dark-mode", title: "Toggle Dark Mode", glyph: SunMoon, keywords: ["theme", "dark mode", "light mode", "system appearance", "toggle system appearance"] },
  { id: "toggle-hidden-files", title: "Toggle Hidden Files", glyph: Eye },
];

export function parseVolume(text: string): number | null {
  const value = text.trim().toLowerCase().replace(/%$/, "").trim();
  return /^\d+(\.\d+)?$/.test(value) ? Math.min(100, Math.round(Number(value))) : null;
}

async function setVolume(text: string) {
  const volume = parseVolume(text);
  if (volume === null) {
    showToast({ variant: "failure", title: "Invalid volume", message: "Enter a number between 0 and 100." });
    return;
  }
  await invoke("hide_launcher", { restoreFocus: true });
  try {
    await invoke("system_action", { action: `set-volume-${volume}` });
    showHUD({ variant: "success", title: `Volume set to ${volume}%` });
  } catch {
    showHUD({ variant: "failure", title: `Unable to set volume to ${volume}%` });
  }
}

export default defineExtension({
  id: "system",
  title: "System Actions",
  icon: { glyph: Power, tone: "red" },
  commands: [
    ...ENTRIES.map(
      ({ id, title, glyph, keywords = [], confirm }): Command => ({
        id,
        title,
        icon: { glyph, tone: "red" },
        subtitle: "System",
        keywords: ["system", ...keywords],
        kind: "Command",
        primaryActionTitle: "Run Command",
        confirm,
        run: () => invoke("system_action", { action: id }),
      }),
    ),
    {
      id: "set-volume",
      title: "Set Volume",
      icon: { glyph: Volume2, tone: "red" },
      subtitle: "System",
      keywords: ["system", "sound", "audio"],
      kind: "Command",
      primaryActionTitle: "Run Command",
      argument: { placeholder: "Volume (0-100)", run: setVolume },
    },
    {
      id: "volume-mixer",
      title: "Volume Mixer",
      icon: { glyph: Volume2, tone: "red" },
      subtitle: "System",
      keywords: ["system", "app volume", "set app volume", "mute app", "audio mixer"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: VolumeMixer,
    },
    ...(
      [
        ["toggle-active-app-mute", "Toggle Active App Mute", VolumeX, "mute", ["mute current app", "mute foreground app", "mute active app"]],
        ["turn-active-app-volume-up", "Turn Active App Volume Up", Volume2, "up", ["current app volume up", "foreground app volume", "active app volume"]],
        ["turn-active-app-volume-down", "Turn Active App Volume Down", Volume1, "down", ["current app volume down", "foreground app volume", "active app volume"]],
      ] as const
    ).map(
      ([id, title, glyph, action, keywords]): Command => ({
        id,
        title,
        icon: { glyph, tone: "red" },
        subtitle: "System",
        keywords: ["system", ...keywords],
        kind: "Command",
        primaryActionTitle: "Run Command",
        run: async () => {
          await invoke("hide_launcher", { restoreFocus: true });
          try {
            showHUD({ variant: "success", title: await invoke<string>("app_volume_active", { action }) });
          } catch (e) {
            showHUD({ variant: "failure", title: e instanceof Error ? e.message : String(e) });
          }
        },
      }),
    ),
    {
      id: "set-sound-output-device",
      title: "Set Sound Output Device",
      icon: { glyph: Speaker, tone: "red" },
      subtitle: "System",
      keywords: ["system", "output device", "audio output", "playback device", "speakers", "headphones", "sound device"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: OutputDevices,
    },
    {
      id: "set-sound-input-device",
      title: "Set Sound Input Device",
      icon: { glyph: Mic, tone: "red" },
      subtitle: "System",
      keywords: ["system", "input device", "audio input", "recording device", "microphone", "mic", "sound device"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: InputDevices,
    },
  ],
});
