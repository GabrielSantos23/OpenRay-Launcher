import type { FlowKeyCapabilities } from "@flowkey-cli/native-sdk";

export type Caps = FlowKeyCapabilities;

export interface Image {
  url: string;
  width?: number | null;
  height?: number | null;
}

export interface ExternalUrls {
  spotify?: string;
}

export interface Artist {
  id: string;
  name: string;
  uri: string;
  images?: Image[];
  external_urls?: ExternalUrls;
}

export interface Album {
  id: string;
  name: string;
  uri: string;
  images: Image[];
  artists: Artist[];
  release_date?: string;
  total_tracks?: number;
  external_urls?: ExternalUrls;
}

export interface Track {
  id: string;
  name: string;
  uri: string;
  type: "track";
  duration_ms: number;
  artists: Artist[];
  album?: Album;
  external_urls?: ExternalUrls;
}

export interface Show {
  id: string;
  name: string;
  uri: string;
  description?: string;
  images: Image[];
  total_episodes?: number;
  external_urls?: ExternalUrls;
}

export interface Episode {
  id: string;
  name: string;
  uri: string;
  type: "episode";
  duration_ms: number;
  description?: string;
  images?: Image[];
  show?: Show;
  external_urls?: ExternalUrls;
}

export interface Playlist {
  id: string;
  name: string;
  uri: string;
  images?: Image[] | null;
  owner?: { id: string; display_name?: string | null };
  tracks?: { total?: number };
  items?: { total?: number };
  external_urls?: ExternalUrls;
}

export interface Device {
  id: string | null;
  name: string;
  type: string;
  is_active: boolean;
  is_restricted: boolean;
  volume_percent: number | null;
}

export interface CurrentlyPlaying {
  is_playing: boolean;
  progress_ms: number | null;
  currently_playing_type: "track" | "episode" | "ad" | "unknown";
  item: Track | Episode | null;
  context: { uri: string; type: string } | null;
}

export interface PlaybackState extends CurrentlyPlaying {
  device?: Device;
  shuffle_state?: boolean;
  repeat_state?: "off" | "context" | "track";
}

export interface Me {
  id: string;
  display_name?: string;
  country?: string;
}

interface Paging<T> {
  items: T[];
  next: string | null;
  total?: number;
}

export interface SearchResults {
  tracks?: Track[];
  artists?: Artist[];
  albums?: Album[];
  playlists?: Playlist[];
  shows?: Show[];
  episodes?: Episode[];
}

export type PlayType = "album" | "artist" | "playlist" | "track" | "show" | "episode";

export class SpotifyError extends Error {
  constructor(
    message: string,
    readonly status: number,
    readonly reason?: string,
  ) {
    super(message);
    this.name = "SpotifyError";
  }
}

export class WinNotSupportedError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "WinNotSupportedError";
  }
}

export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object" && "message" in error) return String((error as { message: unknown }).message);
  return String(error);
}

export function errorCode(error: unknown): string | undefined {
  return error && typeof error === "object" && "code" in error ? String((error as { code: unknown }).code) : undefined;
}

const API = "https://api.spotify.com/v1";
let rateLimitedUntil = 0;
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

type Query = Record<string, string | number | boolean | undefined>;

