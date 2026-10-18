import { getCurrentWindow } from "@tauri-apps/api/window";
import { lazy, StrictMode, Suspense } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { Hud } from "./Hud";
import { SettingsWindow } from "./SettingsWindow";
import { ThemeRoot } from "./theme";
import { Tooltip } from "./Tooltip";
import "./styles.css";

function windowLabel() {
  try {
    return getCurrentWindow().label;
  } catch {
    return "launcher";
  }
}

const NotesWindow = lazy(() => import("./NotesWindow").then((m) => ({ default: m.NotesWindow })));

const views: Record<string, () => React.ReactElement> = {
  hud: () => <Hud />,
  settings: () => <SettingsWindow />,
  notes: () => (
    <Suspense>
      <NotesWindow />
    </Suspense>
  ),
  tooltip: () => <Tooltip />,
};
const View = views[windowLabel()] ?? (() => <App />);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeRoot>
      <View />
    </ThemeRoot>
  </StrictMode>,
);
