import { Action, ActionPanel, Detail, Grid, List, type CommandProps } from "@flowkey-cli/react-ui";
import { useEffect, useState, type ReactElement } from "react";
import { artistNames, cleanupSongTitle, formatMs, largestImage, matchesQuery, normalizeLyrics, removeSyncedTimestamps, smallestImage } from "./format";
import * as sp from "./spotify";
import {
  addToPlaylistAction,
  albumItem,
  artistItem,
  copy,
  ctxOf,
  episodeItem,
  footerActions,
  GREEN,
  iconFor,
  LIKED_SONGS_IMAGE,
  likeAction,
  likedSongsItem,
  notify,
  notifyFailure,
  pauseAction,
  playAction,
  playlistItem,
  pushAction,
  recall,
  remember,
  SHORTCUTS,
  showItem,
  startRadioAction,
  trackItem,
  useArtwork,
  useAsync,
  useDebounced,
  useSaved,
  type Ctx,
} from "./ui";

type Props = { props: CommandProps };

function MessageView({ title, description, actions }: { title: string; description?: string; actions?: ReactElement[] }) {
  return <Detail title={title} markdown={`# ${title}${description ? `\n\n${description}` : ""}`} actions={actions?.length ? <ActionPanel>{actions}</ActionPanel> : undefined} />;
}

function refreshAction(key: string, reload: () => void): ReactElement {
  return <Action key="refresh" id={`${key}:refresh`} title="Refresh" shortcut={SHORTCUTS.refresh} onAction={reload} />;
}

function navigationActions(ctx: Ctx, key: string): ReactElement[] {
  return [
    <Action key="library" id={`${key}:open-library`} title="Your Library" shortcut={SHORTCUTS.openLibrary} onAction={() => ctx.window.launchCommand("your-library")} />,
    <Action key="search" id={`${key}:open-search`} title="Search" shortcut={SHORTCUTS.openSearch} onAction={() => ctx.window.launchCommand("search")} />,
  ];
}

export function ConnectView({ props, error, onConnected }: Props & { error?: unknown; onConnected: () => void }) {
  const ctx = ctxOf(props);
  const notConfigured = sp.errorCode(error) === "notConfigured";
  const description = notConfigured
    ? [
        "Openray needs the Client ID of your own Spotify app.",
        "",
        "1. Create an app in the Spotify Developer Dashboard.",
        "2. Add `http://127.0.0.1/callback` as its redirect address.",
        "3. Paste its Client ID in **Settings → Spotify Player**.",
      ].join("\n")
    : error
      ? `Spotify sign-in failed: ${sp.errorMessage(error)}`
      : "Connect your Spotify account to search, browse your library and control playback.";
  return (
    <Detail
      title="Connect Spotify"
      markdown={`# Connect Spotify\n\n${description}`}
      actions={
        <ActionPanel>
          <Action
            id="connect"
            title="Connect Spotify"
            primary
            onAction={async () => {
              try {
                if (await sp.authorize(ctx.caps, ctx.prefs.clientId)) onConnected();
              } catch (caught) {
                await ctx.caps.toast.show({ title: "Could not connect to Spotify", message: sp.errorMessage(caught), style: "failure" });
              }
            }}
          />
          <Action id="dashboard" title="Open Spotify Developer Dashboard" onAction={() => ctx.caps.shell.openUrl("https://developer.spotify.com/dashboard")} />
        </ActionPanel>
      }
    />
  );
}

const SEARCH_FILTERS: [string, string][] = [
  ["all", "All"],
  ["artists", "Artists"],
  ["tracks", "Songs"],
  ["albums", "Albums"],
  ["playlists", "Playlists"],
  ["shows", "Podcasts & Shows"],
  ["episodes", "Episodes"],
];
const MUSIC_ONLY_IGNORED = ["shows", "episodes"];
const GRID_FILTERS = ["artists", "albums", "shows"];
const RECENT_KEY = "recent-searches";

function useLikedSongsTotal(ctx: Ctx, execute: boolean) {
  const me = useAsync(() => sp.me(ctx.caps), [], execute);
  const liked = useAsync(() => sp.savedTracks(ctx.caps), [], execute);
  return { userId: me.data?.id, total: liked.data?.total };
}

