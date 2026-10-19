const pad = (value: number) => String(value).padStart(2, "0");

export function formatMs(milliseconds: number): string {
  const hours = Math.floor(milliseconds / 3_600_000);
  const minutes = Math.floor((milliseconds % 3_600_000) / 60_000);
  const seconds = Math.floor((milliseconds % 60_000) / 1000);
  return `${hours > 0 ? `${pad(hours)}:` : ""}${pad(minutes)}:${pad(seconds)}`;
}

export function cleanupSongTitle(title: string): string {
  const lower = title.toLowerCase().trim();
  if (title.startsWith("(") || title.startsWith("[") || lower.startsWith("feat.") || lower.startsWith("ft.") || lower.startsWith("featuring")) return title;
  const feat = title.match(/(\s+|\()(feat\.|ft\.|featuring)(\s|\))/i);
  const cut = Math.min(...[title.indexOf("("), title.indexOf("["), title.indexOf(" - "), feat?.index ?? -1].map((i) => (i === -1 ? Infinity : i)));
  return cut === Infinity ? title : title.slice(0, cut).trim();
}

export function embedCode(spotifyUrl?: string): string | undefined {
  if (!spotifyUrl) return undefined;
  const embedUrl = spotifyUrl.replace("open.spotify.com/", "open.spotify.com/embed/");
  return `<iframe style="border-radius:12px" src="${embedUrl}?utm_source=generator" width="100%" height="352" frameBorder="0" allowfullscreen="" allow="autoplay; clipboard-write; encrypted-media; fullscreen; picture-in-picture" loading="lazy"></iframe>`;
}

export function linkHtml(url: string | undefined, title: string): string {
  const escape = (value: string) => value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  return `<a href="${escape(url ?? "")}" title="${escape(title)}">${escape(title)}</a>`;
}

export function removeSyncedTimestamps(lyrics: string): string {
  return lyrics
    .split("\n")
    .map((line) => line.replace(/^\[\d{2}:\d{2}(?:\.\d{2,3})?\]\s*/, "").trim())
    .filter(Boolean)
    .join("\n");
}

export function normalizeLyrics(lyrics?: string | null): string | null {
  const normalized = lyrics
    ?.split("\n")
    .map((line) => line.trim())
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
  return normalized || null;
}

export function matchesQuery(query: string, ...fields: (string | undefined | null)[]): boolean {
  const needle = query.trim().toLowerCase();
  return !needle || fields.some((field) => field?.toLowerCase().includes(needle));
}

export function artistNames(artists?: { name: string }[]): string {
  return (artists ?? []).map((artist) => artist.name).join(", ");
}

export function smallestImage(images?: { url: string }[] | null): string | undefined {
  return images?.length ? images[images.length - 1]?.url : undefined;
}

export function largestImage(images?: { url: string }[] | null): string | undefined {
  return images?.[0]?.url;
}
