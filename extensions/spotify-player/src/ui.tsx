import { Action, Grid, List, type CommandProps } from "@flowkey-cli/react-ui";
import { useEffect, useRef, useState, type ReactElement } from "react";
import { artistNames, embedCode, formatMs, largestImage, linkHtml, smallestImage } from "./format";
import * as sp from "./spotify";

export const GREEN = "#1DB954";

export interface Prefs {
  clientId?: string;
  closeWindowOnAction?: boolean;
  musicOnly?: boolean;
  topView?: string;
  libraryDefaultView?: string;
}

export interface Ctx {
  caps: sp.Caps;
  window: CommandProps["window"];
  prefs: Prefs;
  signal: AbortSignal;
}

export function ctxOf(props: CommandProps): Ctx {
  return { caps: props.capabilities, window: props.window, prefs: props.preferences as Prefs, signal: props.signal };
}

export type Art = (url?: string) => string | undefined;

type Shortcut = { key: string; modifiers: string[] };
const sc = (key: string, ...modifiers: string[]): Shortcut => ({ key, modifiers });

export const SHORTCUTS = {
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
  removeAll: sc("x", "ctrl", "shift"),
};

export async function notify(ctx: Ctx, title: string, message?: string) {
  if (ctx.prefs.closeWindowOnAction) {
    await ctx.caps.hud.show({ title });
    ctx.window.popToRoot();
  } else await ctx.caps.toast.show({ title, message, style: "success" });
}

export async function notifyFailure(ctx: Ctx, title: string, error: unknown, hudTitle?: string) {
  const message = sp.errorMessage(error);
  if (ctx.prefs.closeWindowOnAction) {
    await ctx.caps.hud.show({ title: hudTitle ?? message });
    ctx.window.popToRoot();
  } else await ctx.caps.toast.show({ title, message, style: "failure" });
}

export async function copy(ctx: Ctx, content: { text: string; html?: string }) {
  await ctx.caps.clipboard.writeContent(content);
  await ctx.caps.hud.show({ title: "Copied to Clipboard" });
}

export async function openOnSpotify(ctx: Ctx, uri?: string, url?: string) {
  try {
    await ctx.caps.shell.openUrl(uri || "spotify:");
  } catch {
    await ctx.caps.shell.openUrl(url || "https://open.spotify.com");
  }
  ctx.window.closeMainWindow();
}

const stash = new Map<string, unknown>();

export function remember<T>(key: string, value: T): string {
  stash.set(key, value);
  return key;
}

export function recall<T>(key: string): T | undefined {
  return stash.get(key) as T | undefined;
}

export interface AsyncState<T> {
  data?: T;
  error?: unknown;
  loading: boolean;
  reload: () => void;
}

export function useAsync<T>(load: () => Promise<T>, deps: unknown[], execute = true): AsyncState<T> {
  const [state, setState] = useState<{ data?: T; error?: unknown; loading: boolean }>({ loading: execute });
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
      (error) => alive && setState((s) => ({ data: s.data, error, loading: false })),
    );
    return () => {
      alive = false;
    };
  }, [...deps, tick, execute]);
  return { ...state, reload: () => setTick((t) => t + 1) };
}

export function useDebounced<T>(value: T, ms: number): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const timer = setTimeout(() => setDebounced(value), ms);
    return () => clearTimeout(timer);
  }, [value, ms]);
  return debounced;
}

const artwork = new Map<string, string | null>();
const inflight = new Map<string, Promise<void>>();

export function useArtwork(ctx: Ctx, urls: (string | undefined)[]): Art {
  const [, setVersion] = useState(0);
  const wanted = [...new Set(urls.filter((u): u is string => !!u))];
  const key = wanted.join("|");
  useEffect(() => {
    let alive = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const bump = () => {
      if (!alive || timer) return;
      timer = setTimeout(() => {
        timer = undefined;
        if (alive) setVersion((v) => v + 1);
      }, 60);
    };
    const missing = wanted.filter((url) => !artwork.has(url));
    let index = 0;
    const worker = async () => {
      while (alive && index < missing.length) {
        const url = missing[index++];
        const pending =
          inflight.get(url) ??
          ctx.caps.image
            .fetch(url)
            .then((uri) => void artwork.set(url, uri || null))
            .catch(() => void artwork.set(url, null))
            .finally(() => inflight.delete(url));
        inflight.set(url, pending);
        await pending;
        bump();
      }
    };
    void Promise.all(Array.from({ length: Math.min(6, missing.length) }, worker));
    return () => {
      alive = false;
      if (timer) clearTimeout(timer);
    };
  }, [key]);
  return (url) => (url ? (artwork.get(url) ?? undefined) : undefined);
}