export function SearchView({ props }: Props) {
  const ctx = ctxOf(props);
  const query = props.query.trim();
  const text = useDebounced(query, 250);
  const musicOnly = ctx.prefs.musicOnly === true;
  const topView = musicOnly && MUSIC_ONLY_IGNORED.includes(ctx.prefs.topView ?? "") ? "artists" : (ctx.prefs.topView ?? "artists");
  const filter = props.filterValue && SEARCH_FILTERS.some(([value]) => value === props.filterValue) ? props.filterValue : "all";
  const recent = useAsync(async () => {
    const stored = await ctx.caps.storage.get<string[]>(RECENT_KEY);
    return Array.isArray(stored) ? stored : [];
  }, []);
  const results = useAsync(() => sp.search(ctx.caps, text), [text], !!text);
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
    ...albums.map((a) => (grid ? (a.images?.[1]?.url ?? largestImage(a.images)) : largestImage(a.images))),
    ...(data.artists ?? []).map((a) => (grid ? (a.images?.[1]?.url ?? largestImage(a.images)) : smallestImage(a.images))),
    ...(data.playlists ?? []).map((p) => smallestImage(p.images)),
    ...(data.shows ?? []).map((s) => (grid ? (s.images?.[1]?.url ?? largestImage(s.images)) : largestImage(s.images))),
    ...(data.episodes ?? []).map((e) => smallestImage(e.images)),
    query ? LIKED_SONGS_IMAGE : undefined,
  ]);

  useEffect(() => {
    const history = recent.data ?? [];
    if (query.length <= 3 || history.includes(query) || searching) return;
    const timer = setTimeout(async () => {
      await ctx.caps.storage.set(RECENT_KEY, [query, ...history]);
      recent.reload();
    }, 3000);
    return () => clearTimeout(timer);
  }, [query, searching, recent.data]);

  if (!query) {
    const history = recent.data ?? [];
    const setHistory = async (next: string[]) => {
      await ctx.caps.storage.set(RECENT_KEY, next);
      recent.reload();
    };
    return (
      <List searchBarPlaceholder="What do you want to listen to" isLoading={recent.loading}>
        <List.EmptyView title="What do you want to listen to?" />
        {history.length > 0 && (
          <List.Section title="Recent searches">
            {history.slice(0, 10).map((search, i) => (
              <List.Item
                key={`recent:${i}`}
                id={`recent:${i}`}
                title={search}
                icon={{ lucide: "search" }}
                actions={
                  <ActionPanel>
                    <Action id={`recent:${i}:again`} title="Search Again" primary onAction={() => ctx.window.launchCommand("search", search)} />
                    <Action id={`recent:${i}:remove`} title="Remove Search" style="destructive" shortcut={SHORTCUTS.remove} onAction={() => setHistory(history.filter((item) => item !== search))} />
                    <Action
                      id={`recent:${i}:remove-all`}
                      title="Remove All Searches"
                      style="destructive"
                      shortcut={SHORTCUTS.removeAll}
                      onAction={async () => {
                        if (await ctx.caps.alert.confirm({ title: "Are you sure?", message: "This will remove all recent searches.", confirmTitle: "Remove", cancelTitle: "Cancel", destructive: true })) await setHistory([]);
                      }}
                    />
                  </ActionPanel>
                }
              />
            ))}
          </List.Section>
        )}
      </List>
    );
  }

  const options = SEARCH_FILTERS.filter(([value]) => !musicOnly || !MUSIC_ONLY_IGNORED.includes(value)).map(([value, label]) => ({ value, label }));
  const empty = results.error ? <List.EmptyView title="Something went wrong" description={sp.errorMessage(results.error)} /> : <List.EmptyView title="No results" description={`Nothing found for “${query}”`} />;

  if (grid) {
    const section =
      filter === "artists" ? (
        <Grid.Section title="Artists">{(data.artists ?? []).map((a, i) => artistItem(ctx, art, `artist:${a.id}:${i}`, a, true))}</Grid.Section>
      ) : filter === "albums" ? (
        <Grid.Section title="Albums">{albums.map((a, i) => albumItem(ctx, art, `album:${a.id}:${i}`, a, true, saved.data?.[a.id], saved.reload))}</Grid.Section>
      ) : (
        <Grid.Section title="Podcasts & Shows">{(data.shows ?? []).map((s, i) => showItem(ctx, art, `show:${s.id}:${i}`, s, true))}</Grid.Section>
      );
    return (
      <Grid columns={5} filter={options} searchBarPlaceholder="What do you want to listen to" isLoading={searching}>
        <Grid.EmptyView title={results.error ? "Something went wrong" : "No results"} description={results.error ? sp.errorMessage(results.error) : `Nothing found for “${query}”`} />
        {section}
      </Grid>
    );
  }

  const sections: Record<string, ReactElement | null> = {
    artists: data.artists?.length ? (
      <List.Section key="artists" title="Artists">
        {data.artists.slice(0, filter === "all" ? 3 : undefined).map((a, i) => artistItem(ctx, art, `artist:${a.id}:${i}`, a, false))}
      </List.Section>
    ) : null,
    tracks: tracks.length ? (
      <List.Section key="tracks" title="Songs">
        {tracks.slice(0, filter === "all" ? 4 : undefined).map((t, i) => trackItem(ctx, art, `track:${t.id}:${i}`, t, { showAddToSaved: true, showGoToAlbum: true, liked: liked.data?.[t.id], onLikedChange: liked.reload }))}
      </List.Section>
    ) : null,
    albums: albums.length ? (
      <List.Section key="albums" title="Albums">
        {albums.slice(0, filter === "all" ? 6 : undefined).map((a, i) => albumItem(ctx, art, `album:${a.id}:${i}`, a, false, saved.data?.[a.id], saved.reload))}
      </List.Section>
    ) : null,
    playlists: data.playlists?.length ? (
      <List.Section key="playlists" title="Playlists">
        {likedSongsItem(ctx, art, "liked-songs", likedSongs.userId, likedSongs.total, false)}
        {data.playlists.slice(0, filter === "all" ? 6 : undefined).map((p, i) => playlistItem(ctx, art, `playlist:${p.id}:${i}`, p, false))}
      </List.Section>
    ) : null,
    shows: data.shows?.length ? (
      <List.Section key="shows" title="Podcasts & Shows">
        {data.shows.slice(0, 3).map((s, i) => showItem(ctx, art, `show:${s.id}:${i}`, s, false))}
      </List.Section>
    ) : null,
    episodes: data.episodes?.length ? (
      <List.Section key="episodes" title="Episodes">
        {data.episodes.slice(0, filter === "all" ? 3 : undefined).map((e, i) => episodeItem(ctx, art, `episode:${e.id}:${i}`, e))}
      </List.Section>
    ) : null,
  };
  const order = filter === "all" ? [topView, ...Object.keys(sections).filter((key) => key !== topView)].filter((key) => !musicOnly || !MUSIC_ONLY_IGNORED.includes(key)) : [filter];
  return (
    <List filter={options} searchBarPlaceholder="What do you want to listen to" isLoading={searching}>
      {empty}
      {order.map((key) => sections[key])}
    </List>
  );
}