export async function request<T>(caps: Caps, method: string, path: string, options: { query?: Query; body?: unknown } = {}): Promise<T> {
  const url = new URL(path.startsWith("https://") ? path : `${API}${path}`);
  for (const [key, value] of Object.entries(options.query ?? {})) if (value !== undefined) url.searchParams.set(key, String(value));
  const send = () =>
    caps.http.fetch(url.href, {
      method,
      auth: "spotify",
      headers: options.body !== undefined ? { "Content-Type": "application/json" } : {},
      body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
    });
  if (rateLimitedUntil > Date.now()) await sleep(rateLimitedUntil - Date.now());
  let response = await send();
  if (response.status === 429) {
    const seconds = Number.parseInt(response.headers["retry-after"] ?? "1", 10);
    rateLimitedUntil = Date.now() + (Number.isNaN(seconds) ? 1 : seconds) * 1000;
    await sleep(rateLimitedUntil - Date.now());
    response = await send();
  }
  const parsed = (() => {
    try {
      return response.body ? JSON.parse(response.body) : undefined;
    } catch {
      return undefined;
    }
  })();
  if (response.status >= 400) {
    const detail = parsed?.error;
    const message = typeof detail === "string" ? (parsed.error_description ?? detail) : (detail?.message ?? `Spotify returned ${response.status}`);
    throw new SpotifyError(message, response.status, detail?.reason);
  }
  return parsed as T;
}

const cache = new Map<string, { at: number; value: Promise<unknown> }>();

export function cached<T>(key: string, ttlMs: number, load: () => Promise<T>): Promise<T> {
  const hit = cache.get(key);
  if (hit && Date.now() - hit.at < ttlMs) return hit.value as Promise<T>;
  const value = load();
  cache.set(key, { at: Date.now(), value });
  value.catch(() => cache.get(key)?.value === value && cache.delete(key));
  return value;
}

export function invalidate(...prefixes: string[]) {
  for (const key of [...cache.keys()]) if (prefixes.some((prefix) => key.startsWith(prefix))) cache.delete(key);
}

export async function authorize(caps: Caps, clientId?: string, interactive = true): Promise<boolean> {
  const status = await caps.oauth.status("spotify");
  if (status.ok) return true;
  if (!interactive) return false;
  const result = await caps.oauth.authorize("spotify", { clientId: clientId?.trim() || undefined, timeoutMs: 180_000 });
  return result.ok;
}

async function pages<T>(caps: Caps, first: Paging<T>, limit = Infinity): Promise<T[]> {
  const items = [...first.items];
  let next = first.next;
  while (next && items.length < limit) {
    const page = await request<Paging<T>>(caps, "GET", next);
    items.push(...page.items);
    next = page.next;
  }
  return items.slice(0, limit);
}

const present = <T>(items: (T | null | undefined)[] | undefined): T[] => (items ?? []).filter((item): item is T => item != null);

export const me = (caps: Caps) => cached("me", 3_600_000, () => request<Me>(caps, "GET", "/me"));

export const currentlyPlaying = (caps: Caps) =>
  cached("player:current", 10_000, async () => (await request<CurrentlyPlaying | undefined>(caps, "GET", "/me/player/currently-playing", { query: { additional_types: "episode" } })) ?? null);

export const playbackState = (caps: Caps) => request<PlaybackState | undefined>(caps, "GET", "/me/player", { query: { additional_types: "episode" } }).then((state) => state ?? null);

export const devices = (caps: Caps) => cached("player:devices", 30_000, async () => (await request<{ devices: Device[] }>(caps, "GET", "/me/player/devices")).devices ?? []);

export async function search(caps: Caps, query: string, types = ["track", "artist", "album", "playlist", "show", "episode"], limit = 50): Promise<SearchResults> {
  const response = await request<Record<string, Paging<unknown> | undefined>>(caps, "GET", "/search", { query: { q: query, type: types.join(","), limit } });
  return {
    tracks: present(response.tracks?.items as Track[]),
    artists: present(response.artists?.items as Artist[]),
    albums: present(response.albums?.items as Album[]),
    playlists: present(response.playlists?.items as Playlist[]),
    shows: present(response.shows?.items as Show[]),
    episodes: present(response.episodes?.items as Episode[]),
  };
}

export const savedTracks = (caps: Caps) =>
  cached("library:tracks", 300_000, async () => {
    const page = await request<Paging<{ track: Track | null }>>(caps, "GET", "/me/tracks", { query: { limit: 50 } });
    return { items: present(page.items.map((item) => item.track)), total: page.total ?? 0 };
  });