export function useSaved(ctx: Ctx, kind: "tracks" | "albums", ids: string[]) {
  const key = [...new Set(ids)].join(",");
  return useAsync(() => sp.contains(ctx.caps, kind, ids), [key], ids.length > 0);
}

export const iconFor = (art: Art, url: string | undefined, fallback: string) => {
  const uri = art(url);
  return uri ? { uri } : { lucide: fallback, color: GREEN };
};

export function footerActions(ctx: Ctx, key: string, { url, uri, title }: { url?: string; uri?: string; title: string }): ReactElement[] {
  const embed = embedCode(url);
  const actions = [
    <Action key="copy-url" id={`${key}:copy-url`} title="Copy URL" shortcut={SHORTCUTS.copyUrl} onAction={() => copy(ctx, { text: url ?? "", html: linkHtml(url, title) })} />,
    embed ? <Action key="copy-embed" id={`${key}:copy-embed`} title="Copy Embed Code" shortcut={SHORTCUTS.copyEmbed} onAction={() => copy(ctx, { text: embed })} /> : null,
    <Action key="copy-title" id={`${key}:copy-title`} title="Copy Artist and Title" shortcut={SHORTCUTS.copyTitle} onAction={() => copy(ctx, { text: title })} />,
    <Action key="open" id={`${key}:open`} title="Open on Spotify" shortcut={SHORTCUTS.open} onAction={() => openOnSpotify(ctx, uri, url)} />,
  ];
  return actions.filter((a): a is ReactElement => a !== null);
}

export async function runPlay(ctx: Ctx, options: Parameters<typeof sp.play>[1], onPlay?: () => void) {
  try {
    await sp.play(ctx.caps, options);
    if (ctx.prefs.closeWindowOnAction) {
      await ctx.caps.hud.show({ title: "Playing" });
      ctx.window.popToRoot();
    } else {
      await ctx.caps.toast.show({ title: "Playing", style: "success" });
      onPlay?.();
    }
  } catch (error) {
    const message = sp.errorMessage(error);
    if (ctx.prefs.closeWindowOnAction) {
      await ctx.caps.hud.show({ title: "Failed to play" });
      ctx.window.popToRoot();
    } else await ctx.caps.toast.show({ title: message.toLowerCase().includes("no active device") ? "No active device" : "Failed to play", message, style: "failure" });
  }
}

export function playAction(ctx: Ctx, key: string, options: Parameters<typeof sp.play>[1], onPlay?: () => void): ReactElement {
  return <Action key="play" id={`${key}:play`} title="Play" primary onAction={() => runPlay(ctx, options, onPlay)} />;
}

