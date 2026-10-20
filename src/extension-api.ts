import type { ComponentType, ReactElement, ReactNode } from "react";

export interface Shortcut {
  key: string;
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
}

export type IconTone = "blue" | "orange" | "red" | "yellow" | "purple" | "green" | "pink" | "slate";

export interface IconSpec {
  glyph: ComponentType<{ className?: string; strokeWidth?: number }>;
  tone: IconTone;
}

export interface Confirmation {
  title: string;
  message: string;
  confirmTitle: string;
}

export interface Action {
  id: string;
  title: string;
  icon?: ComponentType<{ className?: string }>;
  shortcut?: Shortcut;
  destructive?: boolean;
  disabled?: boolean;
  keepOpen?: boolean;
  accessory?: string;
  confirm?: Confirmation | (() => Promise<Confirmation>);
  submenu?: () => Action[] | Promise<Action[]>;
  run: () => Promise<void> | void;
}

export interface ViewProps {
  pop: () => void;
}

export interface Command {
  id: string;
  title: string;
  subtitle?: string;
  icon?: string | IconSpec | ReactElement;
  keywords?: string[];
  kind: string;
  primaryActionTitle: string;
  confirm?: Confirmation;
  actions?: Action[][];
  view?: ComponentType<ViewProps>;
  fallback?: (text: string) => Promise<void> | void;
  fallbackView?: (text: string) => ComponentType<ViewProps>;
  argument?: { placeholder: string; optional?: boolean; run: (value: string) => Promise<void> | void };
  query?: { matches: (text: string) => boolean; run: (text: string) => Promise<void> | void };
  run?: () => Promise<void> | void;
}

export interface InlineSide {
  text: string;
  label: string;
  swatch?: string;
}

export interface InlineResult {
  section: string;
  input: InlineSide;
  output: InlineSide;
  note?: string;
  primaryActionTitle: string;
  actions: Action[][];
}

export interface Extension {
  id: string;
  title: string;
  icon?: IconSpec;
  commands?: Command[];
  items?: () => Promise<Command[]>;
  inline?: (query: string) => Promise<InlineResult | null>;
}

export interface RegisteredCommand extends Command {
  key: string;
  extensionId: string;
}

export interface Item {
  id: string;
  title: string;
  icon?: ReactNode;
  subtitle?: string;
  accessory?: ReactNode;
  keywords?: string[];
  actions: Action[][];
}

export function defineExtension(extension: Extension): Extension {
  return extension;
}