const LIBRARY_FILTERS: [string, string][] = [
  ["all", "All"],
  ["playlists", "Playlists"],
  ["albums", "Albums"],
  ["artists", "Artists"],
  ["tracks", "Songs"],
  ["shows", "Podcasts & Shows"],
  ["episodes", "Episodes"],
];

interface LibraryData {
  playlists?: sp.Playlist[];
  albums?: sp.Album[];
  artists?: sp.Artist[];
  tracks?: sp.Track[];
  tracksTotal?: number;
  shows?: sp.Show[];
  episodes?: sp.Episode[];
  userId?: string;
}

async function loadLibrary(ctx: Ctx, category: string): Promise<LibraryData> {
  const want = (name: string) => category === "all" || category === name;
  const [playlists, albums, artists, tracks, shows, episodes, me] = await Promise.all([
    want("playlists") ? sp.myPlaylists(ctx.caps) : undefined,
    want("albums") ? sp.savedAlbums(ctx.caps) : undefined,
    want("artists") ? sp.followedArtists(ctx.caps) : undefined,
    want("tracks") || want("playlists") ? sp.savedTracks(ctx.caps) : undefined,
    want("shows") ? sp.savedShows(ctx.caps) : undefined,
    want("episodes") ? sp.savedEpisodes(ctx.caps) : undefined,
    want("playlists") ? sp.me(ctx.caps) : undefined,
  ]);
  return { playlists, albums, artists, tracks: want("tracks") ? tracks?.items : undefined, tracksTotal: tracks?.total, shows, episodes, userId: me?.id };
}

