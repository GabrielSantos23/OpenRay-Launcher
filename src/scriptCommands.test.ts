import { describe, expect, it } from "vitest";
import { isEmojiIcon, lastLine, scriptFileName, scriptTemplate } from "./scriptCommands";

describe("script commands", () => {
  it("writes the metadata header for a template", () => {
    const text = scriptTemplate({ template: "powershell", title: "Say Hi", mode: "compact", description: "Greets", packageName: "Dev", needsConfirmation: true, withArgument: true });
    expect(text).toContain("# @raycast.schemaVersion 1\n# @raycast.title Say Hi\n# @raycast.mode compact");
    expect(text).toContain("# @raycast.packageName Dev");
    expect(text).toContain("# @raycast.needsConfirmation true");
    expect(text).toContain('# @raycast.argument1 { "type": "text", "placeholder": "Placeholder" }');
    expect(text).toContain('param([string]$arg1)\nWrite-Output "Hello World! Argument1 value: $arg1"');
  });

  it("uses REM comments and CRLF for batch files", () => {
    const text = scriptTemplate({ template: "batch", title: "", mode: "silent", description: "", packageName: "", needsConfirmation: false, withArgument: false });
    expect(text.startsWith("@echo off\r\n\r\nREM Required parameters:")).toBe(true);
    expect(text).toContain("REM @raycast.title My Script Command");
    expect(text).not.toContain("packageName");
  });

  it("drops confirmation and arguments in inline mode", () => {
    const text = scriptTemplate({ template: "python", title: "T", mode: "inline", description: "", packageName: "", needsConfirmation: true, withArgument: true });
    expect(text).not.toContain("needsConfirmation");
    expect(text).not.toContain("argument1");
  });

  it("derives file names and reads output", () => {
    expect(scriptFileName("Open My: Repo", "javascript")).toBe("open-my-repo.js");
    expect(scriptFileName("  ", "batch")).toBe("my-script-command.bat");
    expect(lastLine("one\r\ntwo\r\n\r\n")).toBe("two");
    expect(isEmojiIcon("🤖")).toBe(true);
    expect(isEmojiIcon("icon.png")).toBe(false);
  });
});
