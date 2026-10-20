export type RunKind = "command" | "controlPanelApplet" | "environmentVariable" | "executable" | "managementConsole" | "path" | "uri";

export interface RunEntry {
  kind: RunKind;
  command: string;
  description: string;
}

export interface RunCommand {
  id: string;
  alias: string;
  command: string;
  arguments: string;
  description: string;
  tags: string[];
  pinned?: boolean;
  lastExecutedAt?: number;
}

export interface RunRow {
  id: string;
  source: "input" | "history" | "result";
  kind: RunKind;
  command: string;
  description: string;
  record?: RunCommand;
}

export const KIND_LABELS: Record<RunKind, string> = {
  command: "Run Command",
  controlPanelApplet: "Control Panel Applet",
  environmentVariable: "Environment Variable",
  executable: "Executable",
  managementConsole: "Management Console",
  path: "Path",
  uri: "URI",
};

const ROOTED = /^([a-z]:[\\/]|\\\\)/i;

export function kindOf(command: string): RunKind {
  if (/^%[^%\s]+%$/.test(command)) return "environmentVariable";
  if (/^[a-z][a-z0-9+.-]+:(?![\\/])/i.test(command) && !ROOTED.test(command)) return "uri";
  if (/\.cpl$/i.test(command)) return "controlPanelApplet";
  if (/\.msc$/i.test(command)) return "managementConsole";
  if (/\.(exe|com|bat|cmd)$/i.test(command)) return "executable";
  if (ROOTED.test(command) || /^%[^%]+%[\\/]/.test(command)) return "path";
  return "executable";
}

export function isPathLike(text: string): boolean {
  return ROOTED.test(text.trim().replace(/^"/, "")) || /^%[^%]+%[\\/]/.test(text.trim());
}

export function splitCommand(text: string): { command: string; arguments: string } {
  const value = text.trim();
  if (value.startsWith('"')) {
    const end = value.indexOf('"', 1);
    return end < 0 ? { command: value.slice(1), arguments: "" } : { command: value.slice(1, end), arguments: value.slice(end + 1).trim() };
  }
  const space = value.search(/\s/);
  return space < 0 ? { command: value, arguments: "" } : { command: value.slice(0, space), arguments: value.slice(space).trim() };
}

export function joinCommand(command: string, args: string): string {
  const quoted = command.includes(" ") && !command.startsWith('"') ? `"${command}"` : command;
  return args.trim() ? `${quoted} ${args.trim()}` : quoted;
}

export function runRows(input: string, history: string[], catalog: RunEntry[], commands: RunCommand[], completions: RunEntry[], kinds?: RunKind[]): { title: string; rows: RunRow[] }[] {
  const text = input.trim();
  const lower = text.toLowerCase();
  const allowed = (kind: RunKind) => !kinds || kinds.includes(kind);
  const historyRows = history
    .filter((h) => h.toLowerCase().startsWith(lower))
    .sort((a, b) => (text ? a.length - b.length : 0))
    .map((h): RunRow => ({ id: `history:${h}`, source: "history", kind: kindOf(splitCommand(h).command), command: h, description: "" }))
    .filter((row) => allowed(row.kind));
  const results: RunRow[] = [];
  if (text) {
    for (const record of commands) {
      if ((record.alias.toLowerCase().startsWith(lower) || record.tags.some((t) => t.toLowerCase().startsWith(lower))) && allowed("command")) {
        results.push({ id: `command:${record.id}`, source: "result", kind: "command", command: record.alias, description: record.description || joinCommand(record.command, record.arguments), record });
      }
    }
    const pool = isPathLike(text) ? completions : catalog.filter((e) => e.command.toLowerCase().startsWith(lower));
    for (const entry of pool) {
      if (results.length >= 60) break;
      if (allowed(entry.kind)) results.push({ id: `${entry.kind}:${entry.command}`, source: "result", kind: entry.kind, command: entry.command, description: entry.description });
    }
    results.sort((a, b) => Number(b.kind === "command") - Number(a.kind === "command") || a.command.length - b.command.length);
  }
  const duplicate = historyRows.some((r) => r.command.toLowerCase() === lower) || results.some((r) => r.kind !== "command" && r.command.toLowerCase() === lower && historyRows.length === 0);
  const inputKind = kindOf(splitCommand(text).command);
  const inputRow: RunRow[] = text && !duplicate && allowed(inputKind) ? [{ id: "input", source: "input", kind: inputKind, command: text, description: "" }] : [];
  return [
    { title: "Run", rows: inputRow },
    { title: "History", rows: historyRows },
    { title: "Results", rows: results.filter((r) => r.command.toLowerCase() !== lower || r.kind === "command") },
  ].filter((s) => s.rows.length > 0);
}

export function runCommandSections(commands: RunCommand[], now = new Date()): { title: string; items: RunCommand[] }[] {
  const startOfDay = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const day = 86_400_000;
  const weekStart = startOfDay - ((now.getDay() + 6) % 7) * day;
  const buckets: [string, (t: number) => boolean][] = [
    ["Today", (t) => t >= startOfDay],
    ["Yesterday", (t) => t >= startOfDay - day],
    ["This Week", (t) => t >= weekStart],
    ["This Month", (t) => t >= new Date(now.getFullYear(), now.getMonth(), 1).getTime()],
    ["This Year", (t) => t >= new Date(now.getFullYear(), 0, 1).getTime()],
    ["Older", () => true],
  ];
  const sections = [{ title: "Pinned", items: commands.filter((c) => c.pinned) }, { title: "Recent", items: commands.filter((c) => !c.pinned && !c.lastExecutedAt) }];
  const used = commands.filter((c) => !c.pinned && c.lastExecutedAt).sort((a, b) => b.lastExecutedAt! - a.lastExecutedAt!);
  for (const [title, test] of buckets) {
    const taken = new Set(sections.flatMap((s) => s.items));
    sections.push({ title, items: used.filter((c) => !taken.has(c) && test(c.lastExecutedAt!)) });
  }
  return sections.filter((s) => s.items.length > 0);
}
