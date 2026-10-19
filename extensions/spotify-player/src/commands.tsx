import { List, type CommandProps } from "@flowkey-cli/react-ui";
import { useEffect, useRef } from "react";
import { embedCode } from "./format";
import * as sp from "./spotify";
import { ctxOf, type Ctx } from "./ui";

type Run = (ctx: Ctx, args: Record<string, string>) => Promise<void>;

const hud = (ctx: Ctx, title: string) => ctx.caps.hud.show({ title });

async function nowPlaying(ctx: Ctx): Promise<sp.CurrentlyPlaying | null> {
  sp.invalidate("player:");
  const playing = await sp.currentlyPlaying(ctx.caps);
  if (!playing?.item) {
    await hud(ctx, "Nothing is currently playing");
    return null;
  }
  return playing;
}

async function changeVolume(ctx: Ctx, volume: number) {
  try {
    await sp.setVolume(ctx.caps, volume);
    await hud(ctx, `Volume set to ${volume}%`);
  } catch (error) {
    await hud(ctx, error instanceof sp.WinNotSupportedError ? error.message : "No active device");
  }
}

async function stepVolume(ctx: Ctx, delta: number) {
  const state = await sp.playbackState(ctx.caps);
  const volume = state?.device?.volume_percent;
  if (typeof volume !== "number") {
    await hud(ctx, "No active device");
    return;
  }
  await changeVolume(ctx, Math.max(0, Math.min(100, volume + delta)));
}

const fixedVolume = (volume: number): Run => (ctx) => changeVolume(ctx, volume);

async function skip(ctx: Ctx, next: boolean) {
  if (!(await nowPlaying(ctx))) return;
  try {
    await (next ? sp.skipToNext(ctx.caps) : sp.skipToPrevious(ctx.caps));
    await hud(ctx, next ? "Skipped to next" : "Skipped to previous");
  } catch {
    await hud(ctx, "Nothing is currently playing");
  }
}

async function seekBy(ctx: Ctx, seconds: number) {
  const playing = await nowPlaying(ctx);
  if (!playing) return;
  try {
    const position = (playing.progress_ms || 0) / 1000;
    await sp.seek(ctx.caps, Math.max(position + seconds, 0), playing.item?.duration_ms);
    await hud(ctx, seconds > 0 ? "Skipped ahead 15 seconds" : "Skipped back 15 seconds");
  } catch {
    await hud(ctx, "Nothing is currently playing");
  }
}