export function LibraryView({ props }: Props) {
  const ctx = ctxOf(props);
  const query = props.query.trim();
  const fallback = LIBRARY_FILTERS.some(([value]) => value === ctx.prefs.libraryDefaultView) ? ctx.prefs.libraryDefaultView! : "all";
  const category = props.filterValue && LIBRARY_FILTERS.some(([value]) => value === props.filterValue) ? props.filterValue : fallback;
  const library = useAsync(() => loadLibrary(ctx, category), [category]);
  const data = library.data ?? {};
  const limit = query ? undefined : 6;
  const visible = <T extends { name: string }>(items: T[] | undefined, extra?: (item: T) => string | undefined) => (items ?? []).filter((item) => matchesQuery(query, item.name, extra?.(item)));
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
    ...albums.map((a) => (grid ? (a.images?.[1]?.url ?? largestImage(a.images)) : largestImage(a.images))),
    ...artists.map((a) => (grid ? (a.images?.[1]?.url ?? largestImage(a.images)) : smallestImage(a.images))),
    ...tracks.map((t) => smallestImage(t.album?.images)),
    ...shows.map((s) => (grid ? (s.images?.[1]?.url ?? largestImage(s.images)) : largestImage(s.images))),
    ...episodes.map((e) => smallestImage(e.images)),
    LIKED_SONGS_IMAGE,
  ]);
  const options = LIBRARY_FILTERS.map(([value, label]) => ({ value, label }));
  const placeholder = "Search your library";
  const emptyTitle = library.error ? "Something went wrong" : "Nothing in your library";
  const emptyDescription = library.error ? sp.errorMessage(library.error) : undefined;

  if (grid) {
    return (
      <Grid columns={5} filter={options} searchBarPlaceholder={placeholder} isLoading={library.loading}>
        <Grid.EmptyView title={emptyTitle} description={emptyDescription} />
        {category === "artists" && <Grid.Section title="Artists">{artists.map((a, i) => artistItem(ctx, art, `artist:${a.id}:${i}`, a, true))}</Grid.Section>}
        {category === "albums" && <Grid.Section title="Albums">{albums.map((a, i) => albumItem(ctx, art, `album:${a.id}:${i}`, a, true, saved.data?.[a.id], saved.reload))}</Grid.Section>}
        {category === "shows" && <Grid.Section title="Podcasts & Shows">{shows.map((s, i) => showItem(ctx, art, `show:${s.id}:${i}`, s, true))}</Grid.Section>}
      </Grid>
    );
  }

  const all = category === "all";
  const showLiked = matchesQuery(query, "Liked Songs");
  return (
    <List filter={options} searchBarPlaceholder={placeholder} isLoading={library.loading}>
      <List.EmptyView title={emptyTitle} description={emptyDescription} />
      {(all || category === "playlists") && data.playlists && (showLiked || playlists.length > 0) && (
        <List.Section title="Playlists">
          {showLiked && likedSongsItem(ctx, art, "liked-songs", data.userId, data.tracksTotal, false)}
          {playlists.slice(0, all ? limit : undefined).map((p, i) => playlistItem(ctx, art, `playlist:${p.id}:${i}`, p, false))}
        </List.Section>
      )}
      {all && albums.length > 0 && <List.Section title="Albums">{albums.slice(0, limit).map((a, i) => albumItem(ctx, art, `album:${a.id}:${i}`, a, false, saved.data?.[a.id], saved.reload))}</List.Section>}
      {all && artists.length > 0 && <List.Section title="Artists">{artists.slice(0, limit).map((a, i) => artistItem(ctx, art, `artist:${a.id}:${i}`, a, false))}</List.Section>}
      {(all || category === "tracks") && tracks.length > 0 && (
        <List.Section title="Liked Songs">
          {tracks.slice(0, all ? limit : undefined).map((t, i) => trackItem(ctx, art, `track:${t.id}:${i}`, t, { showAddToSaved: true, showGoToAlbum: true, liked: liked.data?.[t.id], onLikedChange: liked.reload }))}
        </List.Section>
      )}
      {all && shows.length > 0 && <List.Section title="Podcasts & Shows">{shows.slice(0, limit).map((s, i) => showItem(ctx, art, `show:${s.id}:${i}`, s, false))}</List.Section>}
      {(all || category === "episodes") && episodes.length > 0 && <List.Section title="Saved Episodes">{episodes.slice(0, all ? limit : undefined).map((e, i) => episodeItem(ctx, art, `episode:${e.id}:${i}`, e))}</List.Section>}
    </List>
  );
}

function TracksView({ props, title, load, album, playingContext, deps }: Props & { title?: string; load: () => Promise<sp.Track[]>; album?: sp.Album; playingContext?: string; deps: unknown[] }) {
  const ctx = ctxOf(props);
  const tracks = useAsync(load, deps);
  const visible = (tracks.data ?? []).filter((t) => matchesQuery(props.query, t.name, artistNames(t.artists)));
  const art = useArtwork(ctx, visible.map((t) => smallestImage((album ?? t.album)?.images)));
  return (
    <List searchBarPlaceholder="Search songs" isLoading={tracks.loading}>
      <List.EmptyView title={tracks.error ? "Something went wrong" : "No songs"} description={tracks.error ? sp.errorMessage(tracks.error) : undefined} />
      <List.Section title={title}>{visible.map((t, i) => trackItem(ctx, art, `track:${t.id}:${i}`, t, { album: album ?? t.album, playingContext, showGoToAlbum: !album }))}</List.Section>
    </List>
  );
}

export function AlbumTracksView({ props, id }: Props & { id: string }) {
  const ctx = ctxOf(props);
  const stored = recall<sp.Album>(`album:${id}`);
  const details = useAsync(() => (stored ? Promise.resolve(stored) : sp.album(ctx.caps, id)), [id]);
  const album = details.data;
  if (!album) return <List isLoading={details.loading}>{details.error ? <List.EmptyView title="Something went wrong" description={sp.errorMessage(details.error)} /> : null}</List>;
  return <TracksView props={props} title={album.name} load={() => sp.albumTracks(ctx.caps, id)} album={album} playingContext={album.uri} deps={[id]} />;
}

export function PlaylistTracksView({ props, id }: Props & { id: string }) {
  const ctx = ctxOf(props);
  const playlist = recall<sp.Playlist>(`playlist:${id}`);
  return <TracksView props={props} title={playlist?.name} load={() => sp.playlistTracks(ctx.caps, id)} playingContext={playlist?.uri ?? `spotify:playlist:${id}`} deps={[id]} />;
}