export const savedAlbums = (caps: Caps) =>
  cached("library:albums", 300_000, async () => present((await request<Paging<{ album: Album | null }>>(caps, "GET", "/me/albums", { query: { limit: 50 } })).items.map((item) => item.album)));

export const followedArtists = (caps: Caps) =>
  cached("library:artists", 300_000, async () => present((await request<{ artists: Paging<Artist> }>(caps, "GET", "/me/following", { query: { type: "artist", limit: 50 } })).artists.items));

export const savedShows = (caps: Caps) =>
  cached("library:shows", 300_000, async () => present((await request<Paging<{ show: Show | null }>>(caps, "GET", "/me/shows", { query: { limit: 50 } })).items.map((item) => item.show)));

export const savedEpisodes = (caps: Caps) =>
  cached("library:episodes", 300_000, async () => present((await request<Paging<{ episode: Episode | null }>>(caps, "GET", "/me/episodes", { query: { limit: 50 } })).items.map((item) => item.episode)));

export const myPlaylists = (caps: Caps) =>
  cached("library:playlists", 300_000, async () => present(await pages(caps, await request<Paging<Playlist | null>>(caps, "GET", "/me/playlists", { query: { limit: 50 } }))));

export async function contains(caps: Caps, kind: "tracks" | "albums", ids: string[]): Promise<Record<string, boolean>> {
  const size = kind === "tracks" ? 50 : 20;
  const unique = [...new Set(ids.filter(Boolean))];
  const result: Record<string, boolean> = {};
  for (let i = 0; i < unique.length; i += size) {
    const chunk = unique.slice(i, i + size);
    const flags = await cached(`contains:${kind}:${chunk.join(",")}`, 30_000, () => request<boolean[]>(caps, "GET", `/me/${kind}/contains`, { query: { ids: chunk.join(",") } }));
    chunk.forEach((id, j) => (result[id] = flags?.[j] === true));
  }
  return result;
}

export async function setSaved(caps: Caps, kind: "tracks" | "albums", ids: string[], saved: boolean) {
  await request(caps, saved ? "PUT" : "DELETE", `/me/${kind}`, { query: { ids: ids.join(",") } });
  invalidate(`contains:${kind}`, `library:${kind}`);
}

export const album = (caps: Caps, id: string) => cached(`album:${id}`, 300_000, () => request<Album>(caps, "GET", `/albums/${encodeURIComponent(id)}`));

export const albumTracks = (caps: Caps, id: string) =>
  cached(`album-tracks:${id}`, 300_000, async () => present((await request<Paging<Track>>(caps, "GET", `/albums/${encodeURIComponent(id)}/tracks`, { query: { limit: 50 } })).items));

export const playlistTracks = (caps: Caps, id: string, limit = 500) =>
  cached(`playlist-tracks:${id}`, 300_000, async () => {
    const first = await request<Paging<{ track: Track | Episode | null }>>(caps, "GET", `/playlists/${encodeURIComponent(id)}/tracks`, { query: { limit: 50 } });
    return present((await pages(caps, first, limit)).map((item) => item.track)).filter((item): item is Track => item.type === "track");
  });

export const artistAlbums = (caps: Caps, id: string) =>
  cached(`artist-albums:${id}`, 300_000, async () => present((await request<Paging<Album>>(caps, "GET", `/artists/${encodeURIComponent(id)}/albums`, { query: { limit: 50 } })).items));

export const artistTopTracks = (caps: Caps, id: string) =>
  cached(`artist-top:${id}`, 300_000, async () => {
    const { country } = await me(caps);
    return present((await request<{ tracks: Track[] }>(caps, "GET", `/artists/${encodeURIComponent(id)}/top-tracks`, { query: { market: country } })).tracks);
  });

export const show = (caps: Caps, id: string) => cached(`show:${id}`, 300_000, () => request<Show>(caps, "GET", `/shows/${encodeURIComponent(id)}`));

