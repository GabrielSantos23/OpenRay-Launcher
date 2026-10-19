import { invoke } from "@tauri-apps/api/core";
import {
  AppWindow,
  ArrowLeftFromLine,
  ArrowRightFromLine,
  Columns2,
  Columns3,
  Maximize2,
  PanelBottom,
  PanelLeft,
  PanelRight,
  PanelTop,
  Rows3,
  type LucideIcon,
} from "lucide-react";
import { defineExtension, type Command } from "../extension-api";

const LAYOUTS: [id: string, title: string, glyph: LucideIcon, keywords: string[]][] = [
  ["maximize", "Maximize", Maximize2, []],
  ["center", "Center", AppWindow, []],
  ["left-half", "Left Half", PanelLeft, []],
  ["right-half", "Right Half", PanelRight, []],
  ["top-half", "Top Half", PanelTop, []],
  ["bottom-half", "Bottom Half", PanelBottom, []],
  ["first-third", "First Third", Columns3, ["left"]],
  ["center-third", "Center Third", Columns3, []],
  ["last-third", "Last Third", Columns3, ["right"]],
  ["first-two-thirds", "First Two Thirds", Columns2, ["left"]],
  ["center-two-thirds", "Center Two Thirds", Columns2, []],
  ["last-two-thirds", "Last Two Thirds", Columns2, ["right"]],
  ["top-third", "Top Third", Rows3, []],
  ["middle-third", "Middle Third", Rows3, []],
  ["bottom-third", "Bottom Third", Rows3, []],
  ["top-two-thirds", "Top Two Thirds", Rows3, []],
  ["bottom-two-thirds", "Bottom Two Thirds", Rows3, []],
  ["previous-display", "Move to Previous Display", ArrowLeftFromLine, ["desktop", "monitor", "screen"]],
  ["next-display", "Move to Next Display", ArrowRightFromLine, ["desktop", "monitor", "screen"]],
];

export default defineExtension({
  id: "window-management",
  title: "Window Management",
  icon: { glyph: AppWindow, tone: "blue" },
  commands: LAYOUTS.map(
    ([id, title, glyph, keywords]): Command => ({
      id,
      title,
      icon: { glyph, tone: "blue" },
      subtitle: "Window Management",
      keywords: ["window", "resize", "move", ...keywords],
      kind: "Command",
      primaryActionTitle: "Run Command",
      run: () => invoke("window_layout", { layout: id }),
    }),
  ),
});