export function LikedTracksView({ props }: Props) {
  const ctx = ctxOf(props);
  return <TracksView props={props} title="Liked Songs" load={async () => (await sp.savedTracks(ctx.caps)).items} deps={[]} />;
}

export function ArtistTopTracksView({ props, id }: Props & { id: string }) {
  const ctx = ctxOf(props);
  const artist = recall<sp.Artist>(`artist-top:${id}`);
  return <TracksView props={props} title={artist ? `Popular · ${artist.name}` : "Popular"} load={() => sp.artistTopTracks(ctx.caps, id)} deps={[id]} />;
}

export function ArtistAlbumsView({ props, id }: Props & { id: string }) {
  const ctx = ctxOf(props);
  const artist = recall<sp.Artist>(`artist-albums:${id}`);
  const albums = useAsync(() => sp.artistAlbums(ctx.caps, id), [id]);
  const visible = [...(albums.data ?? [])].sort((a, b) => (b.release_date ?? "").localeCompare(a.release_date ?? "")).filter((a) => matchesQuery(props.query, a.name));
  const saved = useSaved(ctx, "albums", visible.map((a) => a.id));
  const art = useArtwork(ctx, visible.map((a) => a.images?.[1]?.url ?? largestImage(a.images)));
  return (
    <Grid columns={5} searchBarPlaceholder="Search albums" isLoading={albums.loading}>
      <Grid.EmptyView title={albums.error ? "Something went wrong" : "No albums"} description={albums.error ? sp.errorMessage(albums.error) : undefined} />
      <Grid.Section title={artist?.name}>{visible.map((a, i) => albumItem(ctx, art, `album:${a.id}:${i}`, a, true, saved.data?.[a.id], saved.reload))}</Grid.Section>
    </Grid>
  );
}

export function ShowEpisodesView({ props, id }: Props & { id: string }) {
  const ctx = ctxOf(props);
  const stored = recall<sp.Show>(`show:${id}`);
  const details = useAsync(() => (stored ? Promise.resolve(stored) : sp.show(ctx.caps, id)), [id]);
  const episodes = useAsync(() => sp.showEpisodes(ctx.caps, id), [id]);
  const visible = (episodes.data ?? []).filter((e) => matchesQuery(props.query, e.name));
  const art = useArtwork(ctx, visible.map((e) => smallestImage(details.data?.images) ?? smallestImage(e.images)));
  return (
    <List searchBarPlaceholder="Search episodes" isLoading={episodes.loading || details.loading}>
      <List.EmptyView title={episodes.error ? "No Podcast Details" : "No episodes"} description={episodes.error ? "Spotify did not return details for this podcast." : undefined} />
      <List.Section title={details.data?.name}>{visible.map((e, i) => episodeItem(ctx, art, `episode:${e.id}:${i}`, e, details.data))}</List.Section>
    </List>
  );
}

