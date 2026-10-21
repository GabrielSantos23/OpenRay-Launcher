import { getCurrentWindow } from "@tauri-apps/api/window";
import { lazy, StrictMode, Suspense, type ComponentType } from "react";
import { createRoot } from "react-dom/client";
import { ThemeRoot } from "./theme";
import "./styles.css";

function windowLabel() {
  try {
    return getCurrentWindow().label;
  } catch {
    return "launcher";
  }
}

document.addEventListener("contextmenu", (event) => event.preventDefault());

const views: Record<string, () => Promise<{ default: ComponentType }>> = {
  hud: () => import("./Hud").then((m) => ({ default: m.Hud })),
  notch: () => import("./NotchWindow").then((m) => ({ default: m.NotchWindow })),
  settings: () => import("./SettingsWindow").then((m) => ({ default: m.SettingsWindow })),
  notes: () => import("./NotesWindow").then((m) => ({ default: m.NotesWindow })),
  tooltip: () => import("./Tooltip").then((m) => ({ default: m.Tooltip })),
  "focus-panel": () => import("./FocusWindows").then((m) => ({ default: m.FocusPanel })),
  "focus-toast": () => import("./FocusWindows").then((m) => ({ default: m.FocusToast })),
  "focus-block": () => import("./FocusWindows").then((m) => ({ default: m.FocusBlock })),
  capture: () => import("./CaptureOverlay").then((m) => ({ default: m.CaptureOverlay })),
};
const label = windowLabel();
const View = lazy(label.startsWith("pin-") ? () => import("./PinWindow").then((m) => ({ default: m.PinWindow })) : (views[label] ?? (() => import("./App"))));

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeRoot>
      <Suspense>
        <View />
      </Suspense>
    </ThemeRoot>
  </StrictMode>,
);
