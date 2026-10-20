import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { AppWindow, Maximize2, Minimize2, MinusCircle, Power, XCircle } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { fail } from "../ActionHost";
import { defineExtension, type Action, type IconSpec, type Item, type ViewProps } from "../extension-api";
import { ListView } from "../ListView";
import { GlyphTile } from "../icons";

const ICON: IconSpec = { glyph: AppWindow, tone: "blue" };

export interface OpenWindow {
  id: number;
  title: string;
  appName: string | null;
  path: string | null;
  minimized: boolean;
  maximized: boolean;
}

function WindowIcon({ path }: { path: string | null }) {
  const [failed, setFailed] = useState(false);
  if (!path || failed) return <AppWindow aria-hidden="true" className="size-[22px] shrink-0 text-fg-60" />;
  return <img src={`${convertFileSrc(path, "appicon")}?size=${Math.round(22 * window.devicePixelRatio)}`} alt="" draggable={false} onError={() => setFailed(true)} className="size-[22px] shrink-0" />;
}

function windowActions(target: OpenWindow, reload: () => void): Action[][] {
  const act = (action: string) => invoke<void>("window_action", { id: target.id, action });
  const quit = (action: string) => async () => {
    await act(action);
    reload();
  };
  const actions: Action[][] = [
    [
      { id: "switch", title: "Switch to Window", icon: AppWindow, run: () => act("switch") },
      { id: "minimize", title: target.minimized ? "Unminimize Window" : "Minimize Window", icon: MinusCircle, shortcut: { key: "m", ctrl: true }, run: () => act(target.minimized ? "switch" : "minimize") },
      { id: "close", title: "Close Window", icon: XCircle, destructive: true, shortcut: { key: "w", ctrl: true, shift: true }, run: () => act("close") },
      { id: "toggle-maximize", title: target.maximized ? "Restore" : "Maximize", icon: target.maximized ? Minimize2 : Maximize2, shortcut: { key: "f", ctrl: true }, run: () => act("toggle-maximize") },
    ],
  ];
  if (target.appName) {
    actions.push([
      { id: "quit", title: "Quit Application", icon: Power, destructive: true, keepOpen: true, shortcut: { key: "q", ctrl: true }, run: quit("quit") },
      { id: "force-quit", title: "Force Quit Application", icon: Power, destructive: true, keepOpen: true, shortcut: { key: "q", ctrl: true, shift: true }, run: quit("force-quit") },
    ]);
  }
  return actions;
}

function SwitchWindows({ pop }: ViewProps) {
  const [windows, setWindows] = useState<OpenWindow[] | null>(null);
  const [version, setVersion] = useState(0);

  useEffect(() => {
    invoke<OpenWindow[]>("window_list")
      .then(setWindows)
      .catch((e) => fail("Could not list windows", e));
  }, [version]);

  const reload = () => setVersion((v) => v + 1);
  const sections = useMemo(
    () => [
      {
        title: "",
        items: (windows ?? []).map(
          (w): Item => ({
            id: String(w.id),
            title: w.title,
            subtitle: w.appName ?? undefined,
            keywords: w.appName ? [w.appName] : [],
            icon: <WindowIcon path={w.path} />,
            actions: windowActions(w, reload),
          }),
        ),
      },
    ],
    [windows],
  );

  return (
    <ListView
      placeholder="Search windows…"
      sections={sections}
      onBack={pop}
      footerTitle="Switch Windows"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={windows === null}
      selectedId={windows?.[1] ? String(windows[1].id) : undefined}
      tabCycles
    />
  );
}

export default defineExtension({
  id: "navigation",
  title: "Navigation",
  icon: ICON,
  commands: [
    {
      id: "switch-windows",
      title: "Switch Windows",
      subtitle: "Navigation",
      keywords: ["windows switch", "window switcher"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: SwitchWindows,
    },
  ],
});
