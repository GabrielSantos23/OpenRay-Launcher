import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { Speaker, Volume2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { fail } from "../ActionHost";
import type { IconSpec, Item, ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { ListView } from "../ListView";
import { showToast } from "../toast";
import { VolumeLevel, volumeActions } from "./sound-devices";

const ICON: IconSpec = { glyph: Volume2, tone: "red" };

interface AppAudio {
  id: string;
  name: string;
  path: string | null;
  volume: number;
  isMuted: boolean;
  isPlaying: boolean;
}

export function sortApps(apps: AppAudio[]): AppAudio[] {
  return [...apps].sort((a, b) => Number(b.isPlaying) - Number(a.isPlaying) || Number(a.id === "system") - Number(b.id === "system") || a.name.localeCompare(b.name));
}

function AppIcon({ app }: { app: AppAudio }) {
  const [failed, setFailed] = useState(false);
  if (!app.path || failed) return <Speaker aria-hidden="true" className="size-4 shrink-0 text-fg-60" />;
  return <img src={`${convertFileSrc(app.path, "appicon")}?size=${Math.round(22 * window.devicePixelRatio)}`} alt="" draggable={false} onError={() => setFailed(true)} className="size-[22px] shrink-0" />;
}

export function VolumeMixer({ pop }: ViewProps) {
  const [apps, setApps] = useState<AppAudio[] | null>(null);

  const reload = useCallback(() => {
    invoke<AppAudio[]>("app_volumes")
      .then(setApps)
      .catch((e) => fail("Could not list app volumes", e));
  }, []);

  useEffect(reload, [reload]);

  const sections = useMemo(() => {
    const item = (app: AppAudio): Item => ({
      id: app.id,
      title: app.name,
      icon: <AppIcon app={app} />,
      accessory: <VolumeLevel device={app} direction="output" />,
      actions: volumeActions("output", app, (values, failTitle) => async () => {
        try {
          await invoke("app_volume_set", { id: app.id, ...values });
        } catch (e) {
          showToast({ variant: "failure", title: failTitle, message: e instanceof Error ? e.message : String(e) });
        } finally {
          reload();
        }
      }),
    });
    const sorted = sortApps(apps ?? []);
    return [
      { title: "Playing", items: sorted.filter((a) => a.isPlaying).map(item) },
      { title: "Idle", items: sorted.filter((a) => !a.isPlaying).map(item) },
    ].filter((s) => s.items.length > 0);
  }, [apps, reload]);

  return (
    <ListView
      placeholder="Search apps…"
      sections={sections}
      onBack={pop}
      footerTitle="Volume Mixer"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={apps === null}
      emptyText="No apps are playing audio"
    />
  );
}
