export type ScriptTemplate = "batch" | "powershell" | "python" | "javascript";
export type ScriptMode = "compact" | "silent" | "fullOutput" | "inline";

export interface ScriptArgument {
  type?: string;
  placeholder?: string;
  optional?: boolean;
}

export interface Script {
  path: string;
  title: string;
  mode: ScriptMode;
  packageName: string;
  description: string;
  icon: string;
  needsConfirmation: boolean;
  arguments: ScriptArgument[];
}

export interface ScriptOutput {
  stdout: string;
  stderr: string;
  code: number | null;
}

export const TEMPLATES: { value: ScriptTemplate; label: string; extension: string; comment: string }[] = [
  { value: "powershell", label: "PowerShell", extension: "ps1", comment: "#" },
  { value: "batch", label: "Batch", extension: "bat", comment: "REM" },
  { value: "python", label: "Python", extension: "py", comment: "#" },
  { value: "javascript", label: "JavaScript (Node.js)", extension: "js", comment: "//" },
];

export const MODES: { value: ScriptMode; label: string }[] = [
  { value: "compact", label: "Compact" },
  { value: "silent", label: "Silent" },
  { value: "fullOutput", label: "Full Output" },
  { value: "inline", label: "Inline" },
];

export interface TemplateOptions {
  template: ScriptTemplate;
  title: string;
  mode: ScriptMode;
  description: string;
  packageName: string;
  needsConfirmation: boolean;
  withArgument: boolean;
}

export function scriptFileName(title: string, template: ScriptTemplate): string {
  const base = (title.trim() || "my-script-command").toLowerCase().replace(/[<>:"/\\|?*]/g, "").replace(/\s+/g, "-");
  return `${base || "my-script-command"}.${TEMPLATES.find((t) => t.value === template)!.extension}`;
}

export function scriptTemplate(options: TemplateOptions): string {
  const { template, mode, withArgument } = options;
  const c = TEMPLATES.find((t) => t.value === template)!.comment;
  const title = options.title.trim() || "My Script Command";
  const hello = withArgument ? "Hello World! Argument1 value:" : "Hello World!";
  const [header, body] = {
    batch: ["@echo off", withArgument ? `echo ${hello} %1` : `echo ${hello}`],
    powershell: ["", withArgument ? `param([string]$arg1)\nWrite-Output "${hello} $arg1"` : `Write-Output "${hello}"`],
    python: ["", withArgument ? `import sys\nprint("${hello} " + sys.argv[1])` : `print("${hello}")`],
    javascript: ["", withArgument ? `console.log("${hello} " + process.argv[2])` : `console.log("${hello}")`],
  }[template];
  const lines = [
    ...(header ? [header, ""] : []),
    `${c} Required parameters:`,
    `${c} @raycast.schemaVersion 1`,
    `${c} @raycast.title ${title}`,
    `${c} @raycast.mode ${mode}`,
    "",
    `${c} Optional parameters:`,
    `${c} @raycast.icon 🤖`,
    ...(options.packageName.trim() ? [`${c} @raycast.packageName ${options.packageName.trim()}`] : []),
    ...(options.description.trim() ? [`${c} @raycast.description ${options.description.trim()}`] : []),
    ...(options.needsConfirmation && mode !== "inline" ? [`${c} @raycast.needsConfirmation true`] : []),
    ...(withArgument && mode !== "inline" ? [`${c} @raycast.argument1 { "type": "text", "placeholder": "Placeholder" }`] : []),
    "",
    body,
    "",
  ];
  return lines.join(template === "batch" ? "\r\n" : "\n");
}

export function lastLine(text: string): string {
  return text.trimEnd().split(/\r?\n/).pop()?.trim() ?? "";
}

export function isEmojiIcon(icon: string): boolean {
  return !!icon && !/[\\/.]/.test(icon) && [...icon].length <= 3;
}
