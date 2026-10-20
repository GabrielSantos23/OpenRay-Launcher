import { describe, expect, it } from "vitest";
import { sortApps } from "./app-volume";
import { sortDevices, volumeStep } from "./sound-devices";

describe("sound controls", () => {
  it("steps volume to the next multiple of five", () => {
    expect([volumeStep(80, true), volumeStep(82, true), volumeStep(82, false), volumeStep(80, false), volumeStep(98, true), volumeStep(0, false)]).toEqual([85, 85, 80, 75, 100, 0]);
  });

  it("puts the default device and playing apps first, System Sounds last", () => {
    const device = (name: string, isDefault: boolean) => ({ id: name, name, state: "active" as const, formFactor: 1, isDefault, volume: 50, isMuted: false });
    expect(sortDevices([device("B", false), device("C", true), device("A", false)]).map((d) => d.name)).toEqual(["C", "A", "B"]);
    const app = (id: string, name: string, isPlaying: boolean) => ({ id, name, path: null, volume: 50, isMuted: false, isPlaying });
    expect(sortApps([app("system", "System Sounds", false), app("b", "Zed", false), app("a", "Spotify", true)]).map((a) => a.name)).toEqual(["Spotify", "Zed", "System Sounds"]);
  });
});
