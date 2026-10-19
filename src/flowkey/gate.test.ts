import { describe, expect, it } from "vitest";
import { hostAllowed, intersect, methodAllowed, missingConsent, NO_GRANTS, pathAllowed, schemeAllowed, scopeRoot } from "./gate";

describe("flowkey gate", () => {
  it("matches exact methods and one-level namespace wildcards", () => {
    expect(methodAllowed("storage.get", ["storage.*"])).toBe(true);
    expect(methodAllowed("http.fetch", ["http.fetch"])).toBe(true);
    expect(methodAllowed("http.upload", ["http.fetch"])).toBe(false);
    expect(methodAllowed("storagex.get", ["storage.*"])).toBe(false);
    expect(methodAllowed("clipboard.read", [])).toBe(false);
  });

  it("allows only declared hosts, suffix wildcards and ports", () => {
    expect(hostAllowed(new URL("https://api.spotify.com/v1/me"), ["api.spotify.com"])).toBe(true);
    expect(hostAllowed(new URL("https://evil.com/?h=api.spotify.com"), ["api.spotify.com"])).toBe(false);
    expect(hostAllowed(new URL("https://image-cdn-ak.spotifycdn.com/x"), [".spotifycdn.com"])).toBe(true);
    expect(hostAllowed(new URL("https://spotifycdn.com/x"), [".spotifycdn.com"])).toBe(false);
    expect(hostAllowed(new URL("https://api.spotify.com:8443/"), ["api.spotify.com"])).toBe(false);
    expect(hostAllowed(new URL("https://localhost:3000/"), ["localhost:3000"])).toBe(true);
    expect(hostAllowed(new URL("http://api.spotify.com/"), ["api.spotify.com"])).toBe(false);
    expect(hostAllowed(new URL("https://user:pw@api.spotify.com/"), ["api.spotify.com"])).toBe(false);
    expect(hostAllowed(new URL("file:///C:/x"), ["api.spotify.com"])).toBe(false);
  });

  it("intersects the manifest with stored consent", () => {
    const declared = { ...NO_GRANTS, nativeMethods: ["http.fetch", "storage.*"], httpHosts: ["a.com"] };
    const consented = { ...NO_GRANTS, nativeMethods: ["storage.*"], httpHosts: ["a.com", "b.com"] };
    expect(intersect(declared, consented)).toEqual({ ...NO_GRANTS, nativeMethods: ["storage.*"], httpHosts: ["a.com"] });
    expect(intersect(declared, undefined)).toEqual(NO_GRANTS);
    expect(missingConsent(declared, consented).nativeMethods).toEqual(["http.fetch"]);
  });

  it("opens web links and declared schemes only", () => {
    expect(schemeAllowed("https://open.spotify.com/track/1", [])).toBe(true);
    expect(schemeAllowed("spotify:track:1", ["spotify"])).toBe(true);
    expect(schemeAllowed("spotify:track:1", [])).toBe(false);
    expect(schemeAllowed("file:///C:/Windows", ["spotify"])).toBe(false);
    expect(schemeAllowed("javascript:alert(1)", [])).toBe(false);
  });

  it("scopes filesystem paths with globs and preference placeholders", () => {
    const scopes = ["{{vaultPath}}/**/*.md", "C:\\Data\\exact"];
    const prefs = { vaultPath: "D:\\Vault\\" };
    expect(pathAllowed("D:\\Vault\\notes\\a.md", scopes, prefs)).toBe(true);
    expect(pathAllowed("d:/vault/a.md", scopes, prefs)).toBe(true);
    expect(pathAllowed("D:\\Vault\\a.txt", scopes, prefs)).toBe(false);
    expect(pathAllowed("D:\\Vault\\..\\secret.md", scopes, prefs)).toBe(false);
    expect(pathAllowed("C:\\Data\\exact\\inner.bin", scopes, prefs)).toBe(true);
    expect(pathAllowed("relative.md", scopes, prefs)).toBe(false);
    expect(pathAllowed("D:\\Vault\\a.md", scopes, {})).toBe(false);
    expect(scopeRoot("{{vaultPath}}/**/*.md", prefs)).toBe("D:/Vault");
  });
});
