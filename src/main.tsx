import { getCurrentWindow } from "@tauri-apps/api/window";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { Hud } from "./Hud";
import "./styles.css";

function windowLabel() {
  try {
    return getCurrentWindow().label;
  } catch {
    return "launcher";
  }
}

createRoot(document.getElementById("root")!).render(<StrictMode>{windowLabel() === "hud" ? <Hud /> : <App />}</StrictMode>);