export const showEpisodes = (caps: Caps, id: string) =>
  cached(`show-episodes:${id}`, 300_000, async () => present((await request<Paging<Episode | null>>(caps, "GET", `/shows/${encodeURIComponent(id)}/episodes`, { query: { limit: 50 } })).items));

export const queue = async (caps: Caps) => present((await request<{ queue: (Track | Episode | null)[] }>(caps, "GET", "/me/player/queue")).queue);

export async function playlistContains(caps: Caps, playlistId: string, uri: string, signal?: AbortSignal): Promise<boolean> {
  let next: string | null = `${API}/playlists/${encodeURIComponent(playlistId)}/tracks?limit=50&fields=${encodeURIComponent("items(track(uri,linked_from(uri))),next")}`;
  while (next) {
    if (signal?.aborted) throw new Error("aborted");
    const page: Paging<{ track?: { uri?: string; linked_from?: { uri?: string } } | null }> = await request(caps, "GET", next);
    if (page.items.some(({ track }) => track?.uri === uri || track?.linked_from?.uri === uri)) return true;
    next = page.next;
  }
  return false;
}

export async function addToPlaylist(caps: Caps, playlistId: string, uris: string[]) {
  await request(caps, "POST", `/playlists/${encodeURIComponent(playlistId)}/tracks`, { body: { uris } });
  invalidate("library:playlists", `playlist-tracks:${playlistId}`);
}

export async function removeFromPlaylist(caps: Caps, playlistId: string, uris: string[]) {
  await request(caps, "DELETE", `/playlists/${encodeURIComponent(playlistId)}/tracks`, { body: { tracks: uris.map((uri) => ({ uri })) } });
  invalidate("library:playlists", `playlist-tracks:${playlistId}`);
}

export async function addToQueue(caps: Caps, uri: string) {
  await request(caps, "POST", "/me/player/queue", { query: { uri } });
}

export async function transferPlayback(caps: Caps, deviceId: string) {
  await request(caps, "PUT", "/me/player", { body: { device_ids: [deviceId] } });
  invalidate("player:");
}

const FALLBACK_ERRORS = ["restricted device", "premium required"];
const LAUNCH_ERRORS = ["no device found", "no active device", ...FALLBACK_ERRORS];

function matches(error: unknown, needles: string[]) {
  const message = errorMessage(error).toLowerCase();
  return needles.some((needle) => message.includes(needle));
}

export function isNoActiveDevice(error: unknown) {
  return matches(error, ["no active device"]);
}

async function withFallback(run: () => Promise<unknown>, fallback: () => Promise<unknown>) {
  try {
    await run();
  } catch (error) {
    if (!matches(error, FALLBACK_ERRORS)) throw new Error(errorMessage(error));
    await fallback();
  } finally {
    invalidate("player:");
  }
}

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

export const pause = (caps: Caps) => withFallback(() => request(caps, "PUT", "/me/player/pause"), () => caps.media.control("playPause"));

export const skipToNext = (caps: Caps) =>
  withFallback(
    async () => {
      await request(caps, "POST", "/me/player/next");
      await delay(100);
    },
    () => caps.media.control("next"),
  );

export const skipToPrevious = (caps: Caps) =>
  withFallback(
    async () => {
      await request(caps, "POST", "/me/player/previous");
      await delay(100);
    },
    () => caps.media.control("previous"),
  );

export const setVolume = (caps: Caps, volume: number) =>
  withFallback(
    () => request(caps, "PUT", "/me/player/volume", { query: { volume_percent: Math.round(volume) } }),
    () => Promise.reject(new WinNotSupportedError("Setting volume on non premium account is not supported on Windows")),
  );