export const BACKGROUND: Record<string, Run> = {
  "toggle-play-pause": async (ctx) => {
    const state = await sp.playbackState(ctx.caps);
    try {
      if (state?.is_playing) {
        await sp.pause(ctx.caps);
        await hud(ctx, "Paused");
      } else {
        await sp.play(ctx.caps);
        await hud(ctx, "Playing");
      }
    } catch (error) {
      await hud(ctx, sp.errorMessage(error));
    }
  },
  next: (ctx) => skip(ctx, true),
  previous: (ctx) => skip(ctx, false),
  like: async (ctx) => {
    const playing = await nowPlaying(ctx);
    if (!playing) return;
    if (playing.currently_playing_type === "episode") return void (await hud(ctx, "Liking episodes is not supported yet"));
    const track = playing.item as sp.Track;
    const suffix = track.artists?.length ? ` by ${track.artists[0]?.name}` : "";
    if (!track.id) return void (await hud(ctx, "Unable to retrieve the track ID"));
    if ((await sp.contains(ctx.caps, "tracks", [track.id]))[track.id]) return void (await hud(ctx, `${track.name}${suffix} has already been liked`));
    try {
      await sp.setSaved(ctx.caps, "tracks", [track.id], true);
      await hud(ctx, `Liked ${track.name}${suffix}`);
    } catch {
      await hud(ctx, "Nothing is currently playing");
    }
  },
  dislike: async (ctx) => {
    const playing = await nowPlaying(ctx);
    if (!playing) return;
    if (playing.currently_playing_type === "episode") return void (await hud(ctx, "Liking episodes is not supported yet"));
    try {
      await sp.setSaved(ctx.caps, "tracks", playing.item?.id ? [playing.item.id] : [], false);
      await hud(ctx, `Disliked ${playing.item?.name}`);
    } catch {
      await hud(ctx, "Nothing is currently playing");
    }
  },
  volume: async (ctx, args) => {
    const volume = Number(args.volume);
    if (Number.isNaN(volume)) return void (await hud(ctx, "Volume must be a number"));
    if (volume < 0 || volume > 100) return void (await hud(ctx, "Volume must be between 0 and 100"));
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
    const state = await sp.playbackState(ctx.caps);
    const shuffle = state?.shuffle_state;
    try {
      await sp.setShuffle(ctx.caps, !shuffle);
      await hud(ctx, `Shuffle is ${shuffle ? "off" : "on"}`);
    } catch {
      await hud(ctx, "No active device");
    }
  },
  "cycle-repeat": async (ctx) => {
    const state = await sp.playbackState(ctx.caps);
    const next = sp.nextRepeatState(state?.repeat_state);
    try {
      await sp.setRepeat(ctx.caps, next);
      await hud(ctx, `Repeat is set to ${next}`);
    } catch {
      await hud(ctx, "No active device");
    }
  },
  replay: async (ctx) => {
    try {
      await sp.seek(ctx.caps, 0, Infinity);
      await hud(ctx, "Replaying");
    } catch (error) {
      await hud(ctx, error instanceof sp.WinNotSupportedError ? error.message : "No active device");
    }
  },
  "current-track": async (ctx) => {
    sp.invalidate("player:");
    const playing = await sp.currentlyPlaying(ctx.caps);
    const item = playing?.item;
    if (!item?.id) return void (await hud(ctx, "No active device"));
    const liked = (await sp.contains(ctx.caps, "tracks", [item.id]))[item.id];
    const by = item.type === "episode" ? (item.show?.name ?? "unknown") : (item.artists?.[0]?.name ?? "unknown");
    await hud(ctx, `${item.name} - ${by} ${liked ? "♥" : "♡"}`);
  },
  "start-radio": async (ctx) => {
    const playing = await nowPlaying(ctx);
    if (!playing) return;
    const trackId = playing.item?.id;
    if (playing.currently_playing_type === "episode" && !trackId) return void (await hud(ctx, "Radio based on episodes isn't available yet"));
    try {
      await sp.startRadio(ctx.caps, { trackIds: trackId ? [trackId] : [] });
      await hud(ctx, "Playing Radio");
    } catch (error) {
      await hud(ctx, sp.errorMessage(error));
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
    const artist = (playing.item as sp.Track).artists?.[0]?.name;
    await ctx.caps.clipboard.write(`${playing.item.name} · ${artist}`);
    await hud(ctx, "Copied artist and track title to clipboard");
  },
  "copy-embed": async (ctx) => {
    const playing = await nowPlaying(ctx);
    if (!playing?.item) return;
    const code = embedCode(playing.item.external_urls?.spotify);
    if (!code) return void (await hud(ctx, "Nothing is currently playing"));
    await ctx.caps.clipboard.write(code);
    await hud(ctx, "Copied embed code to clipboard");
  },
  "just-play": async (ctx, args) => {
    const query = args.query ?? "";
    try {
      const { tracks } = await sp.search(ctx.caps, query, ["track"], 1);
      const first = tracks?.[0];
      if (first?.artists) {
        await sp.play(ctx.caps, { id: first.id, type: "track" });
        await hud(ctx, `Playing ${first.name} by ${first.artists[0]?.name}`);
      } else await hud(ctx, `No results for ${query}`);
    } catch (error) {
      await hud(ctx, sp.errorMessage(error));
    }
  },
  "skip-15": (ctx) => seekBy(ctx, 15),
  "back-15": (ctx) => seekBy(ctx, -15),
  "remove-song-from-playlist": async (ctx) => {
    sp.invalidate("player:");
    const playing = await sp.currentlyPlaying(ctx.caps);
    const fail = (title: string, message?: string) => ctx.caps.toast.show({ title, message, style: "failure" });
    if (!playing?.item) return void (await fail("Nothing is currently playing"));
    if (playing.currently_playing_type === "episode") return void (await fail("Removing episodes is not supported yet"));
    const uri = playing.item.uri;
    const playlistId = (playing.context?.uri ?? "").split(":")[2];
    if (!uri) return void (await fail("Unable to find track"));
    if (!playlistId || playing.context?.type !== "playlist") return void (await fail("Unable to find the playlist"));
    const remove = await ctx.caps.alert.confirm({
      title: "Remove Song",
      message: "Are you sure you want to remove the current playing song from the playlist? This will delete every instance of the song.",
      confirmTitle: "Remove",
      cancelTitle: "Cancel",
      destructive: true,
    });
    if (!remove) return;
    try {
      await sp.removeFromPlaylist(ctx.caps, playlistId, [uri]);
    } catch (error) {
      return void (await fail(error instanceof Error ? error.message : "Error removing song from playlist. Please try again."));
    }
    const skipNext = await ctx.caps.alert.confirm({
      title: "Skip Song",
      message: `"${playing.item.name}" has been removed from the playlist. Do you want to skip to the next song?`,
      confirmTitle: "Skip",
      cancelTitle: "Cancel",
    });
    if (!skipNext) return;
    try {
      await sp.skipToNext(ctx.caps);
      await ctx.caps.toast.show({ title: "Skipped to next", style: "success" });
    } catch {
      await fail("Could not skip to next song");
    }
  },
};

export async function ensureSignedIn(ctx: Ctx): Promise<boolean> {
  try {
    if (await sp.authorize(ctx.caps, ctx.prefs.clientId)) return true;
    await hud(ctx, "Could not connect to Spotify");
  } catch (error) {
    await hud(ctx, sp.errorCode(error) === "notConfigured" ? "Set your Spotify Client ID in Settings → Extensions → Spotify Player" : sp.errorMessage(error));
  }
  return false;
}

export function BackgroundCommand({ props, run }: { props: CommandProps; run: Run }) {
  const started = useRef(false);
  useEffect(() => {
    if (started.current) return;
    started.current = true;
    const ctx = ctxOf(props);
    (async () => {
      try {
        if (await ensureSignedIn(ctx)) await run(ctx, props.arguments ?? {});
      } catch (error) {
        await hud(ctx, sp.errorMessage(error)).catch(() => undefined);
      }
    })();
  }, []);
  return <List isLoading />;
}
