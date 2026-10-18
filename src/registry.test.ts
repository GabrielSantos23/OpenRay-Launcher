import { describe, expect, it, vi } from "vitest";
import { defineExtension } from "./extension-api";
import { loadCommands } from "./registry";

const base = { kind: "Command", primaryActionTitle: "Run Command", run: () => {} };

describe("loadCommands", () => {
  it("merges static and dynamic commands with namespaced keys", async () => {
    const a = defineExtension({ id: "a", title: "A", commands: [{ ...base, id: "one", title: "One" }] });
    const b = defineExtension({ id: "b", title: "B", items: async () => [{ ...base, id: "two", title: "Two" }] });
    const commands = await loadCommands([a, b]);
    expect(commands.map((c) => c.key)).toEqual(["a/one", "b/two"]);
    expect(commands[1].extensionId).toBe("b");
  });

  it("keeps other extensions when one fails to load", async () => {
    const error = vi.spyOn(console, "error").mockImplementation(() => {});
    const broken = defineExtension({
      id: "broken",
      title: "Broken",
      commands: [{ ...base, id: "static", title: "Static" }],
      items: async () => {
        throw new Error("boom");
      },
    });
    const ok = defineExtension({ id: "ok", title: "OK", commands: [{ ...base, id: "x", title: "X" }] });
    const commands = await loadCommands([broken, ok]);
    expect(commands.map((c) => c.key)).toEqual(["broken/static", "ok/x"]);
    expect(error).toHaveBeenCalledOnce();
  });

  it("gives commands without their own icon the extension icon", async () => {
    const icon = { glyph: () => null, tone: "blue" as const };
    const ext = defineExtension({ id: "e", title: "E", icon, commands: [{ ...base, id: "a", title: "A" }, { ...base, id: "b", title: "B", icon: "b.png" }] });
    const [a, b] = await loadCommands([ext]);
    expect(a.icon).toBe(icon);
    expect(b.icon).toBe("b.png");
  });
});
