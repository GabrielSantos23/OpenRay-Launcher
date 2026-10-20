import { describe, expect, it } from "vitest";
import { joinCommand, kindOf, runCommandSections, runRows, splitCommand, type RunCommand, type RunEntry } from "./run";

const catalog: RunEntry[] = [
  { kind: "executable", command: "notepad.exe", description: "C:\\Windows\\System32\\notepad.exe" },
  { kind: "executable", command: "node.exe", description: "C:\\node\\node.exe" },
  { kind: "managementConsole", command: "devmgmt.msc", description: "C:\\Windows\\System32\\devmgmt.msc" },
  { kind: "environmentVariable", command: "%TEMP%", description: "C:\\Temp" },
];
const command: RunCommand = { id: "1", alias: "notes", command: "notepad.exe", arguments: "C:\\notes.txt", description: "", tags: ["work"] };

describe("run", () => {
  it("classifies commands", () => {
    expect(kindOf("%TEMP%")).toBe("environmentVariable");
    expect(kindOf("ms-settings:display")).toBe("uri");
    expect(kindOf("C:\\Windows")).toBe("path");
    expect(kindOf("ncpa.cpl")).toBe("controlPanelApplet");
    expect(kindOf("devmgmt.msc")).toBe("managementConsole");
    expect(kindOf("cmd")).toBe("executable");
  });

  it("splits and joins arguments", () => {
    expect(splitCommand('"C:\\Program Files\\x.exe" -a')).toEqual({ command: "C:\\Program Files\\x.exe", arguments: "-a" });
    expect(splitCommand("ping  1.1.1.1 -t")).toEqual({ command: "ping", arguments: "1.1.1.1 -t" });
    expect(joinCommand("C:\\Program Files\\x.exe", "-a")).toBe('"C:\\Program Files\\x.exe" -a');
  });

  it("builds Run, History and Results sections", () => {
    const sections = runRows("no", ["notepad", "cmd"], catalog, [command], []);
    expect(sections.map((s) => [s.title, s.rows.map((r) => r.command)])).toEqual([
      ["Run", ["no"]],
      ["History", ["notepad"]],
      ["Results", ["notes", "node.exe", "notepad.exe"]],
    ]);
    expect(runRows("", ["cmd"], catalog, [command], []).map((s) => s.title)).toEqual(["History"]);
    expect(runRows("notepad", ["notepad"], catalog, [], []).map((s) => s.title)).toEqual(["History", "Results"]);
    expect(runRows("no", [], catalog, [command], [], ["managementConsole"])).toEqual([]);
  });

  it("uses path completions for rooted input", () => {
    const completions: RunEntry[] = [{ kind: "path", command: "C:\\Windows\\", description: "C:\\Windows" }];
    expect(runRows("C:\\Win", [], catalog, [], completions).at(-1)!.rows.map((r) => r.command)).toEqual(["C:\\Windows\\"]);
  });

  it("groups run commands by pin and last use", () => {
    const now = new Date(2026, 9, 21, 12);
    const list: RunCommand[] = [
      { ...command, id: "a", pinned: true },
      { ...command, id: "b" },
      { ...command, id: "c", lastExecutedAt: new Date(2026, 9, 21, 9).getTime() },
      { ...command, id: "d", lastExecutedAt: new Date(2026, 9, 20, 9).getTime() },
      { ...command, id: "e", lastExecutedAt: new Date(2025, 1, 1).getTime() },
    ];
    expect(runCommandSections(list, now).map((s) => [s.title, s.items.map((c) => c.id)])).toEqual([
      ["Pinned", ["a"]],
      ["Recent", ["b"]],
      ["Today", ["c"]],
      ["Yesterday", ["d"]],
      ["Older", ["e"]],
    ]);
  });
});
