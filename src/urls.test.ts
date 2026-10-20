import { describe, expect, it } from "vitest";
import { toUrl } from "./urls";

describe("toUrl", () => {
  it("accepts links with a scheme", () => {
    expect(toUrl("https://raycast.com/store")).toBe("https://raycast.com/store");
    expect(toUrl("ftp://files.example.org")).toBe("ftp://files.example.org");
  });

  it("adds a scheme to bare domains, hosts and IPs", () => {
    expect(toUrl("github.com/ShareX/ShareX")).toBe("https://github.com/ShareX/ShareX");
    expect(toUrl("localhost:1420")).toBe("http://localhost:1420");
    expect(toUrl("192.168.0.1/admin")).toBe("http://192.168.0.1/admin");
  });

  it("rejects text, numbers and file names that are not links", () => {
    expect(toUrl("hello world")).toBeNull();
    expect(toUrl("notepad")).toBeNull();
    expect(toUrl("calc.exe")).toBeNull();
    expect(toUrl("notes.txt")).toBeNull();
    expect(toUrl("3.14")).toBeNull();
    expect(toUrl("1+2")).toBeNull();
    expect(toUrl("http://")).toBeNull();
  });
});