export async function seek(caps: Caps, seconds: number, durationMs?: number): Promise<"next" | "position"> {
  if (durationMs !== undefined && Number.isFinite(durationMs) && seconds * 1000 > durationMs) {
    await skipToNext(caps);
    return "next";
  }
  await withFallback(
    () => request(caps, "PUT", "/me/player/seek", { query: { position_ms: Math.max(0, Math.round(seconds * 1000)) } }),
    () => Promise.reject(new WinNotSupportedError("Setting position on non premium account is not supported on Windows")),
  );
  return "position";
}

export async function setShuffle(caps: Caps, state: boolean) {
  await request(caps, "PUT", "/me/player/shuffle", { query: { state } });
  invalidate("player:");
}

export async function setRepeat(caps: Caps, state: "off" | "context" | "track") {
  await request(caps, "PUT", "/me/player/repeat", { query: { state } });
  invalidate("player:");
}

export function nextRepeatState(state?: string): "off" | "context" | "track" {
  return state === "off" ? "context" : state === "context" ? "track" : "off";
}

async function openInSpotify(caps: Caps, uri: string) {
  await caps.shell.openUrl(uri);
}

async function launchAndPlay(caps: Caps, id?: string, type?: PlayType) {
  if (!type || !id) {
    await openInSpotify(caps, "spotify:");
    await delay(1500);
    await caps.media.control("playPause");
    return;
  }
  await openInSpotify(caps, `spotify:${type}:${id}`);
}

export async function play(caps: Caps, { id, type, contextUri, uris }: { id?: string; type?: PlayType; contextUri?: string; uris?: string[] } = {}) {
  invalidate("player:");
  const available = await devices(caps).catch(() => [] as Device[]);
  const active = available.find((device) => device.is_active);
  try {
    if (!active) {
      if (uris?.length) {
        await openInSpotify(caps, "spotify:");
        for (let attempt = 0; ; attempt++) {
          try {
            invalidate("player:devices");
            const list = await devices(caps);
            const deviceId = list.find((d) => d.is_active)?.id ?? list[0]?.id ?? undefined;
            await request(caps, "PUT", "/me/player/play", { query: { device_id: deviceId ?? undefined }, body: { uris } });
            return;
          } catch (error) {
            if (attempt >= 5 || !isNoActiveDevice(error)) throw error;
            await delay(1000);
          }
        }
      }
      await launchAndPlay(caps, id, type);
      return;
    }
    const deviceId = active.id ?? available[0]?.id ?? undefined;
    const body = uris?.length
      ? { uris }
      : !type || !id
        ? contextUri
          ? { context_uri: contextUri }
          : undefined
        : type === "track"
          ? contextUri
            ? { context_uri: contextUri, offset: { uri: `spotify:track:${id}` } }
            : { uris: [`spotify:track:${id}`] }
          : type === "episode"
            ? { uris: [`spotify:episode:${id}`] }
            : { context_uri: `spotify:${type}:${id}` };
    await request(caps, "PUT", "/me/player/play", { query: { device_id: deviceId ?? undefined }, body });
  } catch (error) {
    if (matches(error, LAUNCH_ERRORS)) {
      await launchAndPlay(caps, id, type);
      return;
    }
    throw new Error(errorMessage(error));
  } finally {
    invalidate("player:");
  }
}

export async function startRadio(caps: Caps, { trackIds = [], artistIds = [] }: { trackIds?: string[]; artistIds?: string[] }) {
  try {
    const { tracks } = await request<{ tracks?: Track[] }>(caps, "GET", "/recommendations", {
      query: { seed_tracks: trackIds.join(",") || undefined, seed_artists: artistIds.join(",") || undefined },
    });
    if (!tracks?.length) return;
    try {
      await request(caps, "PUT", "/me/player/play", { body: { uris: tracks.map((track) => track.uri) } });
    } catch (error) {
      if (matches(error, LAUNCH_ERRORS)) await openInSpotify(caps, tracks[0].uri);
    }
  } catch (error) {
    throw new Error(errorMessage(error));
  } finally {
    invalidate("player:");
  }
}
