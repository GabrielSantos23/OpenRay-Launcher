import { invoke } from "@tauri-apps/api/core";
import { Cable, Copy, Headphones, Headset, Mic, MicOff, Monitor, RefreshCw, Settings, Speaker, Volume2, VolumeX, type LucideIcon } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { fail } from "../ActionHost";
import type { Action, IconSpec, Item, ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { ListView } from "../ListView";
import { showToast } from "../toast";

type Direction = "output" | "input";

interface SoundDevice {
  id: string;
  name: string;
  state: "active" | "disabled" | "disconnected";
  formFactor: number;
  isDefault: boolean;
  volume: number;
  isMuted: boolean;
}

const SETUP = {
  output: { title: "Set Sound Output Device", label: "Output Device", icon: { glyph: Speaker, tone: "red" } as IconSpec, muted: VolumeX, unmuted: Volume2 },
  input: { title: "Set Sound Input Device", label: "Input Device", icon: { glyph: Mic, tone: "red" } as IconSpec, muted: MicOff, unmuted: Mic },
};

const SECTIONS: [SoundDevice["state"], string][] = [
  ["active", "Available"],
  ["disabled", "Disabled"],
  ["disconnected", "Disconnected"],
];

const LEVELS = [0, 25, 50, 75, 100];

export function deviceIcon(direction: Direction, formFactor: number): LucideIcon {
  const byFormFactor: Record<number, LucideIcon> = { 1: Speaker, 2: Cable, 3: Headphones, 4: Mic, 5: Headset, 6: Headset, 9: Monitor };
  return byFormFactor[formFactor] ?? (direction === "output" ? Speaker : Mic);
}

export function sortDevices(devices: SoundDevice[]): SoundDevice[] {
  return [...devices].sort((a, b) => Number(b.state === "active" && b.isDefault) - Number(a.state === "active" && a.isDefault) || a.name.localeCompare(b.name));
}

function VolumeLevel({ device, direction }: { device: SoundDevice; direction: Direction }) {
  const Icon = device.isMuted ? SETUP[direction].muted : SETUP[direction].unmuted;
  return (
    <span className="flex items-center gap-2 text-12 text-fg-60">
      <span className="w-8 text-right tabular-nums">{Math.round(device.volume)}%</span>
      <span className="h-1 w-12 overflow-hidden rounded-full bg-control">
        <span className="block h-full rounded-full bg-fg-60" style={{ width: `${Math.round(device.volume)}%` }} />
      </span>
      <Icon aria-label={device.isMuted ? "Muted" : undefined} className="size-3.5" />
    </span>
  );
}

function SoundDevices({ pop, direction }: ViewProps & { direction: Direction }) {
  const [devices, setDevices] = useState<SoundDevice[] | null>(null);
  const setup = SETUP[direction];

  const reload = useCallback(() => {
    invoke<SoundDevice[]>("sound_devices", { direction })
      .then(setDevices)
      .catch((e) => fail("Could not list sound devices", e));
  }, [direction]);

  useEffect(reload, [reload]);

  const sections = useMemo(() => {
    const change = (device: SoundDevice, values: { default?: boolean; volume?: number; muted?: boolean }, failTitle: string, success?: string) => async () => {
      try {
        await invoke("sound_device_set", { id: device.id, ...values });
        if (success) showToast({ variant: "success", title: success });
      } catch (e) {
        showToast({ variant: "failure", title: failTitle, message: e instanceof Error ? e.message : String(e) });
      } finally {
        reload();
      }
    };
    const actions = (device: SoundDevice): Action[][] => {
      const groups: Action[][] = [];
      if (device.state === "active") {
        if (!device.isDefault) {
          groups.push([
            {
              id: "select",
              title: `Set as ${setup.label}`,
              icon: deviceIcon(direction, device.formFactor),
              keepOpen: true,
              run: change(device, { default: true }, `Unable to set ${setup.label} to ${device.name}`, `${setup.label} set to ${device.name}`),
            },
          ]);
        }
        const mute = !device.isMuted;
        groups.push(
          [{ id: "toggle-mute", title: device.isMuted ? "Unmute" : "Mute", icon: device.isMuted ? setup.unmuted : setup.muted, keepOpen: true, run: change(device, { muted: mute }, `Unable to ${mute ? "mute" : "unmute"} ${device.name}`) }],
          LEVELS.map((level) => ({
            id: `set-volume-${level}`,
            title: `Set Volume to ${level}%`,
            icon: level ? setup.unmuted : setup.muted,
            accessory: !device.isMuted && Math.round(device.volume) === level ? "Current" : undefined,
            keepOpen: true,
            run: change(device, { volume: level }, `Unable to set ${device.name} volume`),
          })),
        );
      }
      groups.push([
        {
          id: "copy-name",
          title: "Copy Device Name",
          icon: Copy,
          shortcut: { key: "c", ctrl: true, shift: true },
          keepOpen: true,
          run: async () => {
            await invoke("copy_text", { text: device.name });
            showToast({ variant: "success", title: "Copied to Clipboard" });
          },
        },
      ]);
      groups.push([
        { id: "refresh", title: "Refresh Devices", icon: RefreshCw, shortcut: { key: "r", ctrl: true }, keepOpen: true, run: reload },
        { id: "open-settings", title: "Open Sound Settings", icon: Settings, shortcut: { key: "o", ctrl: true, shift: true }, run: () => invoke("open_url", { url: "ms-settings:sound", appPath: null }) },
      ]);
      return groups;
    };
    const item = (device: SoundDevice): Item => {
      const Icon = deviceIcon(direction, device.formFactor);
      return {
        id: device.id,
        title: device.name,
        icon: <Icon aria-hidden="true" className="size-4 shrink-0 text-fg-60" />,
        accessory:
          device.state === "active" ? (
            <span className="flex items-center gap-2">
              {device.isDefault && <span className="rounded-[4px] bg-icon-blue-top/15 px-1.5 py-px text-11 font-medium text-icon-blue-top">Default</span>}
              <VolumeLevel device={device} direction={direction} />
            </span>
          ) : undefined,
        actions: actions(device),
      };
    };
    const sorted = sortDevices(devices ?? []);
    return SECTIONS.map(([state, title]) => ({ title, items: sorted.filter((d) => d.state === state).map(item) })).filter((s) => s.items.length > 0);
  }, [devices, direction, reload, setup]);

  return (
    <ListView
      placeholder="Search devices…"
      sections={sections}
      onBack={pop}
      footerTitle={setup.title}
      footerIcon={<GlyphTile spec={setup.icon} small />}
      isLoading={devices === null}
      emptyText={`No ${setup.label.toLowerCase()}s found`}
    />
  );
}

export const OutputDevices = (props: ViewProps) => <SoundDevices {...props} direction="output" />;
export const InputDevices = (props: ViewProps) => <SoundDevices {...props} direction="input" />;
