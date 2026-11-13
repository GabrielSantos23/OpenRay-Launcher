// ../../node_modules/@flowkey-cli/cli/dist/shims/flowkey-react-ui.ts
var host = globalThis.__FLOWKEY_HOST__;
var reactUi = host?.reactUi ?? {};
function missing(name) {
  throw new Error(
    `the FlowKey sidecar did not provide @flowkey-cli/react-ui (missing ${name}); rebuild the extension with @flowkey-cli/cli`
  );
}
var List = reactUi.List ?? (() => missing("List"));
var Detail = reactUi.Detail ?? (() => missing("Detail"));
var Grid = reactUi.Grid ?? (() => missing("Grid"));
var ActionPanel = reactUi.ActionPanel ?? (() => missing("ActionPanel"));
var Action = reactUi.Action ?? (() => missing("Action"));
var Form = reactUi.Form ?? missingForm;
function missingForm() {
  throw new Error(
    "the FlowKey sidecar did not provide @flowkey-cli/react-ui (missing Form); rebuild the extension with @flowkey-cli/cli"
  );
}
var defineReactExtension = reactUi.defineReactExtension ?? ((module) => module);
var usePromise = reactUi.usePromise ?? (() => missing("usePromise"));
var useFetch = reactUi.useFetch ?? (() => missing("useFetch"));
var useDebounce = reactUi.useDebounce ?? (() => missing("useDebounce"));
var useCachedState = reactUi.useCachedState ?? (() => missing("useCachedState"));
var useLocalStorage = reactUi.useLocalStorage ?? (() => missing("useLocalStorage"));
var useCachedPromise = reactUi.useCachedPromise ?? (() => missing("useCachedPromise"));
var withCache = reactUi.withCache ?? (() => missing("withCache"));
var Color = reactUi.Color ?? {};
var Icon = reactUi.Icon ?? {};

// manifest.json
var manifest_default = {
  id: "spotify-player",
  name: "Spotify Player",
  version: "1.0.0",
  description: "Search Spotify, browse your library and control playback, with the commands of Raycast's Spotify Player.",
  icon: "icon.png",
  commands: [
    {
      id: "search",
      title: "Search",
      subtitle: "Spotify",
      mode: "view",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "music",
        "song",
        "artist",
        "album",
        "playlist",
        "podcast"
      ]
    },
    {
      id: "your-library",
      title: "Your Library",
      subtitle: "Spotify",
      mode: "view",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "saved",
        "liked"
      ]
    },
    {
      id: "now-playing",
      title: "Now Playing",
      subtitle: "Spotify",
      mode: "view",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "current",
        "player"
      ]
    },
    {
      id: "toggle-play-pause",
      title: "Toggle Play/Pause",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "pause",
        "resume"
      ]
    },
    {
      id: "add-playing-song-to-playlist",
      title: "Add Playing Song to Playlist",
      subtitle: "Spotify",
      mode: "view",
      iconColor: "#1DB954",
      keywords: [
        "spotify"
      ]
    },
    {
      id: "remove-song-from-playlist",
      title: "Remove Playing Song from Playlist",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify"
      ]
    },
    {
      id: "next",
      title: "Next",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "skip"
      ]
    },
    {
      id: "previous",
      title: "Previous",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "back"
      ]
    },
    {
      id: "like",
      title: "Like",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "save"
      ]
    },
    {
      id: "dislike",
      title: "Dislike",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "unsave"
      ]
    },
    {
      id: "devices",
      title: "Select Device",
      subtitle: "Spotify",
      mode: "view",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "connect",
        "device"
      ]
    },
    {
      id: "volume",
      title: "Set Volume",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify"
      ],
      arguments: [
        {
          name: "volume",
          type: "text",
          placeholder: "Volume",
          required: true
        }
      ]
    },
    {
      id: "volume-0",
      title: "Set Volume to 0%",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "mute"
      ]
    },
    {
      id: "volume-25",
      title: "Set Volume to 25%",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify"
      ]
    },
    {
      id: "volume-50",
      title: "Set Volume to 50%",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify"
      ]
    },
    {
      id: "volume-75",
      title: "Set Volume to 75%",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify"
      ]
    },
    {
      id: "volume-100",
      title: "Set Volume to 100%",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify"
      ]
    },
    {
      id: "volume-down",
      title: "Turn Volume Down",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify"
      ]
    },
    {
      id: "volume-up",
      title: "Turn Volume up",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify"
      ]
    },
    {
      id: "toggle-shuffle",
      title: "Toggle Shuffle",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify"
      ]
    },
    {
      id: "cycle-repeat",
      title: "Cycle Repeat",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify"
      ]
    },
    {
      id: "replay",
      title: "Replay",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "restart"
      ]
    },
    {
      id: "current-track",
      title: "Current Track",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "status"
      ]
    },
    {
      id: "start-radio",
      title: "Start Radio",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify"
      ]
    },
    {
      id: "copy-url",
      title: "Copy URL",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "share"
      ]
    },
    {
      id: "copy-artist-and-title",
      title: "Copy Artist and Title",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify"
      ]
    },
    {
      id: "just-play",
      title: "Just Play",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify"
      ],
      arguments: [
        {
          name: "query",
          type: "text",
          placeholder: "Song / Artist",
          required: true
        }
      ]
    },
    {
      id: "queue",
      title: "Queue",
      subtitle: "Spotify",
      mode: "view",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "up next"
      ]
    },
    {
      id: "copy-embed",
      title: "Copy Embed Code",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "iframe"
      ]
    },
    {
      id: "skip-15",
      title: "Skip 15 Seconds",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "podcast"
      ]
    },
    {
      id: "back-15",
      title: "Back 15 Seconds",
      subtitle: "Spotify",
      mode: "background",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "podcast"
      ]
    },
    {
      id: "find-lyrics",
      title: "Find Lyrics",
      subtitle: "Spotify",
      mode: "view",
      iconColor: "#1DB954",
      keywords: [
        "spotify",
        "lyrics"
      ]
    }
  ],
  nativeMethods: [
    "http.fetch",
    "oauth.authorize",
    "oauth.status",
    "oauth.disconnect",
    "storage.*",
    "image.fetch",
    "clipboard.write",
    "shell.openUrl",
    "hud.show",
    "toast.show",
    "alert.confirm",
    "media.control"
  ],
  httpHosts: [
    "api.spotify.com",
    "lrclib.net",
    ".scdn.co",
    ".spotifycdn.com"
  ],
  oauth: [
    "spotify"
  ],
  uriSchemes: [
    "spotify"
  ],
  preferences: [
    {
      name: "clientId",
      type: "text",
      title: "Spotify Client ID",
      description: "Client ID of your own Spotify app (developer.spotify.com/dashboard). Register http://127.0.0.1/callback as its Redirect URI. Leave empty to use FLOWKEY_SPOTIFY_CLIENT_ID or spotify-auth.json.",
      required: false,
      placeholder: "32-character client ID"
    },
    {
      name: "closeWindowOnAction",
      type: "checkbox",
      title: "Close Window",
      label: "Close window on action",
      description: "If enabled, the launcher closes after performing an action",
      default: false,
      required: false
    },
    {
      name: "musicOnly",
      type: "checkbox",
      title: "Search Filters",
      label: "Music Only",
      default: false,
      required: false
    },
    {
      name: "topView",
      type: "dropdown",
      title: "Search Top View",
      default: "artists",
      required: false,
      options: [
        {
          title: "Playlists",
          value: "playlists"
        },
        {
          title: "Albums",
          value: "albums"
        },
        {
          title: "Artists",
          value: "artists"
        },
        {
          title: "Songs",
          value: "tracks"
        },
        {
          title: "Podcasts & Shows",
          value: "shows"
        },
        {
          title: "Episodes",
          value: "episodes"
        }
      ]
    },
    {
      name: "libraryDefaultView",
      type: "dropdown",
      title: "Library Default View",
      default: "all",
      required: false,
      options: [
        {
          title: "All",
          value: "all"
        },
        {
          title: "Playlists",
          value: "playlists"
        },
        {
          title: "Albums",
          value: "albums"
        },
        {
          title: "Artists",
          value: "artists"
        },
        {
          title: "Songs",
          value: "tracks"
        },
        {
          title: "Podcasts & Shows",
          value: "shows"
        },
        {
          title: "Episodes",
          value: "episodes"
        }
      ]
    }
  ]
};

// ../../node_modules/@flowkey-cli/cli/dist/shims/react.ts
var host2 = globalThis.__FLOWKEY_HOST__;
var react = host2?.react ?? {};
var useCallback = react.useCallback;
var useContext = react.useContext;
var useDebugValue = react.useDebugValue;
var useDeferredValue = react.useDeferredValue;
var useEffect = react.useEffect;
var useId = react.useId;
var useImperativeHandle = react.useImperativeHandle;
var useInsertionEffect = react.useInsertionEffect;
var useLayoutEffect = react.useLayoutEffect;
var useMemo = react.useMemo;
var useOptimistic = react.useOptimistic;
var useReducer = react.useReducer;
var useRef = react.useRef;
var useState = react.useState;
var useSyncExternalStore = react.useSyncExternalStore;
var useTransition = react.useTransition;
var Children = react.Children;
var Fragment = react.Fragment;
var Profiler = react.Profiler;
var StrictMode = react.StrictMode;
var Suspense = react.Suspense;
var cloneElement = react.cloneElement;
var createContext = react.createContext;
var createElement = react.createElement;
var forwardRef = react.forwardRef;
var isValidElement = react.isValidElement;
var lazy = react.lazy;
var memo = react.memo;
var startTransition = react.startTransition;

// src/format.ts
var pad = (value) => String(value).padStart(2, "0");
function formatMs(milliseconds) {
  const hours = Math.floor(milliseconds / 36e5);
  const minutes = Math.floor(milliseconds % 36e5 / 6e4);
  const seconds = Math.floor(milliseconds % 6e4 / 1e3);
  return `${hours > 0 ? `${pad(hours)}:` : ""}${pad(minutes)}:${pad(seconds)}`;
}
function cleanupSongTitle(title) {
  const lower = title.toLowerCase().trim();
  if (title.startsWith("(") || title.startsWith("[") || lower.startsWith("feat.") || lower.startsWith("ft.") || lower.startsWith("featuring")) return title;
  const feat = title.match(/(\s+|\()(feat\.|ft\.|featuring)(\s|\))/i);
  const cut = Math.min(...[title.indexOf("("), title.indexOf("["), title.indexOf(" - "), feat?.index ?? -1].map((i) => i === -1 ? Infinity : i));
  return cut === Infinity ? title : title.slice(0, cut).trim();
}
function embedCode(spotifyUrl) {
  if (!spotifyUrl) return void 0;
  const embedUrl = spotifyUrl.replace("open.spotify.com/", "open.spotify.com/embed/");
  return `<iframe style="border-radius:12px" src="${embedUrl}?utm_source=generator" width="100%" height="352" frameBorder="0" allowfullscreen="" allow="autoplay; clipboard-write; encrypted-media; fullscreen; picture-in-picture" loading="lazy"></iframe>`;
}
function linkHtml(url, title) {
  const escape = (value) => value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
  return `<a href="${escape(url ?? "")}" title="${escape(title)}">${escape(title)}</a>`;
}
function removeSyncedTimestamps(lyrics) {
  return lyrics.split("\n").map((line) => line.replace(/^\[\d{2}:\d{2}(?:\.\d{2,3})?\]\s*/, "").trim()).filter(Boolean).join("\n");
}
function normalizeLyrics(lyrics) {
  const normalized = lyrics?.split("\n").map((line) => line.trim()).join("\n").replace(/\n{3,}/g, "\n\n").trim();
  return normalized || null;
}
function matchesQuery(query, ...fields) {
  const needle = query.trim().toLowerCase();
  return !needle || fields.some((field) => field?.toLowerCase().includes(needle));
}
function artistNames(artists) {
  return (artists ?? []).map((artist) => artist.name).join(", ");
}
function smallestImage(images) {
  return images?.length ? images[images.length - 1]?.url : void 0;
}
function largestImage(images) {
  return images?.[0]?.url;
}

