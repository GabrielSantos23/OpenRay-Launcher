import { describe, expect, it } from "vitest";
import { cleanupSongTitle, embedCode, formatMs, linkHtml, matchesQuery, normalizeLyrics, removeSyncedTimestamps } from "./format";
import { nextRepeatState } from "./spotify";

describe("spotify-player formatting", () => {
  it("formats durations like the reference", () => {
    expect(formatMs(0)).toBe("00:00");
    expect(formatMs(215_000)).toBe("03:35");
    expect(formatMs(3_725_000)).toBe("01:02:05");
  });

  it("cleans up song titles for lyric lookups", () => {
    expect(cleanupSongTitle("Song (Remastered 2011)")).toBe("Song");
    expect(cleanupSongTitle("Song - Live")).toBe("Song");
    expect(cleanupSongTitle("Song feat. Someone")).toBe("Song");
    expect(cleanupSongTitle("(Intro) Song")).toBe("(Intro) Song");
    expect(cleanupSongTitle("Plain")).toBe("Plain");
  });

  it("builds embed codes and escaped links", () => {
    expect(embedCode("https://open.spotify.com/track/abc")).toContain('src="https://open.spotify.com/embed/track/abc?utm_source=generator"');
    expect(embedCode(undefined)).toBeUndefined();
    expect(linkHtml("https://x/y", 'A "B" <C>')).toBe('<a href="https://x/y" title="A &quot;B&quot; &lt;C&gt;">A &quot;B&quot; &lt;C&gt;</a>');
  });

  it("normalizes LRCLIB lyrics", () => {
    expect(removeSyncedTimestamps("[00:01.00] Hello\n[00:02.50]\n[00:03.10] World")).toBe("Hello\nWorld");
    expect(normalizeLyrics("  a \n\n\n\n b ")).toBe("a\n\nb");
    expect(normalizeLyrics("   ")).toBeNull();
  });

  it("filters locally and cycles repeat states", () => {
    expect(matchesQuery("", "x")).toBe(true);
    expect(matchesQuery("beat", "The Beatles")).toBe(true);
    expect(matchesQuery("zzz", "The Beatles", undefined)).toBe(false);
    expect([nextRepeatState("off"), nextRepeatState("context"), nextRepeatState("track"), nextRepeatState(undefined)]).toEqual(["context", "track", "off", "off"]);
  });
});
