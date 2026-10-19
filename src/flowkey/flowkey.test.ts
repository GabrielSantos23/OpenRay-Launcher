import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ExtensionManifest } from "@flowkey-cli/native-sdk";

const disk = new Map<string, unknown>();
const calls: { command: string; args: Record<string, unknown> }[] = [];

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (command: string, args: Record<string, unknown> = {}) => {
    calls.push({ command, args });
    if (command === "flowkey_data_get") return disk.get(`${args.extensionId}/${args.name}`) ?? null;
    if (command === "flowkey_data_set") return void disk.set(`${args.extensionId}/${args.name}`, structuredClone(args.value));
    if (command === "open_url") return undefined;
    return null;
  }),
}));
vi.mock("@tauri-apps/api/event", () => ({ emit: vi.fn(async () => {}), listen: vi.fn(async () => () => {}) }));

const { dispatchNative, forgetStores } = await import("./natives");
const { missingPreferences, newerVersion, forgetCached } = await import("./extensions");
const { orderActions, toShortcut } = await import("./FlowKeyView");
const { actionIcon, lucideIcon } = await import("./FlowKeyIcon");

function extension(overrides: Partial<ExtensionManifest> = {}, source: "installed" | "bundled" = "bundled") {
  const manifest: ExtensionManifest = { id: "demo", name: "Demo", version: "1.0.0", commands: [{ id: "open", title: "Open" }], nativeMethods: ["storage.*", "cache.*", "shell.openUrl"], httpHosts: [], ...overrides };
  return { extension: { manifest, source, dir: "", baseUrl: "http://flowkey.localhost/bundled/demo/" } };
}

const call = (context: ReturnType<typeof extension>, method: string, params: Record<string, unknown> = {}) => dispatchNative(context, method, params, new AbortController().signal);

beforeEach(() => {
  disk.clear();
  calls.length = 0;
  forgetStores("demo");
  forgetCached("demo");
});

describe("flowkey native dispatcher", () => {
  it("rejects undeclared and ungranted methods", async () => {
    await expect(call(extension(), "clipboard.read")).rejects.toMatchObject({ code: "methodNotDeclared" });
    await expect(call(extension({}, "installed"), "storage.get", { key: "a" })).rejects.toMatchObject({ code: "methodNotDeclared" });
  });

  it("stores values and enforces the storage key quota", async () => {
    const context = extension();
    await call(context, "storage.set", { key: "a", value: { n: 1 } });
    expect(await call(context, "storage.get", { key: "a" })).toEqual({ ok: true, value: { n: 1 } });
    expect(await call(context, "storage.keys")).toEqual({ ok: true, keys: ["a"] });
    expect(disk.get("demo/storage")).toEqual({ a: { value: { n: 1 } } });
    for (let i = 0; i < 255; i++) await call(context, "storage.set", { key: `k${i}`, value: i });
    await expect(call(context, "storage.set", { key: "overflow", value: 1 })).rejects.toMatchObject({ code: "quotaExceeded" });
  });

  it("expires cache entries after their ttl", async () => {
    const context = extension();
    vi.useFakeTimers();
    try {
      await call(context, "cache.set", { key: "c", value: "v", ttlSeconds: 1 });
      expect(await call(context, "cache.get", { key: "c" })).toEqual({ ok: true, value: "v" });
      vi.advanceTimersByTime(1500);
      expect(await call(context, "cache.get", { key: "c" })).toEqual({ ok: true, value: null });
    } finally {
      vi.useRealTimers();
    }
  });

  it("opens only web links and declared schemes", async () => {
    await expect(call(extension(), "shell.openUrl", { url: "spotify:track:1" })).rejects.toMatchObject({ code: "schemeNotAllowed" });
    await call(extension({ uriSchemes: ["spotify"] }), "shell.openUrl", { url: "spotify:track:1" });
    expect(calls.some((c) => c.command === "open_url" && c.args.url === "spotify:track:1")).toBe(true);
  });
});

describe("flowkey helpers", () => {
  it("compares semver for package upgrades", () => {
    expect(newerVersion("1.0.1", "1.0.0")).toBe(true);
    expect(newerVersion("1.0.0", "1.0.0")).toBe(false);
    expect(newerVersion("1.0.0", "1.0.0-beta.2")).toBe(true);
    expect(newerVersion("1.0.0-beta.10", "1.0.0-beta.2")).toBe(true);
    expect(newerVersion("0.9.9", "1.0.0")).toBe(false);
  });

  it("lists required preferences that are still empty", () => {
    const manifest = extension({ preferences: [{ name: "token", type: "password", title: "Token", required: true }, { name: "on", type: "checkbox", title: "On", required: true }] }).extension.manifest;
    expect(missingPreferences(manifest, {}).map((p) => p.name)).toEqual(["token"]);
    expect(missingPreferences(manifest, { token: "x" })).toEqual([]);
  });

  it("maps FlowKey shortcuts and puts the primary action first", () => {
    expect(toShortcut({ key: "arrowRight", modifiers: ["cmd"] })).toEqual({ key: "ArrowRight", ctrl: true, alt: false, shift: false });
    expect(toShortcut({ key: "Q", modifiers: ["opt", "shift"] })).toEqual({ key: "q", ctrl: false, alt: true, shift: true });
    expect(orderActions([{ id: "a", title: "A" }, { id: "b", title: "B", primary: true }]).map((a) => a.id)).toEqual(["b", "a"]);
  });

  it("derives Raycast-style action icons from titles", () => {
    const icon = (title: string, name?: string) => actionIcon(title, name);
    expect(icon("Pause")).toBe(lucideIcon("pause"));
    expect(icon("Dislike")).toBe(lucideIcon("heart-off"));
    expect(icon("Next")).toBe(lucideIcon("forward"));
    expect(icon("Go to Album")).toBe(lucideIcon("layout-grid"));
    expect(icon("Remove from Library")).toBe(lucideIcon("minus"));
    expect(icon("Search Again")).toBe(lucideIcon("search"));
    expect(icon("Copy URL")).toBe(lucideIcon("link"));
    expect(icon("Restart test", "gauge")).toBe(lucideIcon("gauge"));
    expect(icon("Frobnicate")).toBeNull();
  });
});
