export interface Command {
  id: string;
  title: string;
  subtitle?: string;
  icon?: string;
  keywords?: string[];
  kind: string;
  primaryActionTitle: string;
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