// src/spotify.ts
var SpotifyError = class extends Error {
  constructor(message, status, reason) {
    super(message);
    this.status = status;
    this.reason = reason;
    this.name = "SpotifyError";
  }
};
var WinNotSupportedError = class extends Error {
  constructor(message) {
    super(message);
    this.name = "WinNotSupportedError";
  }
};
function errorMessage(error) {
  if (error instanceof Error) return error.message;
  if (error && typeof error === "object" && "message" in error) return String(error.message);
  return String(error);
}
function errorCode(error) {
  return error && typeof error === "object" && "code" in error ? String(error.code) : void 0;
}
var API = "https://api.spotify.com/v1";
var rateLimitedUntil = 0;
var sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function request(caps, method, path, options = {}) {
  const url = new URL(path.startsWith("https://") ? path : `${API}${path}`);
  for (const [key, value] of Object.entries(options.query ?? {})) if (value !== void 0) url.searchParams.set(key, String(value));
  const send = () => caps.http.fetch(url.href, {
    method,
    auth: "spotify",
    headers: options.body !== void 0 ? { "Content-Type": "application/json" } : {},
    body: options.body !== void 0 ? JSON.stringify(options.body) : void 0
  });
  if (rateLimitedUntil > Date.now()) await sleep(rateLimitedUntil - Date.now());
  let response = await send();
  if (response.status === 429) {
    const seconds = Number.parseInt(response.headers["retry-after"] ?? "1", 10);
    rateLimitedUntil = Date.now() + (Number.isNaN(seconds) ? 1 : seconds) * 1e3;
    await sleep(rateLimitedUntil - Date.now());
    response = await send();
  }
  const parsed = (() => {
    try {
      return response.body ? JSON.parse(response.body) : void 0;
    } catch {
      return void 0;
    }
  })();
  if (response.status >= 400) {
    const detail = parsed?.error;
    const message = typeof detail === "string" ? parsed.error_description ?? detail : detail?.message ?? `Spotify returned ${response.status}`;
    throw new SpotifyError(message, response.status, detail?.reason);
  }
  return parsed;
}
var cache = /* @__PURE__ */ new Map();
function cached(key, ttlMs, load) {
  const hit = cache.get(key);
  if (hit && Date.now() - hit.at < ttlMs) return hit.value;
  const value = load();
  cache.set(key, { at: Date.now(), value });
  value.catch(() => cache.get(key)?.value === value && cache.delete(key));
  return value;
}
function invalidate(...prefixes) {
  for (const key of [...cache.keys()]) if (prefixes.some((prefix) => key.startsWith(prefix))) cache.delete(key);
}
async function authorize(caps, clientId, interactive = true) {
  const status = await caps.oauth.status("spotify");
  if (status.ok) return true;
  if (!interactive) return false;
  const result = await caps.oauth.authorize("spotify", { clientId: clientId?.trim() || void 0, timeoutMs: 18e4 });
  return result.ok;
}
async function pages(caps, first, limit = Infinity) {
  const items = [...first.items];
  let next = first.next;
  while (next && items.length < limit) {
    const page = await request(caps, "GET", next);
    items.push(...page.items);
    next = page.next;
  }
  return items.slice(0, limit);
}
var present = (items) => (items ?? []).filter((item) => item != null);
var me = (caps) => cached("me", 36e5, () => request(caps, "GET", "/me"));
var currentlyPlaying = (caps) => cached("player:current", 1e4, async () => await request(caps, "GET", "/me/player/currently-playing", { query: { additional_types: "episode" } }) ?? null);
var playbackState = (caps) => request(caps, "GET", "/me/player", { query: { additional_types: "episode" } }).then((state) => state ?? null);
var devices = (caps) => cached("player:devices", 3e4, async () => (await request(caps, "GET", "/me/player/devices")).devices ?? []);
async function search(caps, query, types = ["track", "artist", "album", "playlist", "show", "episode"], limit = 50) {
  const response = await request(caps, "GET", "/search", { query: { q: query, type: types.join(","), limit } });
  return {
    tracks: present(response.tracks?.items),
    artists: present(response.artists?.items),
    albums: present(response.albums?.items),
    playlists: present(response.playlists?.items),
    shows: present(response.shows?.items),
    episodes: present(response.episodes?.items)
  };
}
var savedTracks = (caps) => cached("library:tracks", 3e5, async () => {
  const page = await request(caps, "GET", "/me/tracks", { query: { limit: 50 } });
  return { items: present(page.items.map((item) => item.track)), total: page.total ?? 0 };
});
var savedAlbums = (caps) => cached("library:albums", 3e5, async () => present((await request(caps, "GET", "/me/albums", { query: { limit: 50 } })).items.map((item) => item.album)));
var followedArtists = (caps) => cached("library:artists", 3e5, async () => present((await request(caps, "GET", "/me/following", { query: { type: "artist", limit: 50 } })).artists.items));
var savedShows = (caps) => cached("library:shows", 3e5, async () => present((await request(caps, "GET", "/me/shows", { query: { limit: 50 } })).items.map((item) => item.show)));
var savedEpisodes = (caps) => cached("library:episodes", 3e5, async () => present((await request(caps, "GET", "/me/episodes", { query: { limit: 50 } })).items.map((item) => item.episode)));
var myPlaylists = (caps) => cached("library:playlists", 3e5, async () => present(await pages(caps, await request(caps, "GET", "/me/playlists", { query: { limit: 50 } }))));
async function contains(caps, kind, ids) {
  const size = kind === "tracks" ? 50 : 20;
  const unique = [...new Set(ids.filter(Boolean))];
  const result = {};
  for (let i = 0; i < unique.length; i += size) {
    const chunk = unique.slice(i, i + size);
    const flags = await cached(`contains:${kind}:${chunk.join(",")}`, 3e4, () => request(caps, "GET", `/me/${kind}/contains`, { query: { ids: chunk.join(",") } }));
    chunk.forEach((id, j) => result[id] = flags?.[j] === true);
  }
  return result;
}
async function setSaved(caps, kind, ids, saved) {
  await request(caps, saved ? "PUT" : "DELETE", `/me/${kind}`, { query: { ids: ids.join(",") } });
  invalidate(`contains:${kind}`, `library:${kind}`);
}
var album = (caps, id) => cached(`album:${id}`, 3e5, () => request(caps, "GET", `/albums/${encodeURIComponent(id)}`));
var albumTracks = (caps, id) => cached(`album-tracks:${id}`, 3e5, async () => present((await request(caps, "GET", `/albums/${encodeURIComponent(id)}/tracks`, { query: { limit: 50 } })).items));
var playlistTracks = (caps, id, limit = 500) => cached(`playlist-tracks:${id}`, 3e5, async () => {
  const first = await request(caps, "GET", `/playlists/${encodeURIComponent(id)}/tracks`, { query: { limit: 50 } });
  return present((await pages(caps, first, limit)).map((item) => item.track)).filter((item) => item.type === "track");
});
var artistAlbums = (caps, id) => cached(`artist-albums:${id}`, 3e5, async () => present((await request(caps, "GET", `/artists/${encodeURIComponent(id)}/albums`, { query: { limit: 50 } })).items));
var artistTopTracks = (caps, id) => cached(`artist-top:${id}`, 3e5, async () => {
  const { country } = await me(caps);
  return present((await request(caps, "GET", `/artists/${encodeURIComponent(id)}/top-tracks`, { query: { market: country } })).tracks);
});
var show = (caps, id) => cached(`show:${id}`, 3e5, () => request(caps, "GET", `/shows/${encodeURIComponent(id)}`));
var showEpisodes = (caps, id) => cached(`show-episodes:${id}`, 3e5, async () => present((await request(caps, "GET", `/shows/${encodeURIComponent(id)}/episodes`, { query: { limit: 50 } })).items));
var queue = async (caps) => present((await request(caps, "GET", "/me/player/queue")).queue);
async function playlistContains(caps, playlistId, uri, signal) {
  let next = `${API}/playlists/${encodeURIComponent(playlistId)}/tracks?limit=50&fields=${encodeURIComponent("items(track(uri,linked_from(uri))),next")}`;
  while (next) {
    if (signal?.aborted) throw new Error("aborted");
    const page = await request(caps, "GET", next);
    if (page.items.some(({ track }) => track?.uri === uri || track?.linked_from?.uri === uri)) return true;
    next = page.next;
  }
  return false;
}
async function addToPlaylist(caps, playlistId, uris) {
  await request(caps, "POST", `/playlists/${encodeURIComponent(playlistId)}/tracks`, { body: { uris } });
  invalidate("library:playlists", `playlist-tracks:${playlistId}`);
}
async function removeFromPlaylist(caps, playlistId, uris) {
  await request(caps, "DELETE", `/playlists/${encodeURIComponent(playlistId)}/tracks`, { body: { tracks: uris.map((uri) => ({ uri })) } });
  invalidate("library:playlists", `playlist-tracks:${playlistId}`);
}
async function addToQueue(caps, uri) {
  await request(caps, "POST", "/me/player/queue", { query: { uri } });
}
async function transferPlayback(caps, deviceId) {
  await request(caps, "PUT", "/me/player", { body: { device_ids: [deviceId] } });
  invalidate("player:");
}
var FALLBACK_ERRORS = ["restricted device", "premium required"];
var LAUNCH_ERRORS = ["no device found", "no active device", ...FALLBACK_ERRORS];
function matches(error, needles) {
  const message = errorMessage(error).toLowerCase();
  return needles.some((needle) => message.includes(needle));
}
function isNoActiveDevice(error) {
  return matches(error, ["no active device"]);
}
async function withFallback(run, fallback2) {
  try {
    await run();
  } catch (error) {
    if (!matches(error, FALLBACK_ERRORS)) throw new Error(errorMessage(error));
    await fallback2();
  } finally {
    invalidate("player:");
  }
}
var delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
var pause = (caps) => withFallback(() => request(caps, "PUT", "/me/player/pause"), () => caps.media.control("playPause"));
var skipToNext = (caps) => withFallback(
  async () => {
    await request(caps, "POST", "/me/player/next");
    await delay(100);
  },
  () => caps.media.control("next")
);
var skipToPrevious = (caps) => withFallback(
  async () => {
    await request(caps, "POST", "/me/player/previous");
    await delay(100);
  },
  () => caps.media.control("previous")
);
var setVolume = (caps, volume) => withFallback(
  () => request(caps, "PUT", "/me/player/volume", { query: { volume_percent: Math.round(volume) } }),
  () => Promise.reject(new WinNotSupportedError("Setting volume on non premium account is not supported on Windows"))
);
async function seek(caps, seconds, durationMs) {
  if (durationMs !== void 0 && Number.isFinite(durationMs) && seconds * 1e3 > durationMs) {
    await skipToNext(caps);
    return "next";
  }
  await withFallback(
    () => request(caps, "PUT", "/me/player/seek", { query: { position_ms: Math.max(0, Math.round(seconds * 1e3)) } }),
    () => Promise.reject(new WinNotSupportedError("Setting position on non premium account is not supported on Windows"))
  );
  return "position";
}
async function setShuffle(caps, state) {
  await request(caps, "PUT", "/me/player/shuffle", { query: { state } });
  invalidate("player:");
}
async function setRepeat(caps, state) {
  await request(caps, "PUT", "/me/player/repeat", { query: { state } });
  invalidate("player:");
}
function nextRepeatState(state) {
  return state === "off" ? "context" : state === "context" ? "track" : "off";
}
async function openInSpotify(caps, uri) {
  await caps.shell.openUrl(uri);
}
async function launchAndPlay(caps, id, type) {
  if (!type || !id) {
    await openInSpotify(caps, "spotify:");
    await delay(1500);
    await caps.media.control("playPause");
    return;
  }
  await openInSpotify(caps, `spotify:${type}:${id}`);
}
async function play(caps, { id, type, contextUri, uris } = {}) {
  invalidate("player:");
  const available = await devices(caps).catch(() => []);
  const active = available.find((device) => device.is_active);
  try {
    if (!active) {
      if (uris?.length) {
        await openInSpotify(caps, "spotify:");
        for (let attempt = 0; ; attempt++) {
          try {
            invalidate("player:devices");
            const list = await devices(caps);
            const deviceId2 = list.find((d) => d.is_active)?.id ?? list[0]?.id ?? void 0;
            await request(caps, "PUT", "/me/player/play", { query: { device_id: deviceId2 ?? void 0 }, body: { uris } });
            return;
          } catch (error) {
            if (attempt >= 5 || !isNoActiveDevice(error)) throw error;
            await delay(1e3);
          }
        }
      }
      await launchAndPlay(caps, id, type);
      return;
    }
    const deviceId = active.id ?? available[0]?.id ?? void 0;
    const body = uris?.length ? { uris } : !type || !id ? contextUri ? { context_uri: contextUri } : void 0 : type === "track" ? contextUri ? { context_uri: contextUri, offset: { uri: `spotify:track:${id}` } } : { uris: [`spotify:track:${id}`] } : type === "episode" ? { uris: [`spotify:episode:${id}`] } : { context_uri: `spotify:${type}:${id}` };
    await request(caps, "PUT", "/me/player/play", { query: { device_id: deviceId ?? void 0 }, body });
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
async function startRadio(caps, { trackIds = [], artistIds = [] }) {
  try {
    const { tracks } = await request(caps, "GET", "/recommendations", {
      query: { seed_tracks: trackIds.join(",") || void 0, seed_artists: artistIds.join(",") || void 0 }
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

// ../../node_modules/@flowkey-cli/cli/dist/shims/react-jsx-runtime.ts
var host3 = globalThis.__FLOWKEY_HOST__;
var jsxRuntime = host3?.reactJsx ?? {};
function missing2(name) {
  throw new Error(
    `the FlowKey sidecar did not provide the JSX runtime (missing ${name}); rebuild the extension with @flowkey-cli/cli`
  );
}
var fallback = () => missing2("jsx");
var Fragment2 = jsxRuntime.Fragment;
var jsx = jsxRuntime.jsx ?? fallback;
var jsxs = jsxRuntime.jsxs ?? fallback;
var jsxDEV = jsxRuntime.jsxDEV ?? jsx;

// src/ui.tsx
var GREEN = "#1DB954";
function ctxOf(props) {
  return { caps: props.capabilities, window: props.window, prefs: props.preferences, signal: props.signal };
}
var sc = (key, ...modifiers) => ({ key, modifiers });
var SHORTCUTS = {
  showContent: sc("a", "ctrl", "shift"),
  addToPlaylist: sc("a", "ctrl"),
  like: sc("l", "ctrl"),
  dislike: sc("d", "ctrl"),
  startRadio: sc("r", "ctrl", "shift"),
  connectDevice: sc("k", "ctrl", "shift"),
  openSearch: sc("f", "ctrl", "shift"),
  openLibrary: sc("l", "ctrl", "shift"),
  addToQueue: sc("q", "alt"),
  next: sc("arrowRight", "ctrl"),
  previous: sc("arrowLeft", "ctrl"),
  copyUrl: sc("c", "ctrl", "shift"),
  copyEmbed: sc("e", "ctrl", "shift"),
  copyTitle: sc("t", "ctrl", "shift"),
  open: sc("o", "ctrl"),
  refresh: sc("r", "ctrl"),
  quickLook: sc("y", "ctrl"),
  remove: sc("x", "ctrl"),
  removeAll: sc("x", "ctrl", "shift")
};
async function notify(ctx, title, message) {
  if (ctx.prefs.closeWindowOnAction) {
    await ctx.caps.hud.show({ title });
    ctx.window.popToRoot();
  } else await ctx.caps.toast.show({ title, message, style: "success" });
}
async function notifyFailure(ctx, title, error, hudTitle) {
  const message = errorMessage(error);
  if (ctx.prefs.closeWindowOnAction) {
    await ctx.caps.hud.show({ title: hudTitle ?? message });
    ctx.window.popToRoot();
  } else await ctx.caps.toast.show({ title, message, style: "failure" });
}
async function copy(ctx, content) {
  await ctx.caps.clipboard.writeContent(content);
  await ctx.caps.hud.show({ title: "Copied to Clipboard" });
}
async function openOnSpotify(ctx, uri, url) {
  try {
    await ctx.caps.shell.openUrl(uri || "spotify:");
  } catch {
    await ctx.caps.shell.openUrl(url || "https://open.spotify.com");
  }
  ctx.window.closeMainWindow();
}
var stash = /* @__PURE__ */ new Map();
function remember(key, value) {
  stash.set(key, value);
  return key;
}
function recall(key) {
  return stash.get(key);
}
function useAsync(load, deps, execute = true) {
  const [state, setState] = useState({ loading: execute });
  const [tick, setTick] = useState(0);
  const loader = useRef(load);
  loader.current = load;
  useEffect(() => {
    if (!execute) {
      setState((s) => ({ ...s, loading: false }));
      return;
    }
    let alive = true;
    setState((s) => ({ data: s.data, loading: true }));
    loader.current().then(
      (data) => alive && setState({ data, loading: false }),
      (error) => alive && setState((s) => ({ data: s.data, error, loading: false }))
    );
    return () => {
      alive = false;
    };
  }, [...deps, tick, execute]);
  return { ...state, reload: () => setTick((t) => t + 1) };
}
function useDebounced(value, ms) {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), ms);
    return () => clearTimeout(timer);
  }, [value, ms]);
  return debounced;
}
var artwork = /* @__PURE__ */ new Map();
var inflight = /* @__PURE__ */ new Map();
function useArtwork(ctx, urls) {
  const [, setVersion] = useState(0);
  const wanted = [...new Set(urls.filter((u) => !!u))];
  const key = wanted.join("|");
  useEffect(() => {
    let alive = true;
    let timer;
    const bump = () => {
      if (!alive || timer) return;
      timer = setTimeout(() => {
        timer = void 0;
        if (alive) setVersion((v) => v + 1);
      }, 60);
    };
    const missing3 = wanted.filter((url) => !artwork.has(url));
    let index = 0;
    const worker = async () => {
      while (alive && index < missing3.length) {
        const url = missing3[index++];
        const pending = inflight.get(url) ?? ctx.caps.image.fetch(url).then((uri) => void artwork.set(url, uri || null)).catch(() => void artwork.set(url, null)).finally(() => inflight.delete(url));
        inflight.set(url, pending);
        await pending;
        bump();
      }
    };
    void Promise.all(Array.from({ length: Math.min(6, missing3.length) }, worker));
    return () => {
      alive = false;
      if (timer) clearTimeout(timer);
    };
  }, [key]);
  return (url) => url ? artwork.get(url) ?? void 0 : void 0;
}
function useSaved(ctx, kind, ids) {
  const key = [...new Set(ids)].join(",");
  return useAsync(() => contains(ctx.caps, kind, ids), [key], ids.length > 0);
}
var iconFor = (art, url, fallback2) => {
  const uri = art(url);
  return uri ? { uri } : { lucide: fallback2, color: GREEN };
};
function footerActions(ctx, key, { url, uri, title }) {
  const embed = embedCode(url);
  const actions = [
    /* @__PURE__ */ jsx(Action, { id: `${key}:copy-url`, title: "Copy URL", shortcut: SHORTCUTS.copyUrl, onAction: () => copy(ctx, { text: url ?? "", html: linkHtml(url, title) }) }, "copy-url"),
    embed ? /* @__PURE__ */ jsx(Action, { id: `${key}:copy-embed`, title: "Copy Embed Code", shortcut: SHORTCUTS.copyEmbed, onAction: () => copy(ctx, { text: embed }) }, "copy-embed") : null,
    /* @__PURE__ */ jsx(Action, { id: `${key}:copy-title`, title: "Copy Artist and Title", shortcut: SHORTCUTS.copyTitle, onAction: () => copy(ctx, { text: title }) }, "copy-title"),
    /* @__PURE__ */ jsx(Action, { id: `${key}:open`, title: "Open on Spotify", shortcut: SHORTCUTS.open, onAction: () => openOnSpotify(ctx, uri, url) }, "open")
  ];
  return actions.filter((a) => a !== null);
}
async function runPlay(ctx, options, onPlay) {
  try {
    await play(ctx.caps, options);
    if (ctx.prefs.closeWindowOnAction) {
      await ctx.caps.hud.show({ title: "Playing" });
      ctx.window.popToRoot();
    } else {
      await ctx.caps.toast.show({ title: "Playing", style: "success" });
      onPlay?.();
    }
  } catch (error) {
    const message = errorMessage(error);
    if (ctx.prefs.closeWindowOnAction) {
      await ctx.caps.hud.show({ title: "Failed to play" });
      ctx.window.popToRoot();
    } else await ctx.caps.toast.show({ title: message.toLowerCase().includes("no active device") ? "No active device" : "Failed to play", message, style: "failure" });
  }
}
function playAction(ctx, key, options, onPlay) {
  return /* @__PURE__ */ jsx(Action, { id: `${key}:play`, title: "Play", primary: true, onAction: () => runPlay(ctx, options, onPlay) }, "play");
}
function pauseAction(ctx, key, onPause) {
  return /* @__PURE__ */ jsx(
    Action,
    {
      id: `${key}:pause`,
      title: "Pause",
      primary: true,
      onAction: async () => {
        try {
          await pause(ctx.caps);
          if (ctx.prefs.closeWindowOnAction) {
            await ctx.caps.hud.show({ title: "Paused" });
            ctx.window.popToRoot();
            return;
          }
          await ctx.caps.toast.show({ title: "Paused", style: "success" });
          onPause?.();
        } catch (error) {
          await notifyFailure(ctx, "Something went wrong", error);
        }
      }
    },
    "pause"
  );
}
function startRadioAction(ctx, key, seeds, onStarted) {
  return /* @__PURE__ */ jsx(
    Action,
    {
      id: `${key}:radio`,
      title: "Start Radio",
      shortcut: SHORTCUTS.startRadio,
      onAction: async () => {
        try {
          await startRadio(ctx.caps, { trackIds: seeds.trackId ? [seeds.trackId] : [], artistIds: seeds.artistId ? [seeds.artistId] : [] });
          if (ctx.prefs.closeWindowOnAction) {
            await ctx.caps.hud.show({ title: "Playing Radio" });
            ctx.window.popToRoot();
            return;
          }
          await ctx.caps.toast.show({ title: "Playing Radio", style: "success" });
          if (onStarted) setTimeout(onStarted, 300);
        } catch (error) {
          await notifyFailure(ctx, "Error Starting Radio", error);
        }
      }
    },
    "radio"
  );
}
function likeAction(ctx, key, trackId, liked, onChange) {
  const like = !liked;
  return /* @__PURE__ */ jsx(
    Action,
    {
      id: `${key}:${like ? "like" : "dislike"}`,
      title: like ? "Like" : "Dislike",
      shortcut: like ? SHORTCUTS.like : SHORTCUTS.dislike,
      onAction: async () => {
        try {
          await setSaved(ctx.caps, "tracks", [trackId], like);
          if (ctx.prefs.closeWindowOnAction) {
            await ctx.caps.hud.show({ title: like ? "Liked" : "Disliked" });
            ctx.window.popToRoot();
            return;
          }
          await ctx.caps.toast.show({ title: like ? "Liked" : "Disliked", style: "success" });
          onChange?.();
        } catch (error) {
          await notifyFailure(ctx, "Something went wrong", error);
        }
      }
    },
    "like"
  );
}
function queueAction(ctx, key, uri, title) {
  return /* @__PURE__ */ jsx(
    Action,
    {
      id: `${key}:queue`,
      title: "Add to Queue",
      shortcut: SHORTCUTS.addToQueue,
      onAction: async () => {
        try {
          await addToQueue(ctx.caps, uri);
          if (ctx.prefs.closeWindowOnAction) {
            await ctx.caps.hud.show({ title: "Added to queue" });
            ctx.window.popToRoot();
            return;
          }
          await ctx.caps.toast.show({ title: "Added to queue", message: title, style: "success" });
        } catch (error) {
          await ctx.caps.toast.show({ title: "Error adding song to queue", message: errorMessage(error), style: "failure" });
        }
      }
    },
    "queue"
  );
}
function addToPlaylistAction(key, uri) {
  const target = `add-to-playlist:${uri}`;
  return /* @__PURE__ */ jsx(Action, { id: `${key}:add-to-playlist`, title: "Add to Playlist", shortcut: SHORTCUTS.addToPlaylist, push: target, onAction: () => {
  } }, "add-to-playlist");
}
function pushAction(key, id, title, target, shortcut) {
  return /* @__PURE__ */ jsx(Action, { id: `${key}:${id}`, title, shortcut, push: target, onAction: () => {
  } }, id);
}
function trackActions(ctx, key, track, options) {
  const album2 = options.album ?? track.album;
  return [
    playAction(ctx, key, { id: track.id, type: "track", contextUri: options.playingContext }),
    album2 && options.showGoToAlbum ? pushAction(key, "go-to-album", "Go to Album", remember(`album:${album2.id}`, album2), SHORTCUTS.showContent) : null,
    startRadioAction(ctx, key, { trackId: track.id }),
    options.showAddToSaved && track.id ? likeAction(ctx, key, track.id, options.liked, options.onLikedChange) : null,
    track.uri ? queueAction(ctx, key, track.uri, track.name) : null,
    track.uri ? addToPlaylistAction(key, track.uri) : null,
    ...footerActions(ctx, key, { url: track.external_urls?.spotify, uri: track.uri, title: track.name })
  ].filter((a) => a !== null);
}
function trackItem(ctx, art, key, track, options) {
  const album2 = options.album ?? track.album;
  return /* @__PURE__ */ jsx(
    List.Item,
    {
      id: key,
      title: track.name || "Unknown",
      subtitle: artistNames(track.artists),
      icon: iconFor(art, smallestImage(album2?.images), "music"),
      accessories: track.duration_ms ? [{ text: formatMs(track.duration_ms) }] : [],
      actions: trackActions(ctx, key, track, options)
    },
    key
  );
}
function episodeItem(ctx, art, key, episode, showOf) {
  const image = smallestImage(showOf?.images) ?? smallestImage(episode.images);
  return /* @__PURE__ */ jsx(
    List.Item,
    {
      id: key,
      title: episode.name || "Unknown",
      icon: iconFor(art, image, "podcast"),
      accessories: episode.duration_ms ? [{ text: formatMs(episode.duration_ms) }] : [],
      actions: [playAction(ctx, key, { id: episode.id, type: "episode" }), queueAction(ctx, key, episode.uri, episode.name), ...footerActions(ctx, key, { url: episode.external_urls?.spotify, uri: episode.uri, title: episode.name })]
    },
    key
  );
}
function albumActions(ctx, key, album2, saved, onChange) {
  return [
    playAction(ctx, key, { id: album2.id, type: "album" }),
    pushAction(key, "show-songs", "Show Songs", remember(`album:${album2.id}`, album2), SHORTCUTS.showContent),
    /* @__PURE__ */ jsx(
      Action,
      {
        id: `${key}:library`,
        title: saved ? "Remove from Library" : "Add to Library",
        shortcut: { key: saved ? "d" : "s", modifiers: ["ctrl", "shift"] },
        onAction: async () => {
          try {
            await setSaved(ctx.caps, "albums", [album2.id], !saved);
            onChange?.();
            await ctx.caps.hud.show({ title: saved ? "Album removed from the library" : "Album added to the library" });
            ctx.window.popToRoot();
          } catch (error) {
            await ctx.caps.hud.show({ title: errorMessage(error) });
          }
        }
      },
      "library"
    ),
    ...footerActions(ctx, key, { url: album2.external_urls?.spotify, uri: album2.uri, title: album2.name })
  ];
}
function albumItem(ctx, art, key, album2, grid, saved, onChange) {
  const subtitle = `${artistNames(album2.artists)}${album2.release_date ? ` \xB7 ${album2.release_date.substring(0, 4)}` : ""}`;
  const icon = iconFor(art, grid ? album2.images?.[1]?.url ?? largestImage(album2.images) : largestImage(album2.images), "disc-3");
  const actions = albumActions(ctx, key, album2, saved, onChange);
  return grid ? /* @__PURE__ */ jsx(Grid.Item, { id: key, title: album2.name || "Unknown", subtitle, icon, actions }, key) : /* @__PURE__ */ jsx(List.Item, { id: key, title: album2.name || "Unknown", subtitle, icon, accessories: album2.total_tracks !== void 0 ? [{ text: `${album2.total_tracks} songs` }] : [], actions }, key);
}
function artistItem(ctx, art, key, artist, grid) {
  const icon = iconFor(art, grid ? artist.images?.[1]?.url ?? largestImage(artist.images) : smallestImage(artist.images), "mic-vocal");
  const actions = [
    playAction(ctx, key, { id: artist.id, type: "artist" }),
    pushAction(key, "show-albums", "Show Albums", remember(`artist-albums:${artist.id}`, artist), SHORTCUTS.showContent),
    pushAction(key, "popular", "Show Popular Songs", remember(`artist-top:${artist.id}`, artist), SHORTCUTS.quickLook),
    startRadioAction(ctx, key, { artistId: artist.id }),
    ...footerActions(ctx, key, { url: artist.external_urls?.spotify, uri: artist.uri, title: artist.name })
  ];
  const title = artist.name || "Unknown";
  return grid ? /* @__PURE__ */ jsx(Grid.Item, { id: key, title, icon, actions }, key) : /* @__PURE__ */ jsx(List.Item, { id: key, title, icon, actions }, key);
}
function playlistItem(ctx, art, key, playlist, grid) {
  const total = playlist.tracks?.total ?? playlist.items?.total;
  const icon = iconFor(art, grid ? largestImage(playlist.images) : smallestImage(playlist.images), "list-music");
  const actions = [
    playAction(ctx, key, { id: playlist.id, type: "playlist" }),
    pushAction(key, "show-songs", "Show Songs", remember(`playlist:${playlist.id}`, playlist), SHORTCUTS.showContent),
    ...footerActions(ctx, key, { url: playlist.external_urls?.spotify, uri: playlist.uri, title: playlist.name })
  ];
  return grid ? /* @__PURE__ */ jsx(Grid.Item, { id: key, title: playlist.name || "Untitled", subtitle: playlist.owner?.display_name ?? void 0, icon, actions }, key) : /* @__PURE__ */ jsx(List.Item, { id: key, title: playlist.name || "Untitled", subtitle: playlist.owner?.display_name ?? void 0, icon, accessories: total !== void 0 ? [{ text: `${total} songs` }] : [], actions }, key);
}
var LIKED_SONGS_IMAGE = "https://misc.scdn.co/liked-songs/liked-songs-64.png";
function likedSongsItem(ctx, art, key, userId, total, grid) {
  const uri = `spotify:user:${userId}:collection`;
  const icon = iconFor(art, LIKED_SONGS_IMAGE, "heart");
  const actions = [
    ...userId ? [playAction(ctx, key, { contextUri: uri }), pushAction(key, "show-songs", "Show Songs", "liked", SHORTCUTS.showContent)] : [],
    ...footerActions(ctx, key, { url: "https://open.spotify.com/collection/tracks", uri, title: "Liked Songs" })
  ];
  return grid ? /* @__PURE__ */ jsx(Grid.Item, { id: key, title: "Liked Songs", icon, actions }, key) : /* @__PURE__ */ jsx(List.Item, { id: key, title: "Liked Songs", icon, accessories: total !== void 0 ? [{ text: `${total} songs` }] : [], actions }, key);
}
function showItem(ctx, art, key, item, grid) {
  const icon = iconFor(art, grid ? item.images?.[1]?.url ?? largestImage(item.images) : largestImage(item.images), "podcast");
  const actions = [
    playAction(ctx, key, { id: item.id, type: "show" }),
    pushAction(key, "show-episodes", "Show Episodes", remember(`show:${item.id}`, item), SHORTCUTS.showContent),
    ...footerActions(ctx, key, { url: item.external_urls?.spotify, uri: item.uri, title: item.name })
  ];
  return grid ? /* @__PURE__ */ jsx(Grid.Item, { id: key, title: item.name || "Unknown", icon, actions }, key) : /* @__PURE__ */ jsx(List.Item, { id: key, title: item.name || "Unknown", icon, accessories: item.total_episodes !== void 0 ? [{ text: `${item.total_episodes} episodes` }] : [], actions }, key);
}

// src/commands.tsx
var hud = (ctx, title) => ctx.caps.hud.show({ title });
async function nowPlaying(ctx) {
  invalidate("player:");
  const playing = await currentlyPlaying(ctx.caps);
  if (!playing?.item) {
    await hud(ctx, "Nothing is currently playing");
    return null;
  }
  return playing;
}
async function changeVolume(ctx, volume) {
  try {
    await setVolume(ctx.caps, volume);
    await hud(ctx, `Volume set to ${volume}%`);
  } catch (error) {
    await hud(ctx, error instanceof WinNotSupportedError ? error.message : "No active device");
  }
}
async function stepVolume(ctx, delta) {
  const state = await playbackState(ctx.caps);
  const volume = state?.device?.volume_percent;
  if (typeof volume !== "number") {
    await hud(ctx, "No active device");
    return;
  }
  await changeVolume(ctx, Math.max(0, Math.min(100, volume + delta)));
}
var fixedVolume = (volume) => (ctx) => changeVolume(ctx, volume);
async function skip(ctx, next) {
  if (!await nowPlaying(ctx)) return;
  try {
    await (next ? skipToNext(ctx.caps) : skipToPrevious(ctx.caps));
    await hud(ctx, next ? "Skipped to next" : "Skipped to previous");
  } catch {
    await hud(ctx, "Nothing is currently playing");
  }
}
async function seekBy(ctx, seconds) {
  const playing = await nowPlaying(ctx);
  if (!playing) return;
  try {
    const position = (playing.progress_ms || 0) / 1e3;
    await seek(ctx.caps, Math.max(position + seconds, 0), playing.item?.duration_ms);
    await hud(ctx, seconds > 0 ? "Skipped ahead 15 seconds" : "Skipped back 15 seconds");
  } catch {
    await hud(ctx, "Nothing is currently playing");
  }
}
var BACKGROUND = {
  "toggle-play-pause": async (ctx) => {
    const state = await playbackState(ctx.caps);
    try {
      if (state?.is_playing) {
        await pause(ctx.caps);
        await hud(ctx, "Paused");
      } else {
        await play(ctx.caps);
        await hud(ctx, "Playing");
      }
    } catch (error) {
      await hud(ctx, errorMessage(error));
    }
  },
  next: (ctx) => skip(ctx, true),
  previous: (ctx) => skip(ctx, false),
  like: async (ctx) => {
    const playing = await nowPlaying(ctx);
    if (!playing) return;
    if (playing.currently_playing_type === "episode") return void await hud(ctx, "Liking episodes is not supported yet");
    const track = playing.item;
    const suffix = track.artists?.length ? ` by ${track.artists[0]?.name}` : "";
    if (!track.id) return void await hud(ctx, "Unable to retrieve the track ID");
    if ((await contains(ctx.caps, "tracks", [track.id]))[track.id]) return void await hud(ctx, `${track.name}${suffix} has already been liked`);
    try {
      await setSaved(ctx.caps, "tracks", [track.id], true);
      await hud(ctx, `Liked ${track.name}${suffix}`);
    } catch {
      await hud(ctx, "Nothing is currently playing");
    }
  },
  dislike: async (ctx) => {
    const playing = await nowPlaying(ctx);
    if (!playing) return;
    if (playing.currently_playing_type === "episode") return void await hud(ctx, "Liking episodes is not supported yet");
    try {
      await setSaved(ctx.caps, "tracks", playing.item?.id ? [playing.item.id] : [], false);
      await hud(ctx, `Disliked ${playing.item?.name}`);
    } catch {
      await hud(ctx, "Nothing is currently playing");
    }
  },
  volume: async (ctx, args) => {
    const volume = Number(args.volume);
    if (Number.isNaN(volume)) return void await hud(ctx, "Volume must be a number");
    if (volume < 0 || volume > 100) return void await hud(ctx, "Volume must be between 0 and 100");
    await changeVolume(ctx, volume);
  },
  "volume-0": fixedVolume(0),
  "volume-25": fixedVolume(25),
  "volume-50": fixedVolume(50),
  "volume-75": fixedVolume(75),
  "volume-100": fixedVolume(100),
  "volume-up": (ctx) => stepVolume(ctx, 10),
  "volume-down": (ctx) => stepVolume(ctx, -10),
  "toggle-shuffle": async (ctx) => {
    const state = await playbackState(ctx.caps);
    const shuffle = state?.shuffle_state;
    try {
      await setShuffle(ctx.caps, !shuffle);
      await hud(ctx, `Shuffle is ${shuffle ? "off" : "on"}`);
    } catch {
      await hud(ctx, "No active device");
    }
  },
  "cycle-repeat": async (ctx) => {
    const state = await playbackState(ctx.caps);
    const next = nextRepeatState(state?.repeat_state);
    try {
      await setRepeat(ctx.caps, next);
      await hud(ctx, `Repeat is set to ${next}`);
    } catch {
      await hud(ctx, "No active device");
    }
  },
  replay: async (ctx) => {
    try {
      await seek(ctx.caps, 0, Infinity);
      await hud(ctx, "Replaying");
    } catch (error) {
      await hud(ctx, error instanceof WinNotSupportedError ? error.message : "No active device");
    }
  },
  "current-track": async (ctx) => {
    invalidate("player:");
    const playing = await currentlyPlaying(ctx.caps);
    const item = playing?.item;
    if (!item?.id) return void await hud(ctx, "No active device");
    const liked = (await contains(ctx.caps, "tracks", [item.id]))[item.id];
    const by = item.type === "episode" ? item.show?.name ?? "unknown" : item.artists?.[0]?.name ?? "unknown";
    await hud(ctx, `${item.name} - ${by} ${liked ? "\u2665" : "\u2661"}`);
  },
  "start-radio": async (ctx) => {
    const playing = await nowPlaying(ctx);
    if (!playing) return;
    const trackId = playing.item?.id;
    if (playing.currently_playing_type === "episode" && !trackId) return void await hud(ctx, "Radio based on episodes isn't available yet");
    try {
      await startRadio(ctx.caps, { trackIds: trackId ? [trackId] : [] });
      await hud(ctx, "Playing Radio");
    } catch (error) {
      await hud(ctx, errorMessage(error));
    }
  },
  "copy-url": async (ctx) => {
    const playing = await nowPlaying(ctx);
    if (!playing?.item) return;
    const url = playing.item.external_urls?.spotify;
    await ctx.caps.clipboard.writeContent({ html: `<a href=${url}>${playing.item.name}</a>`, text: url ?? "" });
    await hud(ctx, "Copied URL to clipboard");
  },
  "copy-artist-and-title": async (ctx) => {
    const playing = await nowPlaying(ctx);
    if (!playing?.item) return;
    const artist = playing.item.artists?.[0]?.name;
    await ctx.caps.clipboard.write(`${playing.item.name} \xB7 ${artist}`);
    await hud(ctx, "Copied artist and track title to clipboard");
  },
  "copy-embed": async (ctx) => {
    const playing = await nowPlaying(ctx);
    if (!playing?.item) return;
    const code = embedCode(playing.item.external_urls?.spotify);
    if (!code) return void await hud(ctx, "Nothing is currently playing");
    await ctx.caps.clipboard.write(code);
    await hud(ctx, "Copied embed code to clipboard");
  },
  "just-play": async (ctx, args) => {
    const query = args.query ?? "";
    try {
      const { tracks } = await search(ctx.caps, query, ["track"], 1);
      const first = tracks?.[0];
      if (first?.artists) {
        await play(ctx.caps, { id: first.id, type: "track" });
        await hud(ctx, `Playing ${first.name} by ${first.artists[0]?.name}`);
      } else await hud(ctx, `No results for ${query}`);
    } catch (error) {
      await hud(ctx, errorMessage(error));
    }
  },
  "skip-15": (ctx) => seekBy(ctx, 15),
  "back-15": (ctx) => seekBy(ctx, -15),
  "remove-song-from-playlist": async (ctx) => {
    invalidate("player:");
    const playing = await currentlyPlaying(ctx.caps);
    const fail = (title, message) => ctx.caps.toast.show({ title, message, style: "failure" });
    if (!playing?.item) return void await fail("Nothing is currently playing");
    if (playing.currently_playing_type === "episode") return void await fail("Removing episodes is not supported yet");
    const uri = playing.item.uri;
    const playlistId = (playing.context?.uri ?? "").split(":")[2];
    if (!uri) return void await fail("Unable to find track");
    if (!playlistId || playing.context?.type !== "playlist") return void await fail("Unable to find the playlist");
    const remove = await ctx.caps.alert.confirm({
      title: "Remove Song",
      message: "Are you sure you want to remove the current playing song from the playlist? This will delete every instance of the song.",
      confirmTitle: "Remove",
      cancelTitle: "Cancel",
      destructive: true
    });
    if (!remove) return;
    try {
      await removeFromPlaylist(ctx.caps, playlistId, [uri]);
    } catch (error) {
      return void await fail(error instanceof Error ? error.message : "Error removing song from playlist. Please try again.");
    }
    const skipNext = await ctx.caps.alert.confirm({
      title: "Skip Song",
      message: `"${playing.item.name}" has been removed from the playlist. Do you want to skip to the next song?`,
      confirmTitle: "Skip",
      cancelTitle: "Cancel"
    });
    if (!skipNext) return;
    try {
      await skipToNext(ctx.caps);
      await ctx.caps.toast.show({ title: "Skipped to next", style: "success" });
    } catch {
      await fail("Could not skip to next song");
    }
  }
};
async function ensureSignedIn(ctx) {
  try {
    if (await authorize(ctx.caps, ctx.prefs.clientId)) return true;
    await hud(ctx, "Could not connect to Spotify");
  } catch (error) {
    await hud(ctx, errorCode(error) === "notConfigured" ? "Set your Spotify Client ID in Settings \u2192 Extensions \u2192 Spotify Player" : errorMessage(error));
  }
  return false;
}
function BackgroundCommand({ props, run }) {
  const started = useRef(false);
  useEffect(() => {
    if (started.current) return;
    started.current = true;
    const ctx = ctxOf(props);
    (async () => {
      try {
        if (await ensureSignedIn(ctx)) await run(ctx, props.arguments ?? {});
      } catch (error) {
        await hud(ctx, errorMessage(error)).catch(() => void 0);
      }
    })();
  }, []);
  return /* @__PURE__ */ jsx(List, { isLoading: true });
}

// src/views.tsx
function MessageView({ title, description, actions }) {
  return /* @__PURE__ */ jsx(Detail, { title, markdown: `# ${title}${description ? `

${description}` : ""}`, actions: actions?.length ? /* @__PURE__ */ jsx(ActionPanel, { children: actions }) : void 0 });
}
function refreshAction(key, reload) {
  return /* @__PURE__ */ jsx(Action, { id: `${key}:refresh`, title: "Refresh", shortcut: SHORTCUTS.refresh, onAction: reload }, "refresh");
}
function navigationActions(ctx, key) {
  return [
    /* @__PURE__ */ jsx(Action, { id: `${key}:open-library`, title: "Your Library", shortcut: SHORTCUTS.openLibrary, onAction: () => ctx.window.launchCommand("your-library") }, "library"),
    /* @__PURE__ */ jsx(Action, { id: `${key}:open-search`, title: "Search", shortcut: SHORTCUTS.openSearch, onAction: () => ctx.window.launchCommand("search") }, "search")
  ];
}
function ConnectView({ props, error, onConnected }) {
  const ctx = ctxOf(props);
  const notConfigured = errorCode(error) === "notConfigured";
  const description = notConfigured ? [
    "Openray needs the Client ID of your own Spotify app.",
    "",
    "1. Create an app in the Spotify Developer Dashboard.",
    "2. Add `http://127.0.0.1/callback` as its redirect address.",
    "3. Paste its Client ID in **Settings \u2192 Spotify Player**."
  ].join("\n") : error ? `Spotify sign-in failed: ${errorMessage(error)}` : "Connect your Spotify account to search, browse your library and control playback.";
  return /* @__PURE__ */ jsx(
    Detail,
    {
      title: "Connect Spotify",
      markdown: `# Connect Spotify

${description}`,
      actions: /* @__PURE__ */ jsxs(ActionPanel, { children: [
        /* @__PURE__ */ jsx(
          Action,
          {
            id: "connect",
            title: "Connect Spotify",
            primary: true,
            onAction: async () => {
              try {
                if (await authorize(ctx.caps, ctx.prefs.clientId)) onConnected();
              } catch (caught) {
                await ctx.caps.toast.show({ title: "Could not connect to Spotify", message: errorMessage(caught), style: "failure" });
              }
            }
          }
        ),
        /* @__PURE__ */ jsx(Action, { id: "dashboard", title: "Open Spotify Developer Dashboard", onAction: () => ctx.caps.shell.openUrl("https://developer.spotify.com/dashboard") })
      ] })
    }
  );
}
var SEARCH_FILTERS = [
  ["all", "All"],
  ["artists", "Artists"],
  ["tracks", "Songs"],
  ["albums", "Albums"],
  ["playlists", "Playlists"],
  ["shows", "Podcasts & Shows"],
  ["episodes", "Episodes"]
];
var MUSIC_ONLY_IGNORED = ["shows", "episodes"];
var GRID_FILTERS = ["artists", "albums", "shows"];
var RECENT_KEY = "recent-searches";
function useLikedSongsTotal(ctx, execute) {
  const me2 = useAsync(() => me(ctx.caps), [], execute);
  const liked = useAsync(() => savedTracks(ctx.caps), [], execute);
  return { userId: me2.data?.id, total: liked.data?.total };
}
function SearchView({ props }) {
  const ctx = ctxOf(props);
  const query = props.query.trim();
  const text = useDebounced(query, 250);
  const musicOnly = ctx.prefs.musicOnly === true;
  const topView = musicOnly && MUSIC_ONLY_IGNORED.includes(ctx.prefs.topView ?? "") ? "artists" : ctx.prefs.topView ?? "artists";
  const filter = props.filterValue && SEARCH_FILTERS.some(([value]) => value === props.filterValue) ? props.filterValue : "all";
  const recent = useAsync(async () => {
    const stored = await ctx.caps.storage.get(RECENT_KEY);
    return Array.isArray(stored) ? stored : [];
  }, []);
  const results = useAsync(() => search(ctx.caps, text), [text], !!text);
  const searching = results.loading || text !== query;
  const data = results.data ?? {};
  const tracks = data.tracks ?? [];
  const albums = data.albums ?? [];
  const liked = useSaved(ctx, "tracks", tracks.map((t) => t.id));
  const saved = useSaved(ctx, "albums", albums.map((a) => a.id));
  const likedSongs = useLikedSongsTotal(ctx, !!query);
  const grid = GRID_FILTERS.includes(filter);
  const art = useArtwork(ctx, [
    ...tracks.map((t) => smallestImage(t.album?.images)),
    ...albums.map((a) => grid ? a.images?.[1]?.url ?? largestImage(a.images) : largestImage(a.images)),
    ...(data.artists ?? []).map((a) => grid ? a.images?.[1]?.url ?? largestImage(a.images) : smallestImage(a.images)),
    ...(data.playlists ?? []).map((p) => smallestImage(p.images)),
    ...(data.shows ?? []).map((s) => grid ? s.images?.[1]?.url ?? largestImage(s.images) : largestImage(s.images)),
    ...(data.episodes ?? []).map((e) => smallestImage(e.images)),
    query ? LIKED_SONGS_IMAGE : void 0
  ]);
  useEffect(() => {
    const history = recent.data ?? [];
    if (query.length <= 3 || history.includes(query) || searching) return;
    const timer = setTimeout(async () => {
      await ctx.caps.storage.set(RECENT_KEY, [query, ...history]);
      recent.reload();
    }, 3e3);
    return () => clearTimeout(timer);
  }, [query, searching, recent.data]);
  if (!query) {
    const history = recent.data ?? [];
    const setHistory = async (next) => {
      await ctx.caps.storage.set(RECENT_KEY, next);
      recent.reload();
    };
    return /* @__PURE__ */ jsxs(List, { searchBarPlaceholder: "What do you want to listen to", isLoading: recent.loading, children: [
      /* @__PURE__ */ jsx(List.EmptyView, { title: "What do you want to listen to?" }),
      history.length > 0 && /* @__PURE__ */ jsx(List.Section, { title: "Recent searches", children: history.slice(0, 10).map((search2, i) => /* @__PURE__ */ jsx(
        List.Item,
        {
          id: `recent:${i}`,
          title: search2,
          icon: { lucide: "search" },
          actions: /* @__PURE__ */ jsxs(ActionPanel, { children: [
            /* @__PURE__ */ jsx(Action, { id: `recent:${i}:again`, title: "Search Again", primary: true, onAction: () => ctx.window.launchCommand("search", search2) }),
            /* @__PURE__ */ jsx(Action, { id: `recent:${i}:remove`, title: "Remove Search", style: "destructive", shortcut: SHORTCUTS.remove, onAction: () => setHistory(history.filter((item) => item !== search2)) }),
            /* @__PURE__ */ jsx(
              Action,
              {
                id: `recent:${i}:remove-all`,
                title: "Remove All Searches",
                style: "destructive",
                shortcut: SHORTCUTS.removeAll,
                onAction: async () => {
                  if (await ctx.caps.alert.confirm({ title: "Are you sure?", message: "This will remove all recent searches.", confirmTitle: "Remove", cancelTitle: "Cancel", destructive: true })) await setHistory([]);
                }
              }
            )
          ] })
        },
        `recent:${i}`
      )) })
    ] });
  }
  const options = SEARCH_FILTERS.filter(([value]) => !musicOnly || !MUSIC_ONLY_IGNORED.includes(value)).map(([value, label]) => ({ value, label }));
  const empty = results.error ? /* @__PURE__ */ jsx(List.EmptyView, { title: "Something went wrong", description: errorMessage(results.error) }) : /* @__PURE__ */ jsx(List.EmptyView, { title: "No results", description: `Nothing found for \u201C${query}\u201D` });
  if (grid) {
    const section = filter === "artists" ? /* @__PURE__ */ jsx(Grid.Section, { title: "Artists", children: (data.artists ?? []).map((a, i) => artistItem(ctx, art, `artist:${a.id}:${i}`, a, true)) }) : filter === "albums" ? /* @__PURE__ */ jsx(Grid.Section, { title: "Albums", children: albums.map((a, i) => albumItem(ctx, art, `album:${a.id}:${i}`, a, true, saved.data?.[a.id], saved.reload)) }) : /* @__PURE__ */ jsx(Grid.Section, { title: "Podcasts & Shows", children: (data.shows ?? []).map((s, i) => showItem(ctx, art, `show:${s.id}:${i}`, s, true)) });
    return /* @__PURE__ */ jsxs(Grid, { columns: 5, filter: options, searchBarPlaceholder: "What do you want to listen to", isLoading: searching, children: [
      /* @__PURE__ */ jsx(Grid.EmptyView, { title: results.error ? "Something went wrong" : "No results", description: results.error ? errorMessage(results.error) : `Nothing found for \u201C${query}\u201D` }),
      section
    ] });
  }
  const sections = {
    artists: data.artists?.length ? /* @__PURE__ */ jsx(List.Section, { title: "Artists", children: data.artists.slice(0, filter === "all" ? 3 : void 0).map((a, i) => artistItem(ctx, art, `artist:${a.id}:${i}`, a, false)) }, "artists") : null,
    tracks: tracks.length ? /* @__PURE__ */ jsx(List.Section, { title: "Songs", children: tracks.slice(0, filter === "all" ? 4 : void 0).map((t, i) => trackItem(ctx, art, `track:${t.id}:${i}`, t, { showAddToSaved: true, showGoToAlbum: true, liked: liked.data?.[t.id], onLikedChange: liked.reload })) }, "tracks") : null,
    albums: albums.length ? /* @__PURE__ */ jsx(List.Section, { title: "Albums", children: albums.slice(0, filter === "all" ? 6 : void 0).map((a, i) => albumItem(ctx, art, `album:${a.id}:${i}`, a, false, saved.data?.[a.id], saved.reload)) }, "albums") : null,
    playlists: data.playlists?.length ? /* @__PURE__ */ jsxs(List.Section, { title: "Playlists", children: [
      likedSongsItem(ctx, art, "liked-songs", likedSongs.userId, likedSongs.total, false),
      data.playlists.slice(0, filter === "all" ? 6 : void 0).map((p, i) => playlistItem(ctx, art, `playlist:${p.id}:${i}`, p, false))
    ] }, "playlists") : null,
    shows: data.shows?.length ? /* @__PURE__ */ jsx(List.Section, { title: "Podcasts & Shows", children: data.shows.slice(0, 3).map((s, i) => showItem(ctx, art, `show:${s.id}:${i}`, s, false)) }, "shows") : null,
    episodes: data.episodes?.length ? /* @__PURE__ */ jsx(List.Section, { title: "Episodes", children: data.episodes.slice(0, filter === "all" ? 3 : void 0).map((e, i) => episodeItem(ctx, art, `episode:${e.id}:${i}`, e)) }, "episodes") : null
  };
  const order = filter === "all" ? [topView, ...Object.keys(sections).filter((key) => key !== topView)].filter((key) => !musicOnly || !MUSIC_ONLY_IGNORED.includes(key)) : [filter];
  return /* @__PURE__ */ jsxs(List, { filter: options, searchBarPlaceholder: "What do you want to listen to", isLoading: searching, children: [
    empty,
    order.map((key) => sections[key])
  ] });
}
var LIBRARY_FILTERS = [
  ["all", "All"],
  ["playlists", "Playlists"],
  ["albums", "Albums"],
  ["artists", "Artists"],
  ["tracks", "Songs"],
  ["shows", "Podcasts & Shows"],
  ["episodes", "Episodes"]
];
async function loadLibrary(ctx, category) {
  const want = (name) => category === "all" || category === name;
  const [playlists, albums, artists, tracks, shows, episodes, me2] = await Promise.all([
    want("playlists") ? myPlaylists(ctx.caps) : void 0,
    want("albums") ? savedAlbums(ctx.caps) : void 0,
    want("artists") ? followedArtists(ctx.caps) : void 0,
    want("tracks") || want("playlists") ? savedTracks(ctx.caps) : void 0,
    want("shows") ? savedShows(ctx.caps) : void 0,
    want("episodes") ? savedEpisodes(ctx.caps) : void 0,
    want("playlists") ? me(ctx.caps) : void 0
  ]);
  return { playlists, albums, artists, tracks: want("tracks") ? tracks?.items : void 0, tracksTotal: tracks?.total, shows, episodes, userId: me2?.id };
}
function LibraryView({ props }) {
  const ctx = ctxOf(props);
  const query = props.query.trim();
  const fallback2 = LIBRARY_FILTERS.some(([value]) => value === ctx.prefs.libraryDefaultView) ? ctx.prefs.libraryDefaultView : "all";
  const category = props.filterValue && LIBRARY_FILTERS.some(([value]) => value === props.filterValue) ? props.filterValue : fallback2;
  const library = useAsync(() => loadLibrary(ctx, category), [category]);
  const data = library.data ?? {};
  const limit = query ? void 0 : 6;
  const visible = (items, extra) => (items ?? []).filter((item) => matchesQuery(query, item.name, extra?.(item)));
  const playlists = visible(data.playlists);
  const albums = visible(data.albums, (a) => artistNames(a.artists));
  const artists = visible(data.artists);
  const tracks = visible(data.tracks, (t) => artistNames(t.artists));
  const shows = visible(data.shows);
  const episodes = visible(data.episodes);
  const liked = useSaved(ctx, "tracks", tracks.map((t) => t.id));
  const saved = useSaved(ctx, "albums", albums.map((a) => a.id));
  const grid = GRID_FILTERS.includes(category);
  const art = useArtwork(ctx, [
    ...playlists.map((p) => smallestImage(p.images)),
    ...albums.map((a) => grid ? a.images?.[1]?.url ?? largestImage(a.images) : largestImage(a.images)),
    ...artists.map((a) => grid ? a.images?.[1]?.url ?? largestImage(a.images) : smallestImage(a.images)),
    ...tracks.map((t) => smallestImage(t.album?.images)),
    ...shows.map((s) => grid ? s.images?.[1]?.url ?? largestImage(s.images) : largestImage(s.images)),
    ...episodes.map((e) => smallestImage(e.images)),
    LIKED_SONGS_IMAGE
  ]);
  const options = LIBRARY_FILTERS.map(([value, label]) => ({ value, label }));
  const placeholder = "Search your library";
  const emptyTitle = library.error ? "Something went wrong" : "Nothing in your library";
  const emptyDescription = library.error ? errorMessage(library.error) : void 0;
  if (grid) {
    return /* @__PURE__ */ jsxs(Grid, { columns: 5, filter: options, searchBarPlaceholder: placeholder, isLoading: library.loading, children: [
      /* @__PURE__ */ jsx(Grid.EmptyView, { title: emptyTitle, description: emptyDescription }),
      category === "artists" && /* @__PURE__ */ jsx(Grid.Section, { title: "Artists", children: artists.map((a, i) => artistItem(ctx, art, `artist:${a.id}:${i}`, a, true)) }),
      category === "albums" && /* @__PURE__ */ jsx(Grid.Section, { title: "Albums", children: albums.map((a, i) => albumItem(ctx, art, `album:${a.id}:${i}`, a, true, saved.data?.[a.id], saved.reload)) }),
      category === "shows" && /* @__PURE__ */ jsx(Grid.Section, { title: "Podcasts & Shows", children: shows.map((s, i) => showItem(ctx, art, `show:${s.id}:${i}`, s, true)) })
    ] });
  }
  const all = category === "all";
  const showLiked = matchesQuery(query, "Liked Songs");
  return /* @__PURE__ */ jsxs(List, { filter: options, searchBarPlaceholder: placeholder, isLoading: library.loading, children: [
    /* @__PURE__ */ jsx(List.EmptyView, { title: emptyTitle, description: emptyDescription }),
    (all || category === "playlists") && data.playlists && (showLiked || playlists.length > 0) && /* @__PURE__ */ jsxs(List.Section, { title: "Playlists", children: [
      showLiked && likedSongsItem(ctx, art, "liked-songs", data.userId, data.tracksTotal, false),
      playlists.slice(0, all ? limit : void 0).map((p, i) => playlistItem(ctx, art, `playlist:${p.id}:${i}`, p, false))
    ] }),
    all && albums.length > 0 && /* @__PURE__ */ jsx(List.Section, { title: "Albums", children: albums.slice(0, limit).map((a, i) => albumItem(ctx, art, `album:${a.id}:${i}`, a, false, saved.data?.[a.id], saved.reload)) }),
    all && artists.length > 0 && /* @__PURE__ */ jsx(List.Section, { title: "Artists", children: artists.slice(0, limit).map((a, i) => artistItem(ctx, art, `artist:${a.id}:${i}`, a, false)) }),
    (all || category === "tracks") && tracks.length > 0 && /* @__PURE__ */ jsx(List.Section, { title: "Liked Songs", children: tracks.slice(0, all ? limit : void 0).map((t, i) => trackItem(ctx, art, `track:${t.id}:${i}`, t, { showAddToSaved: true, showGoToAlbum: true, liked: liked.data?.[t.id], onLikedChange: liked.reload })) }),
    all && shows.length > 0 && /* @__PURE__ */ jsx(List.Section, { title: "Podcasts & Shows", children: shows.slice(0, limit).map((s, i) => showItem(ctx, art, `show:${s.id}:${i}`, s, false)) }),
    (all || category === "episodes") && episodes.length > 0 && /* @__PURE__ */ jsx(List.Section, { title: "Saved Episodes", children: episodes.slice(0, all ? limit : void 0).map((e, i) => episodeItem(ctx, art, `episode:${e.id}:${i}`, e)) })
  ] });
}
function TracksView({ props, title, load, album: album2, playingContext, deps }) {
  const ctx = ctxOf(props);
  const tracks = useAsync(load, deps);
  const visible = (tracks.data ?? []).filter((t) => matchesQuery(props.query, t.name, artistNames(t.artists)));
  const art = useArtwork(ctx, visible.map((t) => smallestImage((album2 ?? t.album)?.images)));
  return /* @__PURE__ */ jsxs(List, { searchBarPlaceholder: "Search songs", isLoading: tracks.loading, children: [
    /* @__PURE__ */ jsx(List.EmptyView, { title: tracks.error ? "Something went wrong" : "No songs", description: tracks.error ? errorMessage(tracks.error) : void 0 }),
    /* @__PURE__ */ jsx(List.Section, { title, children: visible.map((t, i) => trackItem(ctx, art, `track:${t.id}:${i}`, t, { album: album2 ?? t.album, playingContext, showGoToAlbum: !album2 })) })
  ] });
}
function AlbumTracksView({ props, id }) {
  const ctx = ctxOf(props);
  const stored = recall(`album:${id}`);
  const details = useAsync(() => stored ? Promise.resolve(stored) : album(ctx.caps, id), [id]);
  const album2 = details.data;
  if (!album2) return /* @__PURE__ */ jsx(List, { isLoading: details.loading, children: details.error ? /* @__PURE__ */ jsx(List.EmptyView, { title: "Something went wrong", description: errorMessage(details.error) }) : null });
  return /* @__PURE__ */ jsx(TracksView, { props, title: album2.name, load: () => albumTracks(ctx.caps, id), album: album2, playingContext: album2.uri, deps: [id] });
}
function PlaylistTracksView({ props, id }) {
  const ctx = ctxOf(props);
  const playlist = recall(`playlist:${id}`);
  return /* @__PURE__ */ jsx(TracksView, { props, title: playlist?.name, load: () => playlistTracks(ctx.caps, id), playingContext: playlist?.uri ?? `spotify:playlist:${id}`, deps: [id] });
}
function LikedTracksView({ props }) {
  const ctx = ctxOf(props);
  return /* @__PURE__ */ jsx(TracksView, { props, title: "Liked Songs", load: async () => (await savedTracks(ctx.caps)).items, deps: [] });
}
function ArtistTopTracksView({ props, id }) {
  const ctx = ctxOf(props);
  const artist = recall(`artist-top:${id}`);
  return /* @__PURE__ */ jsx(TracksView, { props, title: artist ? `Popular \xB7 ${artist.name}` : "Popular", load: () => artistTopTracks(ctx.caps, id), deps: [id] });
}
function ArtistAlbumsView({ props, id }) {
  const ctx = ctxOf(props);
  const artist = recall(`artist-albums:${id}`);
  const albums = useAsync(() => artistAlbums(ctx.caps, id), [id]);
  const visible = [...albums.data ?? []].sort((a, b) => (b.release_date ?? "").localeCompare(a.release_date ?? "")).filter((a) => matchesQuery(props.query, a.name));
  const saved = useSaved(ctx, "albums", visible.map((a) => a.id));
  const art = useArtwork(ctx, visible.map((a) => a.images?.[1]?.url ?? largestImage(a.images)));
  return /* @__PURE__ */ jsxs(Grid, { columns: 5, searchBarPlaceholder: "Search albums", isLoading: albums.loading, children: [
    /* @__PURE__ */ jsx(Grid.EmptyView, { title: albums.error ? "Something went wrong" : "No albums", description: albums.error ? errorMessage(albums.error) : void 0 }),
    /* @__PURE__ */ jsx(Grid.Section, { title: artist?.name, children: visible.map((a, i) => albumItem(ctx, art, `album:${a.id}:${i}`, a, true, saved.data?.[a.id], saved.reload)) })
  ] });
}
function ShowEpisodesView({ props, id }) {
  const ctx = ctxOf(props);
  const stored = recall(`show:${id}`);
  const details = useAsync(() => stored ? Promise.resolve(stored) : show(ctx.caps, id), [id]);
  const episodes = useAsync(() => showEpisodes(ctx.caps, id), [id]);
  const visible = (episodes.data ?? []).filter((e) => matchesQuery(props.query, e.name));
  const art = useArtwork(ctx, visible.map((e) => smallestImage(details.data?.images) ?? smallestImage(e.images)));
  return /* @__PURE__ */ jsxs(List, { searchBarPlaceholder: "Search episodes", isLoading: episodes.loading || details.loading, children: [
    /* @__PURE__ */ jsx(List.EmptyView, { title: episodes.error ? "No Podcast Details" : "No episodes", description: episodes.error ? "Spotify did not return details for this podcast." : void 0 }),
    /* @__PURE__ */ jsx(List.Section, { title: details.data?.name, children: visible.map((e, i) => episodeItem(ctx, art, `episode:${e.id}:${i}`, e, details.data)) })
  ] });
}
function NowPlayingView({ props }) {
  const ctx = ctxOf(props);
  const current = useAsync(() => currentlyPlaying(ctx.caps), []);
  const reload = () => {
    invalidate("player:");
    current.reload();
  };
  const playing = current.data;
  const item = playing?.item ?? void 0;
  const isTrack = playing?.currently_playing_type !== "episode";
  const track = item && isTrack ? item : void 0;
  const episode = item && !isTrack ? item : void 0;
  const liked = useAsync(() => contains(ctx.caps, "tracks", [track.id]), [track?.id], !!track?.id);
  const me2 = useAsync(() => me(ctx.caps), [], !!item);
  const playlists = useAsync(() => myPlaylists(ctx.caps), [], !!item);
  const owned = (playlists.data ?? []).filter((p) => p.owner?.id === me2.data?.id);
  const ownedKey = owned.map((p) => p.id).join(",");
  const containing = useAsync(
    async () => {
      const ids = [];
      for (const playlist of owned) {
        if (ctx.signal.aborted) break;
        try {
          if (await playlistContains(ctx.caps, playlist.id, track.uri, ctx.signal)) ids.push(playlist.id);
        } catch {
          if (ctx.signal.aborted) break;
        }
      }
      return ids;
    },
    [track?.uri, ownedKey],
    !!track && owned.length > 0
  );
  const image = track ? largestImage(track.album?.images) : largestImage(episode?.images) ?? largestImage(episode?.show?.images);
  const inPlaylists = owned.filter((p) => containing.data?.includes(p.id));
  const art = useArtwork(ctx, [image, ...inPlaylists.map((p) => smallestImage(p.images))]);
  if (current.loading && current.data === void 0) return /* @__PURE__ */ jsx(List, { isLoading: true });
  if (!item) {
    return /* @__PURE__ */ jsx(
      MessageView,
      {
        title: "Nothing is playing right now",
        description: current.error ? errorMessage(current.error) : void 0,
        actions: [...navigationActions(ctx, "idle"), refreshAction("idle", reload)]
      }
    );
  }
  const key = `now:${item.id}`;
  const isPlaying = playing?.is_playing === true;
  const cover = art(image);
  const skip2 = (next) => async () => {
    try {
      await (next ? skipToNext(ctx.caps) : skipToPrevious(ctx.caps));
      const title = next ? "Skipped to next" : "Skipped to previous";
      if (ctx.prefs.closeWindowOnAction) {
        await ctx.caps.hud.show({ title });
        ctx.window.popToRoot();
        return;
      }
      await ctx.caps.toast.show({ title, style: "success" });
      reload();
    } catch (error) {
      await notifyFailure(ctx, "Something went wrong", error);
    }
  };
  const transport = [
    isPlaying ? pauseAction(ctx, key, reload) : playAction(ctx, key, {}, reload),
    ...track ? [likeAction(ctx, key, track.id, liked.data?.[track.id], liked.reload)] : [],
    /* @__PURE__ */ jsx(Action, { id: `${key}:next`, title: "Next", shortcut: SHORTCUTS.next, onAction: skip2(true) }, "next"),
    /* @__PURE__ */ jsx(Action, { id: `${key}:previous`, title: "Previous", shortcut: SHORTCUTS.previous, onAction: skip2(false) }, "previous")
  ];
  if (track) {
    const artistName = track.artists?.[0]?.name ?? "";
    const title = `${track.name} \xB7 ${artistName}`;
    const likedText = liked.error ? "Unknown" : liked.loading || !liked.data ? "Checking\u2026" : liked.data[track.id] ? "Yes" : "No";
    const markdown2 = [`# ${track.name}`, `by ${artistName}`, "", cover ? `![${track.name}](${cover})` : ""].join("\n");
    return /* @__PURE__ */ jsx(
      Detail,
      {
        title: track.name || "Now Playing",
        subtitle: artistName,
        markdown: markdown2,
        actions: /* @__PURE__ */ jsxs(ActionPanel, { children: [
          transport,
          startRadioAction(ctx, key, { trackId: track.id, artistId: track.artists?.[0]?.id }, reload),
          track.album ? pushAction(key, "go-to-album", "Go to Album", remember(`album:${track.album.id}`, track.album), SHORTCUTS.showContent) : null,
          addToPlaylistAction(key, track.uri),
          pushAction(key, "connect-device", "Connect Device", "connect-device", SHORTCUTS.connectDevice),
          footerActions(ctx, key, { url: track.external_urls?.spotify, uri: track.uri, title })
        ] }),
        children: /* @__PURE__ */ jsxs(Detail.Metadata, { children: [
          /* @__PURE__ */ jsx(Detail.Metadata.Field, { label: "Track", value: track.name || "\u2014" }),
          track.duration_ms ? /* @__PURE__ */ jsx(Detail.Metadata.Field, { label: "Duration", value: formatMs(track.duration_ms) }) : null,
          track.artists.length > 1 ? /* @__PURE__ */ jsx(Detail.Metadata.Field, { label: "Artists", value: artistNames(track.artists) }) : /* @__PURE__ */ jsx(Detail.Metadata.Field, { label: "Artist", value: artistName || "\u2014" }),
          /* @__PURE__ */ jsx(Detail.Metadata.Field, { label: "Album", value: track.album?.name || "\u2014" }),
          /* @__PURE__ */ jsx(Detail.Metadata.Field, { label: "Liked", value: likedText }),
          inPlaylists.length > 0 ? /* @__PURE__ */ jsx(Detail.Metadata.Field, { label: "In Playlists", kind: "tags", tags: inPlaylists.map((p) => p.name || "Untitled") }) : null
        ] })
      }
    );
  }
  const ep = episode;
  const showName = ep.show?.name ?? "";
  const markdown = [`# ${showName}`, ep.name, "", cover ? `![${ep.name}](${cover})` : "", "", ep.description ?? ""].join("\n");
  return /* @__PURE__ */ jsx(
    Detail,
    {
      title: ep.name || "Now Playing",
      subtitle: showName,
      markdown,
      actions: /* @__PURE__ */ jsxs(ActionPanel, { children: [
        transport,
        addToPlaylistAction(key, ep.uri),
        pushAction(key, "connect-device", "Connect Device", "connect-device", SHORTCUTS.connectDevice),
        footerActions(ctx, key, { url: ep.external_urls?.spotify, uri: ep.uri, title: `${ep.name} \xB7 ${showName}` })
      ] }),
      children: /* @__PURE__ */ jsxs(Detail.Metadata, { children: [
        /* @__PURE__ */ jsx(Detail.Metadata.Field, { label: "Title", value: ep.name || "\u2014" }),
        /* @__PURE__ */ jsx(Detail.Metadata.Field, { label: "Name", value: showName || "\u2014" }),
        /* @__PURE__ */ jsx(Detail.Metadata.Field, { label: "About", value: ep.show?.description || "\u2014" })
      ] })
    }
  );
}
function DevicesView({ props, hideRestricted = false }) {
  const ctx = ctxOf(props);
  const list = useAsync(() => devices(ctx.caps), []);
  const reload = () => {
    invalidate("player:devices");
    list.reload();
  };
  if (list.loading && list.data === void 0) return /* @__PURE__ */ jsx(List, { isLoading: true });
  if (list.error) return /* @__PURE__ */ jsx(MessageView, { title: "Unable to load devices", description: errorMessage(list.error), actions: [refreshAction("devices", reload)] });
  const devices2 = (list.data ?? []).filter((d) => !hideRestricted || !d.is_restricted);
  if (devices2.length === 0) return /* @__PURE__ */ jsx(MessageView, { title: "No devices found", description: "You don't have any devices connected to your account.", actions: [refreshAction("devices", reload)] });
  return /* @__PURE__ */ jsx(List, { isLoading: list.loading, searchBarPlaceholder: "Search devices", children: devices2.filter((d) => matchesQuery(props.query, d.name, d.type)).map((device, i) => {
    const key = `device:${device.id ?? i}`;
    return /* @__PURE__ */ jsx(
      List.Item,
      {
        id: key,
        title: device.name || "Unknown",
        subtitle: device.type,
        icon: device.is_active ? { lucide: "volume-2", color: GREEN } : { lucide: "volume-x", color: "#8E8E93" },
        accessories: device.is_active ? [{ text: "Active", color: "success" }] : [],
        actions: /* @__PURE__ */ jsxs(ActionPanel, { children: [
          !device.is_active && /* @__PURE__ */ jsx(
            Action,
            {
              id: `${key}:connect`,
              title: `Connect to ${device.name}`,
              primary: true,
              onAction: async () => {
                try {
                  if (device.id) {
                    await transferPlayback(ctx.caps, device.id);
                    setTimeout(reload, 500);
                  }
                  await notify(ctx, `Connected to ${device.name}`);
                } catch (error) {
                  await notifyFailure(ctx, "Something went wrong", error);
                }
              }
            }
          ),
          refreshAction(key, reload)
        ] })
      },
      key
    );
  }) });
}
function QueueView({ props }) {
  const ctx = ctxOf(props);
  const queue2 = useAsync(() => queue(ctx.caps), []);
  const items = (queue2.data ?? []).filter((item) => matchesQuery(props.query, item.name));
  const art = useArtwork(ctx, items.map((item) => item.type === "episode" ? smallestImage(item.show?.images) ?? smallestImage(item.images) : smallestImage(item.album?.images)));
  const reload = () => queue2.reload();
  if (queue2.loading && queue2.data === void 0) return /* @__PURE__ */ jsx(List, { isLoading: true });
  if (queue2.error) return /* @__PURE__ */ jsx(MessageView, { title: "Unable to load queue", description: errorMessage(queue2.error), actions: [refreshAction("queue", reload)] });
  if (!queue2.data?.length) return /* @__PURE__ */ jsx(MessageView, { title: "Queue", description: "Your queue is empty.", actions: [refreshAction("queue", reload)] });
  return /* @__PURE__ */ jsx(List, { isLoading: queue2.loading, searchBarPlaceholder: "Search queue", children: items.map(
    (item, i) => item.type === "episode" ? episodeItem(ctx, art, `episode:${item.id}:${i}`, item, item.show) : trackItem(ctx, art, `track:${item.id}:${i}`, item, { album: item.album })
  ) });
}
async function currentSong(ctx) {
  const playing = await currentlyPlaying(ctx.caps);
  if (!playing) throw new Error("Unable to get playback information from Spotify");
  if (!playing.item) throw new Error("Nothing is currently playing on Spotify");
  if (playing.currently_playing_type === "episode") throw new Error("Lyrics are only available for music tracks, not podcasts or episodes");
  const track = playing.item;
  const info = { title: track.name, artist: track.artists?.[0]?.name, album: track.album?.name, duration: track.duration_ms ? Math.round(track.duration_ms / 1e3) : 0 };
  if (!info.title || !info.artist || !info.album || !info.duration) throw new Error("Could not get song information from the currently playing track");
  return info;
}
async function findLyrics(ctx, song) {
  const params = new URLSearchParams({ track_name: cleanupSongTitle(song.title), artist_name: song.artist, album_name: song.album, duration: String(song.duration) });
  try {
    const response = await ctx.caps.http.fetch(`https://lrclib.net/api/get?${params}`, { headers: { Accept: "application/json", "User-Agent": "Openray Spotify Player" } });
    if (response.status === 404 || response.status >= 400) return null;
    const result = JSON.parse(response.body ?? "{}");
    if (result.instrumental) return "Instrumental";
    return normalizeLyrics(result.plainLyrics) || normalizeLyrics(removeSyncedTimestamps(result.syncedLyrics ?? ""));
  } catch {
    return null;
  }
}
function LyricsView({ props }) {
  const ctx = ctxOf(props);
  const [song, setSong] = useState(null);
  const lyrics = useAsync(async () => {
    const info = await currentSong(ctx);
    setSong(info);
    const text = await findLyrics(ctx, info);
    if (!text?.trim()) throw new Error(`Oops! Lyrics not available for "${info.title}" by ${info.artist}`);
    return text;
  }, []);
  const markdown = lyrics.error ? `# Error

${errorMessage(lyrics.error)}

## Tips:
- Try searching for a different version of the song
- Check if the song is available on LRCLIB
- Some songs may not have lyrics available` : !lyrics.data ? `# Loading lyrics for "${song?.title || "current song"}"

Please wait while we fetch the lyrics...` : `# ${song?.title}

**Artist:** ${song?.artist}

${song?.album ? `**Album:** ${song.album}

` : ""}---

${lyrics.data.split("\n").map((line) => line.trim()).filter(Boolean).join("\n\n")}`;
  const actions = song ? [
    ...lyrics.data ? [
      /* @__PURE__ */ jsx(Action, { id: "copy-lyrics", title: "Copy Lyrics", shortcut: { key: "c", modifiers: ["ctrl"] }, onAction: () => copy(ctx, { text: lyrics.data }) }, "copy-lyrics"),
      /* @__PURE__ */ jsx(Action, { id: "copy-info", title: "Copy Song Info", shortcut: { key: "c", modifiers: ["ctrl", "shift"] }, onAction: () => copy(ctx, { text: `${song.title} by ${song.artist}` }) }, "copy-info")
    ] : [],
    /* @__PURE__ */ jsx(
      Action,
      {
        id: "search-web",
        title: "Search Web for Lyrics",
        shortcut: { key: "s", modifiers: ["ctrl"] },
        onAction: () => ctx.caps.shell.openUrl(`https://www.google.com/search?q=${encodeURIComponent(`${song.title} ${song.artist} lyrics`)}`)
      },
      "web"
    )
  ] : [];
  return /* @__PURE__ */ jsx(Detail, { title: song ? `${song.title} - ${song.artist}` : "Find Lyrics", markdown, actions: actions.length ? /* @__PURE__ */ jsx(ActionPanel, { children: actions }) : void 0 });
}
function PlaylistPickerView({ props, uri }) {
  const ctx = ctxOf(props);
  const me2 = useAsync(() => me(ctx.caps), []);
  const playlists = useAsync(() => myPlaylists(ctx.caps), []);
  const owned = (playlists.data ?? []).filter((p) => p.owner?.id === me2.data?.id && p.id && p.name);
  const [membership, setMembership] = useState({});
  const [busy, setBusy] = useState(false);
  const ownedKey = owned.map((p) => p.id).join(",");
  useEffect(() => {
    let alive = true;
    setMembership({});
    (async () => {
      for (const playlist of owned) {
        if (!alive || ctx.signal.aborted) return;
        try {
          const contains2 = await playlistContains(ctx.caps, playlist.id, uri, ctx.signal);
          if (alive) setMembership((m) => ({ ...m, [playlist.id]: contains2 }));
        } catch {
          if (ctx.signal.aborted) return;
        }
      }
    })();
    return () => {
      alive = false;
    };
  }, [uri, ownedKey]);
  const visible = owned.filter((p) => matchesQuery(props.query, p.name));
  const art = useArtwork(ctx, visible.map((p) => smallestImage(p.images)));
  return /* @__PURE__ */ jsxs(List, { searchBarPlaceholder: "Search for Playlist", isLoading: me2.loading || playlists.loading || busy, children: [
    /* @__PURE__ */ jsx(List.EmptyView, { title: playlists.error ? "Something went wrong" : "No playlists", description: playlists.error ? errorMessage(playlists.error) : void 0 }),
    visible.map((playlist) => {
      const key = `pick:${playlist.id}`;
      const checked = playlist.id in membership;
      const contains2 = membership[playlist.id] === true;
      return /* @__PURE__ */ jsx(
        List.Item,
        {
          id: key,
          title: playlist.name || "Untitled",
          icon: iconFor(art, smallestImage(playlist.images), "list-music"),
          accessories: contains2 ? [{ text: "\u2713", tooltip: "Already in playlist", color: "success" }] : [],
          actions: /* @__PURE__ */ jsx(ActionPanel, { children: /* @__PURE__ */ jsx(
            Action,
            {
              id: `${key}:toggle`,
              title: !checked ? "Add or Remove from Playlist" : contains2 ? "Remove from Playlist" : "Add to Playlist",
              primary: true,
              onAction: async () => {
                if (busy) return;
                setBusy(true);
                try {
                  const exists = await playlistContains(ctx.caps, playlist.id, uri);
                  if (checked && exists !== contains2) {
                    setMembership((m) => ({ ...m, [playlist.id]: exists }));
                    await ctx.caps.toast.show({ title: "Playlist Changed", message: "The playlist was updated elsewhere. Choose the updated action to continue.", style: "failure" });
                    return;
                  }
                  if (exists) await removeFromPlaylist(ctx.caps, playlist.id, [uri]);
                  else await addToPlaylist(ctx.caps, playlist.id, [uri]);
                  const title = exists ? `Removed from ${playlist.name}` : `Added to ${playlist.name}`;
                  setMembership((m) => ({ ...m, [playlist.id]: !exists }));
                  if (ctx.prefs.closeWindowOnAction) {
                    await ctx.caps.hud.show({ title });
                    ctx.window.popToRoot();
                  } else await ctx.caps.toast.show({ title, style: "success" });
                } catch (error) {
                  await ctx.caps.toast.show({ title: "Could not update playlist", message: errorMessage(error), style: "failure" });
                } finally {
                  setBusy(false);
                }
              }
            }
          ) })
        },
        key
      );
    })
  ] });
}
function AddPlayingView({ props }) {
  const ctx = ctxOf(props);
  const current = useAsync(() => currentlyPlaying(ctx.caps), []);
  const uri = current.data?.item?.uri;
  if (current.loading && current.data === void 0) return /* @__PURE__ */ jsx(List, { isLoading: true });
  if (!uri) return /* @__PURE__ */ jsx(MessageView, { title: "Nothing is playing right now", actions: navigationActions(ctx, "add-playing") });
  return /* @__PURE__ */ jsx(PlaylistPickerView, { props, uri });
}

// src/index.tsx
function after(commandId, prefix) {
  return commandId.startsWith(prefix) ? commandId.slice(prefix.length) : null;
}
function View({ props, commandId }) {
  const ids = [
    ["album:", (id) => /* @__PURE__ */ jsx(AlbumTracksView, { props, id })],
    ["playlist:", (id) => /* @__PURE__ */ jsx(PlaylistTracksView, { props, id })],
    ["artist-albums:", (id) => /* @__PURE__ */ jsx(ArtistAlbumsView, { props, id })],
    ["artist-top:", (id) => /* @__PURE__ */ jsx(ArtistTopTracksView, { props, id })],
    ["show:", (id) => /* @__PURE__ */ jsx(ShowEpisodesView, { props, id })],
    ["add-to-playlist:", (uri) => /* @__PURE__ */ jsx(PlaylistPickerView, { props, uri })]
  ];
  for (const [prefix, render] of ids) {
    const id = after(commandId, prefix);
    if (id) return render(id);
  }
  switch (commandId) {
    case "your-library":
      return /* @__PURE__ */ jsx(LibraryView, { props });
    case "now-playing":
      return /* @__PURE__ */ jsx(NowPlayingView, { props });
    case "devices":
      return /* @__PURE__ */ jsx(DevicesView, { props });
    case "connect-device":
      return /* @__PURE__ */ jsx(DevicesView, { props, hideRestricted: true });
    case "queue":
      return /* @__PURE__ */ jsx(QueueView, { props });
    case "find-lyrics":
      return /* @__PURE__ */ jsx(LyricsView, { props });
    case "add-playing-song-to-playlist":
      return /* @__PURE__ */ jsx(AddPlayingView, { props });
    case "liked":
      return /* @__PURE__ */ jsx(LikedTracksView, { props });
    default:
      return /* @__PURE__ */ jsx(SearchView, { props });
  }
}
function SignedInView({ props, commandId }) {
  const ctx = ctxOf(props);
  const auth = useAsync(() => authorize(ctx.caps, ctx.prefs.clientId, false), []);
  if (auth.data === void 0 && auth.loading) return /* @__PURE__ */ jsx(List, { isLoading: true });
  if (!auth.data) return /* @__PURE__ */ jsx(ConnectView, { props, error: auth.error, onConnected: auth.reload });
  return /* @__PURE__ */ jsx(View, { props, commandId });
}
function SpotifyPlayer(props) {
  const commandId = props.commandId ?? "search";
  const background = BACKGROUND[commandId];
  if (background) return /* @__PURE__ */ jsx(BackgroundCommand, { props, run: background });
  return /* @__PURE__ */ jsx(SignedInView, { props, commandId });
}
var index_default = defineReactExtension({ manifest: manifest_default, component: SpotifyPlayer });
export {
  index_default as default
};
