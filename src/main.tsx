import { getCurrentWindow } from "@tauri-apps/api/window";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { Hud } from "./Hud";
import { SettingsWindow } from "./SettingsWindow";
import { ThemeRoot } from "./theme";
import "./styles.css";

function windowLabel() {
  try {
    return getCurrentWindow().label;
  } catch {
    return "launcher";
  }
}

const views: Record<string, () => React.ReactElement> = { hud: () => <Hud />, settings: () => <SettingsWindow /> };
const View = views[windowLabel()] ?? (() => <App />);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeRoot>
      <View />
    </ThemeRoot>
  </StrictMode>,
);