export function pauseAction(ctx: Ctx, key: string, onPause?: () => void): ReactElement {
  return (
    <Action
      key="pause"
      id={`${key}:pause`}
      title="Pause"
      primary
      onAction={async () => {
        try {
          await sp.pause(ctx.caps);
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
      }}
    />
  );
}

export function startRadioAction(ctx: Ctx, key: string, seeds: { trackId?: string; artistId?: string }, onStarted?: () => void): ReactElement {
  return (
    <Action
      key="radio"
      id={`${key}:radio`}
      title="Start Radio"
      shortcut={SHORTCUTS.startRadio}
      onAction={async () => {
        try {
          await sp.startRadio(ctx.caps, { trackIds: seeds.trackId ? [seeds.trackId] : [], artistIds: seeds.artistId ? [seeds.artistId] : [] });
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
      }}
    />
  );
}

export function likeAction(ctx: Ctx, key: string, trackId: string, liked: boolean | undefined, onChange?: () => void): ReactElement {
  const like = !liked;
  return (
    <Action
      key="like"
      id={`${key}:${like ? "like" : "dislike"}`}
      title={like ? "Like" : "Dislike"}
      shortcut={like ? SHORTCUTS.like : SHORTCUTS.dislike}
      onAction={async () => {
        try {
          await sp.setSaved(ctx.caps, "tracks", [trackId], like);
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
      }}
    />
  );
}

export function queueAction(ctx: Ctx, key: string, uri: string, title: string): ReactElement {
  return (
    <Action
      key="queue"
      id={`${key}:queue`}
      title="Add to Queue"
      shortcut={SHORTCUTS.addToQueue}
      onAction={async () => {
        try {
          await sp.addToQueue(ctx.caps, uri);
          if (ctx.prefs.closeWindowOnAction) {
            await ctx.caps.hud.show({ title: "Added to queue" });
            ctx.window.popToRoot();
            return;
          }
          await ctx.caps.toast.show({ title: "Added to queue", message: title, style: "success" });
        } catch (error) {
          await ctx.caps.toast.show({ title: "Error adding song to queue", message: sp.errorMessage(error), style: "failure" });
        }
      }}
    />
  );
}

export function addToPlaylistAction(key: string, uri: string): ReactElement {
  const target = `add-to-playlist:${uri}`;
  return <Action key="add-to-playlist" id={`${key}:add-to-playlist`} title="Add to Playlist" shortcut={SHORTCUTS.addToPlaylist} push={target} onAction={() => {}} />;
}

export function pushAction(key: string, id: string, title: string, target: string, shortcut?: Shortcut): ReactElement {
  return <Action key={id} id={`${key}:${id}`} title={title} shortcut={shortcut} push={target} onAction={() => {}} />;
}

export interface TrackOptions {
  album?: sp.Album;
  showAddToSaved?: boolean;
  showGoToAlbum?: boolean;
  playingContext?: string;
  liked?: boolean;
  onLikedChange?: () => void;
}

export function trackActions(ctx: Ctx, key: string, track: sp.Track, options: TrackOptions): ReactElement[] {
  const album = options.album ?? track.album;
  return [
    playAction(ctx, key, { id: track.id, type: "track", contextUri: options.playingContext }),
    album && options.showGoToAlbum ? pushAction(key, "go-to-album", "Go to Album", remember(`album:${album.id}`, album), SHORTCUTS.showContent) : null,
    startRadioAction(ctx, key, { trackId: track.id }),
    options.showAddToSaved && track.id ? likeAction(ctx, key, track.id, options.liked, options.onLikedChange) : null,
    track.uri ? queueAction(ctx, key, track.uri, track.name) : null,
    track.uri ? addToPlaylistAction(key, track.uri) : null,
    ...footerActions(ctx, key, { url: track.external_urls?.spotify, uri: track.uri, title: track.name }),
  ].filter((a): a is ReactElement => a !== null);
}

export function trackItem(ctx: Ctx, art: Art, key: string, track: sp.Track, options: TrackOptions) {
  const album = options.album ?? track.album;
  return (
    <List.Item
      key={key}
      id={key}
      title={track.name || "Unknown"}
      subtitle={artistNames(track.artists)}
      icon={iconFor(art, smallestImage(album?.images), "music")}
      accessories={track.duration_ms ? [{ text: formatMs(track.duration_ms) }] : []}
      actions={trackActions(ctx, key, track, options)}
    />
  );
}

export function episodeItem(ctx: Ctx, art: Art, key: string, episode: sp.Episode, showOf?: sp.Show) {
  const image = smallestImage(showOf?.images) ?? smallestImage(episode.images);
  return (
    <List.Item
      key={key}
      id={key}
      title={episode.name || "Unknown"}
      icon={iconFor(art, image, "podcast")}
      accessories={episode.duration_ms ? [{ text: formatMs(episode.duration_ms) }] : []}
      actions={[playAction(ctx, key, { id: episode.id, type: "episode" }), queueAction(ctx, key, episode.uri, episode.name), ...footerActions(ctx, key, { url: episode.external_urls?.spotify, uri: episode.uri, title: episode.name })]}
    />
  );
}

export function albumActions(ctx: Ctx, key: string, album: sp.Album, saved: boolean | undefined, onChange?: () => void): ReactElement[] {
  return [
    playAction(ctx, key, { id: album.id, type: "album" }),
    pushAction(key, "show-songs", "Show Songs", remember(`album:${album.id}`, album), SHORTCUTS.showContent),
    <Action
      key="library"
      id={`${key}:library`}
      title={saved ? "Remove from Library" : "Add to Library"}
      shortcut={{ key: saved ? "d" : "s", modifiers: ["ctrl", "shift"] }}
      onAction={async () => {
        try {
          await sp.setSaved(ctx.caps, "albums", [album.id], !saved);
          onChange?.();
          await ctx.caps.hud.show({ title: saved ? "Album removed from the library" : "Album added to the library" });
          ctx.window.popToRoot();
        } catch (error) {
          await ctx.caps.hud.show({ title: sp.errorMessage(error) });
        }
      }}
    />,
    ...footerActions(ctx, key, { url: album.external_urls?.spotify, uri: album.uri, title: album.name }),
  ];
}

export function albumItem(ctx: Ctx, art: Art, key: string, album: sp.Album, grid: boolean, saved: boolean | undefined, onChange?: () => void) {
  const subtitle = `${artistNames(album.artists)}${album.release_date ? ` · ${album.release_date.substring(0, 4)}` : ""}`;
  const icon = iconFor(art, grid ? (album.images?.[1]?.url ?? largestImage(album.images)) : largestImage(album.images), "disc-3");
  const actions = albumActions(ctx, key, album, saved, onChange);
  return grid ? (
    <Grid.Item key={key} id={key} title={album.name || "Unknown"} subtitle={subtitle} icon={icon} actions={actions} />
  ) : (
    <List.Item key={key} id={key} title={album.name || "Unknown"} subtitle={subtitle} icon={icon} accessories={album.total_tracks !== undefined ? [{ text: `${album.total_tracks} songs` }] : []} actions={actions} />
  );
}

export function artistItem(ctx: Ctx, art: Art, key: string, artist: sp.Artist, grid: boolean) {
  const icon = iconFor(art, grid ? (artist.images?.[1]?.url ?? largestImage(artist.images)) : smallestImage(artist.images), "mic-vocal");
  const actions = [
    playAction(ctx, key, { id: artist.id, type: "artist" }),
    pushAction(key, "show-albums", "Show Albums", remember(`artist-albums:${artist.id}`, artist), SHORTCUTS.showContent),
    pushAction(key, "popular", "Show Popular Songs", remember(`artist-top:${artist.id}`, artist), SHORTCUTS.quickLook),
    startRadioAction(ctx, key, { artistId: artist.id }),
    ...footerActions(ctx, key, { url: artist.external_urls?.spotify, uri: artist.uri, title: artist.name }),
  ];
  const title = artist.name || "Unknown";
  return grid ? <Grid.Item key={key} id={key} title={title} icon={icon} actions={actions} /> : <List.Item key={key} id={key} title={title} icon={icon} actions={actions} />;
}

export function playlistItem(ctx: Ctx, art: Art, key: string, playlist: sp.Playlist, grid: boolean) {
  const total = playlist.tracks?.total ?? playlist.items?.total;
  const icon = iconFor(art, grid ? largestImage(playlist.images) : smallestImage(playlist.images), "list-music");
  const actions = [
    playAction(ctx, key, { id: playlist.id, type: "playlist" }),
    pushAction(key, "show-songs", "Show Songs", remember(`playlist:${playlist.id}`, playlist), SHORTCUTS.showContent),
    ...footerActions(ctx, key, { url: playlist.external_urls?.spotify, uri: playlist.uri, title: playlist.name }),
  ];
  return grid ? (
    <Grid.Item key={key} id={key} title={playlist.name || "Untitled"} subtitle={playlist.owner?.display_name ?? undefined} icon={icon} actions={actions} />
  ) : (
    <List.Item key={key} id={key} title={playlist.name || "Untitled"} subtitle={playlist.owner?.display_name ?? undefined} icon={icon} accessories={total !== undefined ? [{ text: `${total} songs` }] : []} actions={actions} />
  );
}

export const LIKED_SONGS_IMAGE = "https://misc.scdn.co/liked-songs/liked-songs-64.png";

export function likedSongsItem(ctx: Ctx, art: Art, key: string, userId: string | undefined, total: number | undefined, grid: boolean) {
  const uri = `spotify:user:${userId}:collection`;
  const icon = iconFor(art, LIKED_SONGS_IMAGE, "heart");
  const actions = [
    ...(userId ? [playAction(ctx, key, { contextUri: uri }), pushAction(key, "show-songs", "Show Songs", "liked", SHORTCUTS.showContent)] : []),
    ...footerActions(ctx, key, { url: "https://open.spotify.com/collection/tracks", uri, title: "Liked Songs" }),
  ];
  return grid ? (
    <Grid.Item key={key} id={key} title="Liked Songs" icon={icon} actions={actions} />
  ) : (
    <List.Item key={key} id={key} title="Liked Songs" icon={icon} accessories={total !== undefined ? [{ text: `${total} songs` }] : []} actions={actions} />
  );
}

export function showItem(ctx: Ctx, art: Art, key: string, item: sp.Show, grid: boolean) {
  const icon = iconFor(art, grid ? (item.images?.[1]?.url ?? largestImage(item.images)) : largestImage(item.images), "podcast");
  const actions = [
    playAction(ctx, key, { id: item.id, type: "show" }),
    pushAction(key, "show-episodes", "Show Episodes", remember(`show:${item.id}`, item), SHORTCUTS.showContent),
    ...footerActions(ctx, key, { url: item.external_urls?.spotify, uri: item.uri, title: item.name }),
  ];
  return grid ? (
    <Grid.Item key={key} id={key} title={item.name || "Unknown"} icon={icon} actions={actions} />
  ) : (
    <List.Item key={key} id={key} title={item.name || "Unknown"} icon={icon} accessories={item.total_episodes !== undefined ? [{ text: `${item.total_episodes} episodes` }] : []} actions={actions} />
  );
}

export function imagesOf(items: { images?: sp.Image[] | null; album?: sp.Album }[], pick: (images?: sp.Image[] | null) => string | undefined = smallestImage): (string | undefined)[] {
  return items.map((item) => pick(item.album?.images ?? item.images));
}
