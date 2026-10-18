import type { ComponentType } from "react";

export interface Shortcut {
  key: string;
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
}

export interface Action {
  id: string;
  title: string;
  icon?: ComponentType<{ className?: string }>;
  shortcut?: Shortcut;
  destructive?: boolean;
  keepOpen?: boolean;
  run: () => Promise<void> | void;
}

export interface Command {
  id: string;
  title: string;
  subtitle?: string;
  icon?: string;
  keywords?: string[];
  kind: string;
  primaryActionTitle: string;
  actions?: Action[][];
  run: () => Promise<void> | void;
}

export interface Extension {
  id: string;
  title: string;
  commands?: Command[];
  items?: () => Promise<Command[]>;
}

export interface RegisteredCommand extends Command {
  key: string;
  extensionId: string;
}

export function defineExtension(extension: Extension): Extension {
  return extension;
}