export function NowPlayingView({ props }: Props) {
  const ctx = ctxOf(props);
  const current = useAsync(() => sp.currentlyPlaying(ctx.caps), []);
  const reload = () => {
    sp.invalidate("player:");
    current.reload();
  };
  const playing = current.data;
  const item = playing?.item ?? undefined;
  const isTrack = playing?.currently_playing_type !== "episode";
  const track = item && isTrack ? (item as sp.Track) : undefined;
  const episode = item && !isTrack ? (item as sp.Episode) : undefined;
  const liked = useAsync(() => sp.contains(ctx.caps, "tracks", [track!.id]), [track?.id], !!track?.id);
  const me = useAsync(() => sp.me(ctx.caps), [], !!item);
  const playlists = useAsync(() => sp.myPlaylists(ctx.caps), [], !!item);
  const owned = (playlists.data ?? []).filter((p) => p.owner?.id === me.data?.id);
  const ownedKey = owned.map((p) => p.id).join(",");
  const containing = useAsync(
    async () => {
      const ids: string[] = [];
      for (const playlist of owned) {
        if (ctx.signal.aborted) break;
        try {
          if (await sp.playlistContains(ctx.caps, playlist.id, track!.uri, ctx.signal)) ids.push(playlist.id);
        } catch {
          if (ctx.signal.aborted) break;
        }
      }
      return ids;
    },
    [track?.uri, ownedKey],
    !!track && owned.length > 0,
  );
  const image = track ? largestImage(track.album?.images) : (largestImage(episode?.images) ?? largestImage(episode?.show?.images));
  const inPlaylists = owned.filter((p) => containing.data?.includes(p.id));
  const art = useArtwork(ctx, [image, ...inPlaylists.map((p) => smallestImage(p.images))]);

  if (current.loading && current.data === undefined) return <List isLoading />;
  if (!item) {
    return (
      <MessageView
        title="Nothing is playing right now"
        description={current.error ? sp.errorMessage(current.error) : undefined}
        actions={[...navigationActions(ctx, "idle"), refreshAction("idle", reload)]}
      />
    );
  }

  const key = `now:${item.id}`;
  const isPlaying = playing?.is_playing === true;
  const cover = art(image);
  const skip = (next: boolean) => async () => {
    try {
      await (next ? sp.skipToNext(ctx.caps) : sp.skipToPrevious(ctx.caps));
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
    ...(track ? [likeAction(ctx, key, track.id, liked.data?.[track.id], liked.reload)] : []),
    <Action key="next" id={`${key}:next`} title="Next" shortcut={SHORTCUTS.next} onAction={skip(true)} />,
    <Action key="previous" id={`${key}:previous`} title="Previous" shortcut={SHORTCUTS.previous} onAction={skip(false)} />,
  ];

  if (track) {
    const artistName = track.artists?.[0]?.name ?? "";
    const title = `${track.name} · ${artistName}`;
    const likedText = liked.error ? "Unknown" : liked.loading || !liked.data ? "Checking…" : liked.data[track.id] ? "Yes" : "No";
    const markdown = [`# ${track.name}`, `by ${artistName}`, "", cover ? `![${track.name}](${cover})` : ""].join("\n");
    return (
      <Detail
        title={track.name || "Now Playing"}
        subtitle={artistName}
        markdown={markdown}
        actions={
          <ActionPanel>
            {transport}
            {startRadioAction(ctx, key, { trackId: track.id, artistId: track.artists?.[0]?.id }, reload)}
            {track.album ? pushAction(key, "go-to-album", "Go to Album", remember(`album:${track.album.id}`, track.album), SHORTCUTS.showContent) : null}
            {addToPlaylistAction(key, track.uri)}
            {pushAction(key, "connect-device", "Connect Device", "connect-device", SHORTCUTS.connectDevice)}
            {footerActions(ctx, key, { url: track.external_urls?.spotify, uri: track.uri, title })}
          </ActionPanel>
        }
      >
        <Detail.Metadata>
          <Detail.Metadata.Field label="Track" value={track.name || "—"} />
          {track.duration_ms ? <Detail.Metadata.Field label="Duration" value={formatMs(track.duration_ms)} /> : null}
          {track.artists.length > 1 ? <Detail.Metadata.Field label="Artists" value={artistNames(track.artists)} /> : <Detail.Metadata.Field label="Artist" value={artistName || "—"} />}
          <Detail.Metadata.Field label="Album" value={track.album?.name || "—"} />
          <Detail.Metadata.Field label="Liked" value={likedText} />
          {inPlaylists.length > 0 ? <Detail.Metadata.Field label="In Playlists" kind="tags" tags={inPlaylists.map((p) => p.name || "Untitled")} /> : null}
        </Detail.Metadata>
      </Detail>
    );
  }

  const ep = episode!;
  const showName = ep.show?.name ?? "";
  const markdown = [`# ${showName}`, ep.name, "", cover ? `![${ep.name}](${cover})` : "", "", ep.description ?? ""].join("\n");
  return (
    <Detail
      title={ep.name || "Now Playing"}
      subtitle={showName}
      markdown={markdown}
      actions={
        <ActionPanel>
          {transport}
          {addToPlaylistAction(key, ep.uri)}
          {pushAction(key, "connect-device", "Connect Device", "connect-device", SHORTCUTS.connectDevice)}
          {footerActions(ctx, key, { url: ep.external_urls?.spotify, uri: ep.uri, title: `${ep.name} · ${showName}` })}
        </ActionPanel>
      }
    >
      <Detail.Metadata>
        <Detail.Metadata.Field label="Title" value={ep.name || "—"} />
        <Detail.Metadata.Field label="Name" value={showName || "—"} />
        <Detail.Metadata.Field label="About" value={ep.show?.description || "—"} />
      </Detail.Metadata>
    </Detail>
  );
}

export function DevicesView({ props, hideRestricted = false }: Props & { hideRestricted?: boolean }) {
  const ctx = ctxOf(props);
  const list = useAsync(() => sp.devices(ctx.caps), []);
  const reload = () => {
    sp.invalidate("player:devices");
    list.reload();
  };
  if (list.loading && list.data === undefined) return <List isLoading />;
  if (list.error) return <MessageView title="Unable to load devices" description={sp.errorMessage(list.error)} actions={[refreshAction("devices", reload)]} />;
  const devices = (list.data ?? []).filter((d) => !hideRestricted || !d.is_restricted);
  if (devices.length === 0) return <MessageView title="No devices found" description="You don't have any devices connected to your account." actions={[refreshAction("devices", reload)]} />;
  return (
    <List isLoading={list.loading} searchBarPlaceholder="Search devices">
      {devices
        .filter((d) => matchesQuery(props.query, d.name, d.type))
        .map((device, i) => {
          const key = `device:${device.id ?? i}`;
          return (
            <List.Item
              key={key}
              id={key}
              title={device.name || "Unknown"}
              subtitle={device.type}
              icon={device.is_active ? { lucide: "volume-2", color: GREEN } : { lucide: "volume-x", color: "#8E8E93" }}
              accessories={device.is_active ? [{ text: "Active", color: "success" }] : []}
              actions={
                <ActionPanel>
                  {!device.is_active && (
                    <Action
                      id={`${key}:connect`}
                      title={`Connect to ${device.name}`}
                      primary
                      onAction={async () => {
                        try {
                          if (device.id) {
                            await sp.transferPlayback(ctx.caps, device.id);
                            setTimeout(reload, 500);
                          }
                          await notify(ctx, `Connected to ${device.name}`);
                        } catch (error) {
                          await notifyFailure(ctx, "Something went wrong", error);
                        }
                      }}
                    />
                  )}
                  {refreshAction(key, reload)}
                </ActionPanel>
              }
            />
          );
        })}
    </List>
  );
}

export function QueueView({ props }: Props) {
  const ctx = ctxOf(props);
  const queue = useAsync(() => sp.queue(ctx.caps), []);
  const items = (queue.data ?? []).filter((item) => matchesQuery(props.query, item.name));
  const art = useArtwork(ctx, items.map((item) => (item.type === "episode" ? (smallestImage(item.show?.images) ?? smallestImage(item.images)) : smallestImage(item.album?.images))));
  const reload = () => queue.reload();
  if (queue.loading && queue.data === undefined) return <List isLoading />;
  if (queue.error) return <MessageView title="Unable to load queue" description={sp.errorMessage(queue.error)} actions={[refreshAction("queue", reload)]} />;
  if (!queue.data?.length) return <MessageView title="Queue" description="Your queue is empty." actions={[refreshAction("queue", reload)]} />;
  return (
    <List isLoading={queue.loading} searchBarPlaceholder="Search queue">
      {items.map((item, i) =>
        item.type === "episode" ? episodeItem(ctx, art, `episode:${item.id}:${i}`, item, item.show) : trackItem(ctx, art, `track:${item.id}:${i}`, item, { album: item.album }),
      )}
    </List>
  );
}

interface SongInfo {
  title: string;
  artist: string;
  album: string;
  duration: number;
}

async function currentSong(ctx: Ctx): Promise<SongInfo> {
  const playing = await sp.currentlyPlaying(ctx.caps);
  if (!playing) throw new Error("Unable to get playback information from Spotify");
  if (!playing.item) throw new Error("Nothing is currently playing on Spotify");
  if (playing.currently_playing_type === "episode") throw new Error("Lyrics are only available for music tracks, not podcasts or episodes");
  const track = playing.item as sp.Track;
  const info = { title: track.name, artist: track.artists?.[0]?.name, album: track.album?.name, duration: track.duration_ms ? Math.round(track.duration_ms / 1000) : 0 };
  if (!info.title || !info.artist || !info.album || !info.duration) throw new Error("Could not get song information from the currently playing track");
  return info as SongInfo;
}

export async function findLyrics(ctx: Ctx, song: SongInfo): Promise<string | null> {
  const params = new URLSearchParams({ track_name: cleanupSongTitle(song.title), artist_name: song.artist, album_name: song.album, duration: String(song.duration) });
  try {
    const response = await ctx.caps.http.fetch(`https://lrclib.net/api/get?${params}`, { headers: { Accept: "application/json", "User-Agent": "Openray Spotify Player" } });
    if (response.status === 404 || response.status >= 400) return null;
    const result = JSON.parse(response.body ?? "{}") as { instrumental?: boolean; plainLyrics?: string | null; syncedLyrics?: string | null };
    if (result.instrumental) return "Instrumental";
    return normalizeLyrics(result.plainLyrics) || normalizeLyrics(removeSyncedTimestamps(result.syncedLyrics ?? ""));
  } catch {
    return null;
  }
}

export function LyricsView({ props }: Props) {
  const ctx = ctxOf(props);
  const [song, setSong] = useState<SongInfo | null>(null);
  const lyrics = useAsync(async () => {
    const info = await currentSong(ctx);
    setSong(info);
    const text = await findLyrics(ctx, info);
    if (!text?.trim()) throw new Error(`Oops! Lyrics not available for "${info.title}" by ${info.artist}`);
    return text;
  }, []);
  const markdown = lyrics.error
    ? `# Error\n\n${sp.errorMessage(lyrics.error)}\n\n## Tips:\n- Try searching for a different version of the song\n- Check if the song is available on LRCLIB\n- Some songs may not have lyrics available`
    : !lyrics.data
      ? `# Loading lyrics for "${song?.title || "current song"}"\n\nPlease wait while we fetch the lyrics...`
      : `# ${song?.title}\n\n**Artist:** ${song?.artist}\n\n${song?.album ? `**Album:** ${song.album}\n\n` : ""}---\n\n${lyrics.data
          .split("\n")
          .map((line) => line.trim())
          .filter(Boolean)
          .join("\n\n")}`;
  const actions = song
    ? [
        ...(lyrics.data
          ? [
              <Action key="copy-lyrics" id="copy-lyrics" title="Copy Lyrics" shortcut={{ key: "c", modifiers: ["ctrl"] }} onAction={() => copy(ctx, { text: lyrics.data! })} />,
              <Action key="copy-info" id="copy-info" title="Copy Song Info" shortcut={{ key: "c", modifiers: ["ctrl", "shift"] }} onAction={() => copy(ctx, { text: `${song.title} by ${song.artist}` })} />,
            ]
          : []),
        <Action
          key="web"
          id="search-web"
          title="Search Web for Lyrics"
          shortcut={{ key: "s", modifiers: ["ctrl"] }}
          onAction={() => ctx.caps.shell.openUrl(`https://www.google.com/search?q=${encodeURIComponent(`${song.title} ${song.artist} lyrics`)}`)}
        />,
      ]
    : [];
  return <Detail title={song ? `${song.title} - ${song.artist}` : "Find Lyrics"} markdown={markdown} actions={actions.length ? <ActionPanel>{actions}</ActionPanel> : undefined} />;
}

export function PlaylistPickerView({ props, uri }: Props & { uri: string }) {
  const ctx = ctxOf(props);
  const me = useAsync(() => sp.me(ctx.caps), []);
  const playlists = useAsync(() => sp.myPlaylists(ctx.caps), []);
  const owned = (playlists.data ?? []).filter((p) => p.owner?.id === me.data?.id && p.id && p.name);
  const [membership, setMembership] = useState<Record<string, boolean>>({});
  const [busy, setBusy] = useState(false);
  const ownedKey = owned.map((p) => p.id).join(",");
  useEffect(() => {
    let alive = true;
    setMembership({});
    (async () => {
      for (const playlist of owned) {
        if (!alive || ctx.signal.aborted) return;
        try {
          const contains = await sp.playlistContains(ctx.caps, playlist.id, uri, ctx.signal);
          if (alive) setMembership((m) => ({ ...m, [playlist.id]: contains }));
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
  return (
    <List searchBarPlaceholder="Search for Playlist" isLoading={me.loading || playlists.loading || busy}>
      <List.EmptyView title={playlists.error ? "Something went wrong" : "No playlists"} description={playlists.error ? sp.errorMessage(playlists.error) : undefined} />
      {visible.map((playlist) => {
        const key = `pick:${playlist.id}`;
        const checked = playlist.id in membership;
        const contains = membership[playlist.id] === true;
        return (
          <List.Item
            key={key}
            id={key}
            title={playlist.name || "Untitled"}
            icon={iconFor(art, smallestImage(playlist.images), "list-music")}
            accessories={contains ? [{ text: "✓", tooltip: "Already in playlist", color: "success" }] : []}
            actions={
              <ActionPanel>
                <Action
                  id={`${key}:toggle`}
                  title={!checked ? "Add or Remove from Playlist" : contains ? "Remove from Playlist" : "Add to Playlist"}
                  primary
                  onAction={async () => {
                    if (busy) return;
                    setBusy(true);
                    try {
                      const exists = await sp.playlistContains(ctx.caps, playlist.id, uri);
                      if (checked && exists !== contains) {
                        setMembership((m) => ({ ...m, [playlist.id]: exists }));
                        await ctx.caps.toast.show({ title: "Playlist Changed", message: "The playlist was updated elsewhere. Choose the updated action to continue.", style: "failure" });
                        return;
                      }
                      if (exists) await sp.removeFromPlaylist(ctx.caps, playlist.id, [uri]);
                      else await sp.addToPlaylist(ctx.caps, playlist.id, [uri]);
                      const title = exists ? `Removed from ${playlist.name}` : `Added to ${playlist.name}`;
                      setMembership((m) => ({ ...m, [playlist.id]: !exists }));
                      if (ctx.prefs.closeWindowOnAction) {
                        await ctx.caps.hud.show({ title });
                        ctx.window.popToRoot();
                      } else await ctx.caps.toast.show({ title, style: "success" });
                    } catch (error) {
                      await ctx.caps.toast.show({ title: "Could not update playlist", message: sp.errorMessage(error), style: "failure" });
                    } finally {
                      setBusy(false);
                    }
                  }}
                />
              </ActionPanel>
            }
          />
        );
      })}
    </List>
  );
}

export function AddPlayingView({ props }: Props) {
  const ctx = ctxOf(props);
  const current = useAsync(() => sp.currentlyPlaying(ctx.caps), []);
  const uri = current.data?.item?.uri;
  if (current.loading && current.data === undefined) return <List isLoading />;
  if (!uri) return <MessageView title="Nothing is playing right now" actions={navigationActions(ctx, "add-playing")} />;
  return <PlaylistPickerView props={props} uri={uri} />;
}
