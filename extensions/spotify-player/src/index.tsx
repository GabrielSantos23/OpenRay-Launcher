import { defineReactExtension, List, type CommandProps } from "@flowkey-cli/react-ui";
import type { ExtensionManifest } from "@flowkey-cli/native-sdk";
import type { ReactElement } from "react";
import manifest from "../manifest.json";
import { BACKGROUND, BackgroundCommand } from "./commands";
import * as sp from "./spotify";
import { ctxOf, useAsync } from "./ui";
import {
  AddPlayingView,
  AlbumTracksView,
  ArtistAlbumsView,
  ArtistTopTracksView,
  ConnectView,
  DevicesView,
  LibraryView,
  LikedTracksView,
  LyricsView,
  NowPlayingView,
  PlaylistPickerView,
  PlaylistTracksView,
  QueueView,
  SearchView,
  ShowEpisodesView,
} from "./views";

function after(commandId: string, prefix: string): string | null {
  return commandId.startsWith(prefix) ? commandId.slice(prefix.length) : null;
}

function View({ props, commandId }: { props: CommandProps; commandId: string }) {
  const ids: [string, (id: string) => ReactElement][] = [
    ["album:", (id) => <AlbumTracksView props={props} id={id} />],
    ["playlist:", (id) => <PlaylistTracksView props={props} id={id} />],
    ["artist-albums:", (id) => <ArtistAlbumsView props={props} id={id} />],
    ["artist-top:", (id) => <ArtistTopTracksView props={props} id={id} />],
    ["show:", (id) => <ShowEpisodesView props={props} id={id} />],
    ["add-to-playlist:", (uri) => <PlaylistPickerView props={props} uri={uri} />],
  ];
  for (const [prefix, render] of ids) {
    const id = after(commandId, prefix);
    if (id) return render(id);
  }
  switch (commandId) {
    case "your-library":
      return <LibraryView props={props} />;
    case "now-playing":
      return <NowPlayingView props={props} />;
    case "devices":
      return <DevicesView props={props} />;
    case "connect-device":
      return <DevicesView props={props} hideRestricted />;
    case "queue":
      return <QueueView props={props} />;
    case "find-lyrics":
      return <LyricsView props={props} />;
    case "add-playing-song-to-playlist":
      return <AddPlayingView props={props} />;
    case "liked":
      return <LikedTracksView props={props} />;
    default:
      return <SearchView props={props} />;
  }
}

function SignedInView({ props, commandId }: { props: CommandProps; commandId: string }) {
  const ctx = ctxOf(props);
  const auth = useAsync(() => sp.authorize(ctx.caps, ctx.prefs.clientId, false), []);
  if (auth.data === undefined && auth.loading) return <List isLoading />;
  if (!auth.data) return <ConnectView props={props} error={auth.error} onConnected={auth.reload} />;
  return <View props={props} commandId={commandId} />;
}

function SpotifyPlayer(props: CommandProps) {
  const commandId = props.commandId ?? "search";
  const background = BACKGROUND[commandId];
  if (background) return <BackgroundCommand props={props} run={background} />;
  return <SignedInView props={props} commandId={commandId} />;
}

export default defineReactExtension({ manifest: manifest as unknown as ExtensionManifest, component: SpotifyPlayer });
