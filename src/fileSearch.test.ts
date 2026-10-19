import { describe, expect, it } from "vitest";
import { parentFolder, shortLocation } from "./fileSearch";

describe("fileSearch", () => {
  it("shortens locations under the home folder", () => {
    expect(parentFolder("C:\\Users\\ana\\Documents\\plan.md")).toBe("C:\\Users\\ana\\Documents");
    expect(shortLocation("C:\\Users\\ana\\Documents\\plan.md", "C:\\Users\\ana")).toBe("~\\Documents");
    expect(shortLocation("C:\\Users\\ana\\plan.md", "C:\\Users\\Ana")).toBe("~");
    expect(shortLocation("C:\\Users\\anabel\\plan.md", "C:\\Users\\ana")).toBe("C:\\Users\\anabel");
    expect(shortLocation("D:\\work\\plan.md", "C:\\Users\\ana")).toBe("D:\\work");
  });
});
