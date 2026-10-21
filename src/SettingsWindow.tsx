import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import type { Update } from "@tauri-apps/plugin-updater";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  Keyboard,
  Minus,
  Moon,
  Sun,
  Rocket,
  RotateCcw,
  Search,
  Settings as Gear,
  Command,
  Puzzle,
  X,
  LayoutGrid,
  PanelTop,
} from "lucide-react";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { Extension, RegisteredCommand } from "./extension-api";
import { builtInExtensions } from "./extensions";
import { flowKeyFailures, flowKeyInstalled, loadFlowKeyExtensions } from "./extensions/flowkey";
import { FlowKeyDialog } from "./flowkey/Dialog";
import { FlowKeyIcon } from "./flowkey/FlowKeyIcon";
import { FlowKeyDetails, FlowKeyInstallCard } from "./flowkey/SettingsPanel";
import type { Failure } from "./flowkey/extensions";
import { loadCommands } from "./registry";
import { Dropdown as Select } from "./Dropdown";
import { HotkeyRecorder, type HotkeyConflict } from "./HotkeyRecorder";
import {
  commandPrefs,
  DEFAULT_HOTKEY,
  START_MENU_HOTKEY,
  hotkeyOwner,
  LAUNCHER_OWNER,
  useSettings,
  withCommandPrefs,
  withHotkey,
  type Settings as SettingsData,
} from "./settings";
import { Card, Checkbox, Row, Switch } from "./controls";
import { CommandIcon, GlyphTile, OpenrayGlyph } from "./icons";
import { THEMES, type Theme } from "./theme";
import { checkForUpdate } from "./updater";
import { DEFAULT_PREFERENCES, loadPreferences, savePreferences, SNOOZE_OPTIONS, type FocusPreferences } from "./focus";

type Save = (next: SettingsData) => Promise<void>;
type RecorderFor = (owner: string) => {
  findConflict: (hotkey: string) => HotkeyConflict | null;
  onSave: (hotkey: string, overwrite: boolean) => Promise<void>;
};

function ThemeIcon({ theme }: { theme: Theme }) {
  const Glyph = theme.appearance === "dark" ? Moon : Sun;
  return (
    <span
      aria-hidden="true"
      style={{ backgroundImage: `linear-gradient(to bottom, ${theme.background}, ${theme.backgroundSecondary})`, color: theme.foreground }}
      className="relative flex size-4 shrink-0 items-center justify-center rounded-full shadow-[inset_0_0_0_0.5px_var(--color-fg-40)]"
    >
      <Glyph className={theme.appearance === "dark" ? "size-2.5" : "size-3"} />
    </span>
  );
}

function Section({ id, title, children }: { id: string; title: string; children: ReactNode }) {
  return (
    <div role="group" aria-labelledby={id} className="flex flex-col gap-1">
      <h2 id={id} className="flex min-h-8 items-center pl-3 font-medium">
        {title}
      </h2>
      {children}
    </div>
  );
}

interface Page {
  id: string;
  title: string;
  icon: ReactNode;
  keywords: string[];
}

function Tile({ children, className = "bg-control" }: { children: ReactNode; className?: string }) {
  return (
    <span aria-hidden="true" className={`flex size-[22px] shrink-0 items-center justify-center rounded-[5px] text-fg ${className}`}>
      {children}
    </span>
  );
}

const PAGES: Page[] = [
  { id: "general", title: "General", icon: <Tile><Gear className="size-3.5" /></Tile>, keywords: ["open at login", "system tray", "hotkey", "updates", "check for updates", "version"] },
  { id: "launcher", title: "Launcher", icon: <Tile><Rocket className="size-3.5" /></Tile>, keywords: ["show openray on", "screen"] },
  { id: "shortcuts", title: "Shortcuts", icon: <Tile><Command className="size-3.5" /></Tile>, keywords: ["alias", "hotkey", "commands"] },
  { id: "notch", title: "Notch", icon: <Tile><PanelTop className="size-3.5" /></Tile>, keywords: ["notch", "now playing", "fullscreen", "widgets", "volume"] },
  { id: "keyboard", title: "Keyboard", icon: <Tile><Keyboard className="size-3.5" /></Tile>, keywords: ["escape key"] },
  { id: "extensions", title: "Extensions", icon: <Tile><Puzzle className="size-3.5" /></Tile>, keywords: ["install", "flowkey", "package", "uninstall"] },
];

function extensionIcon(extension: Extension) {
  const flowkey = flowKeyInstalled.get(extension.id);
  if (flowkey) return <FlowKeyIcon fields={{ icon: flowkey.manifest.icon }} extension={flowkey} title={flowkey.manifest.name} />;
  return extension.icon ? (
    <GlyphTile spec={extension.icon} />
  ) : (
    <Tile>
      <OpenrayGlyph className="size-3.5" />
    </Tile>
  );
}

const TEXT_BUTTON = "h-6 shrink-0 rounded-[5px] bg-control px-2.5 font-medium text-fg outline-offset-2 hover:bg-control-hover disabled:text-fg-40";

function UpdatesRow() {
  const [version, setVersion] = useState("");
  const [status, setStatus] = useState<{ state: "idle" | "checking" | "current" | "installing" } | { state: "available"; update: Update } | { state: "failed"; error: string }>({ state: "idle" });

  useEffect(() => {
    getVersion().then(setVersion);
  }, []);

  async function runCheck() {
    setStatus({ state: "checking" });
    try {
      const update = await checkForUpdate();
      setStatus(update ? { state: "available", update } : { state: "current" });
    } catch (e) {
      setStatus({ state: "failed", error: String(e) });
    }
  }

  async function install(update: Update) {
    setStatus({ state: "installing" });
    try {
      await update.downloadAndInstall();
    } catch (e) {
      setStatus({ state: "failed", error: String(e) });
    }
  }

  const description =
    status.state === "checking"
      ? "Checking for updates..."
      : status.state === "current"
        ? `Openray ${version} is up to date`
        : status.state === "available"
          ? `Openray ${status.update.version} is available. You have ${version}.`
          : status.state === "installing"
            ? "Downloading and installing. Openray restarts when it finishes."
            : status.state === "failed"
              ? `Could not check for updates: ${status.error}`
              : version && `Version ${version}`;

  return (
    <Row id="updates" title="Updates" description={description || undefined}>
      <span aria-live="polite" className="sr-only">
        {status.state === "idle" ? "" : description}
      </span>
      {status.state === "available" ? (
        <button type="button" onClick={() => install(status.update)} className={TEXT_BUTTON}>
          Install Update
        </button>
      ) : (
        <button type="button" disabled={status.state === "checking" || status.state === "installing"} onClick={runCheck} className={TEXT_BUTTON}>
          {status.state === "failed" ? "Try Again" : "Check for Updates"}
        </button>
      )}
    </Row>
  );
}

function FocusSettings() {
  const [prefs, setPrefs] = useState<FocusPreferences | null>(null);
  useEffect(() => {
    loadPreferences()
      .then(setPrefs)
      .catch(() => setPrefs(DEFAULT_PREFERENCES));
  }, []);
  if (!prefs) return null;
  const update = (patch: Partial<FocusPreferences>) => {
    const next = { ...prefs, ...patch };
    setPrefs(next);
    savePreferences(next).catch((e) => console.error("focus preferences save failed", e));
  };
  return (
    <>
      <Card>
        <Row id="focus-snooze" title="Snooze Duration" description="Choose how long blocked apps and websites stay snoozed.">
          <Select labelledBy="focus-snooze" value={String(prefs.snoozeMs ?? 0)} options={SNOOZE_OPTIONS} onChange={(value) => update({ snoozeMs: Number(value) || null })} />
        </Row>
        <Row id="focus-sound" title="Play Sound" description="Play sounds for Focus notifications.">
          <Switch labelledBy="focus-sound" checked={prefs.playSound} onChange={(playSound) => update({ playSound })} />
        </Row>
      </Card>
      <Card>
        <Row id="focus-allow-all" title="Allow all Websites" description="Allow all websites when a browser is allowed but no specific websites are listed.">
          <Switch labelledBy="focus-allow-all" checked={prefs.allowAllWebsites} onChange={(allowAllWebsites) => update({ allowAllWebsites })} />
        </Row>
      </Card>
      <Card>
        <Row id="focus-restore" title="Restore Terminated Apps" description="Reopen apps Openray Focus closed when the session ends.">
          <Switch labelledBy="focus-restore" checked={prefs.restoreApps} onChange={(restoreApps) => update({ restoreApps })} />
        </Row>
      </Card>
    </>
  );
}

interface ScreenshotFolder {
  path: string;
  defaultPath: string;
  custom: boolean;
}

function ScreenshotSettings() {
  const [folder, setFolder] = useState<ScreenshotFolder | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    invoke<ScreenshotFolder>("screenshots_folder").then(setFolder);
  }, []);
  if (!folder) return null;
  const apply = (path: string | null) =>
    invoke<ScreenshotFolder>("screenshots_set_folder", { path })
      .then((next) => {
        setFolder(next);
        setError(null);
      })
      .catch((e) => setError(String(e)));
  const choose = async () => {
    const [path] = await invoke<string[]>("flowkey_pick_path", { directory: true, multiple: false });
    if (path) await apply(path);
  };
  return (
    <Card>
      <Row id="screenshots-folder" title="Save Screenshots To" description={error ?? `${folder.path}${folder.custom ? "" : " (default)"}`}>
        <button type="button" onClick={() => invoke("launch_app", { path: folder.path }).catch((e) => setError(String(e)))} className={TEXT_BUTTON}>
          Open
        </button>
        {folder.custom && (
          <button type="button" onClick={() => apply(null)} className={TEXT_BUTTON}>
            Use Default
          </button>
        )}
        <button type="button" aria-describedby="screenshots-folder" onClick={choose} className={TEXT_BUTTON}>
          Change…
        </button>
      </Row>
    </Card>
  );
}

function GeneralPage({ settings, save, recorder }: { settings: SettingsData; save: Save; recorder: RecorderFor }) {
  const [openAtLogin, setOpenAtLogin] = useState<boolean | null>(null);
  const [loginError, setLoginError] = useState<string | null>(null);

  useEffect(() => {
    invoke<boolean>("login_item_get").then(setOpenAtLogin);
  }, []);

  async function toggleLogin(enabled: boolean) {
    setOpenAtLogin(enabled);
    setLoginError(null);
    try {
      await invoke("login_item_set", { enabled });
    } catch (e) {
      setOpenAtLogin(!enabled);
      setLoginError(String(e));
    }
  }

  const themeOptions = (appearance: Theme["appearance"]) =>
    THEMES.filter((t) => t.appearance === appearance).map((t) => ({ value: t.id, label: t.name, icon: <ThemeIcon theme={t} /> }));
  const keepAppearance = (forced: Theme["appearance"]) => (settings.appearance === "system" ? "system" : forced);

  return (
    <>
      <Card>
      <Row id="open-at-login" title="Open at Login" description={loginError ?? undefined}>
        <Switch labelledBy="open-at-login" checked={!!openAtLogin} onChange={toggleLogin} />
      </Row>
      <Row id="show-in-tray" title="Show in System Tray">
        <Switch labelledBy="show-in-tray" checked={settings.showInTray} onChange={(showInTray) => save({ ...settings, showInTray })} />
      </Row>
      <Row id="openray-hotkey" title="Openray Hotkey">
        {settings.hotkey === START_MENU_HOTKEY ? null : (
          <>
            <button type="button" onClick={() => save({ ...settings, hotkey: START_MENU_HOTKEY })} className={`${TEXT_BUTTON} flex items-center gap-1.5`}>
              Replace Start Menu
              <LayoutGrid aria-hidden="true" className="size-3" />
            </button>
            <HotkeyRecorder variant="button" label="Openray Hotkey" value={settings.hotkey} {...recorder(LAUNCHER_OWNER)} />
          </>
        )}
        <button
          type="button"
          aria-label="Reset hotkey to Alt+Space"
          title="Reset to Alt+Space"
          disabled={settings.hotkey === DEFAULT_HOTKEY}
          onClick={() => save({ ...settings, hotkey: DEFAULT_HOTKEY })}
          className="flex size-6 items-center justify-center rounded-[5px] bg-control text-fg outline-offset-2 hover:bg-control-hover disabled:text-fg-40"
        >
          <RotateCcw className="size-3.5" />
        </button>
      </Row>
      <UpdatesRow />
      </Card>
      <Section id="appearance" title="Appearance">
        <Card>
          <Row id="follow-system" title="Follow System Appearance">
            <Switch
              labelledBy="follow-system"
              checked={settings.appearance === "system"}
              onChange={(on) =>
                save({ ...settings, appearance: on ? "system" : window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light" })
              }
            />
          </Row>
          <Row id="dark-theme" title="Dark Theme">
            <Select
              labelledBy="dark-theme"
              value={settings.themeDarkId}
              options={themeOptions("dark")}
              onChange={(themeDarkId) => save({ ...settings, themeDarkId, appearance: keepAppearance("dark") })}
            />
          </Row>
          <Row id="light-theme" title="Light Theme">
            <Select
              labelledBy="light-theme"
              value={settings.themeLightId}
              options={themeOptions("light")}
              onChange={(themeLightId) => save({ ...settings, themeLightId, appearance: keepAppearance("light") })}
            />
          </Row>
        </Card>
      </Section>
    </>
  );
}

function LauncherPage({ settings, save }: { settings: SettingsData; save: Save }) {
  return (
    <Card>
      <Row id="show-on" title="Show Openray on">
        <Select
          labelledBy="show-on"
          value={settings.showOn}
          onChange={(showOn) => save({ ...settings, showOn })}
          options={[
            { value: "mouse-screen", label: "Screen containing mouse" },
            { value: "active-screen", label: "Screen with active window" },
            { value: "primary-screen", label: "Primary screen" },
          ]}
        />
      </Row>
    </Card>
  );
}

function NotchPage({ settings, save }: { settings: SettingsData; save: Save }) {
  return (
    <Card>
      <Row id="notch-enabled" title="Enable Notch" description="Show a notch at the top of the primary screen. Hover it for media, calendar, clipboard and capture shortcuts.">
        <Switch labelledBy="notch-enabled" checked={settings.notchEnabled} onChange={(notchEnabled) => save({ ...settings, notchEnabled })} />
      </Row>
      <Row id="notch-fullscreen" title="Show in Fullscreen" description="Keep the notch visible over fullscreen apps and games.">
        <Switch labelledBy="notch-fullscreen" checked={settings.notchShowInFullscreen} onChange={(notchShowInFullscreen) => save({ ...settings, notchShowInFullscreen })} />
      </Row>
    </Card>
  );
}

function KeyboardPage({ settings, save }: { settings: SettingsData; save: Save }) {
  return (
    <Card>
      <Row
        id="escape-behavior"
        title="Escape Key Behavior"
        description="Preferred behavior for the launcher when the Escape key is pressed. The default clears the search, then closes the window."
      >
        <Select
          labelledBy="escape-behavior"
          value={settings.escapeKeyBehavior}
          onChange={(escapeKeyBehavior) => save({ ...settings, escapeKeyBehavior })}
          options={[
            { value: "pop-back-or-close", label: "Navigate back or close window" },
            { value: "close-and-pop-to-root", label: "Close window and pop to root" },
          ]}
        />
      </Row>
      <Row id="escape-closes" title="Escape Key Closes Window" description="Dismiss windows such as Settings by pressing the Escape key.">
        <Switch
          labelledBy="escape-closes"
          checked={settings.escapeKeyClosesWindow}
          onChange={(escapeKeyClosesWindow) => save({ ...settings, escapeKeyClosesWindow })}
        />
      </Row>
    </Card>
  );
}

function ExtensionTable({
  extension,
  commands,
  settings,
  save,
  recorder,
}: {
  extension: Extension;
  commands: RegisteredCommand[];
  settings: SettingsData;
  save: Save;
  recorder: RecorderFor;
}) {
  const [collapsed, setCollapsed] = useState(false);
  const extensionEnabled = !commandPrefs(settings, extension.id).disabled;
  return (
    <Card>
      <div className="flex h-[46px] items-center gap-2 px-3">
        {extensionIcon(extension)}
        <button
          type="button"
          aria-expanded={!collapsed}
          onClick={() => setCollapsed((c) => !c)}
          className="flex items-center gap-1 rounded font-medium outline-offset-2"
        >
          {extension.title}
          <ChevronDown aria-hidden="true" className={`size-3 text-fg-60 transition-transform ${collapsed ? "-rotate-90" : ""}`} />
        </button>
        <span className="ml-auto">
          <Checkbox label={`Enable ${extension.title}`} checked={extensionEnabled} onChange={(on) => save(withCommandPrefs(settings, extension.id, { disabled: !on }))} />
        </span>
      </div>
      {!collapsed && commands.length > 0 && (
        <table className={`w-full table-fixed border-collapse ${extensionEnabled ? "" : "opacity-50"}`}>
          <thead>
            <tr className="h-6 border-b border-card-separator text-left text-12 text-fg-60">
              <th className="pl-3 font-semibold">Name</th>
              <th className="w-[108px] font-semibold">Alias</th>
              <th className="w-[133px] font-semibold">Hotkey</th>
              <th className="w-10">
                <span className="sr-only">Enabled</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {commands.map((command) => {
              const prefs = commandPrefs(settings, command.key);
              return (
                <tr key={command.key} className="h-[46px] border-b border-card-separator last:border-b-0">
                  <td className="pl-3">
                    <span className="flex min-w-0 items-center gap-2">
                      <CommandIcon title={command.title} icon={command.icon} fallback={extension.icon} small />
                      <span className="truncate font-medium">{command.title}</span>
                    </span>
                  </td>
                  <td>
                    <input
                      defaultValue={prefs.alias}
                      placeholder="Add Alias"
                      aria-label={`Alias for ${command.title}`}
                      spellCheck={false}
                      onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
                      onBlur={(e) => {
                        const alias = e.target.value.trim();
                        if (alias !== prefs.alias) save(withCommandPrefs(settings, command.key, { alias })).catch(() => (e.target.value = prefs.alias));
                      }}
                      className="-mx-1.5 h-6 w-full rounded-[5px] bg-transparent px-1.5 font-medium text-fg outline-offset-2 placeholder:text-fg-40 hover:bg-control focus:bg-control"
                    />
                  </td>
                  <td>
                    <HotkeyRecorder
                      variant="cell"
                      label={`Hotkey for ${command.title}`}
                      value={prefs.hotkey}
                      onClear={() => save(withCommandPrefs(settings, command.key, { hotkey: "" }))}
                      {...recorder(command.key)}
                    />
                  </td>
                  <td className="pr-3">
                    <span className="flex justify-end">
                      <Checkbox
                        label={`Enable ${command.title}`}
                        checked={!prefs.disabled}
                        onChange={(on) => save(withCommandPrefs(settings, command.key, { disabled: !on }))}
                      />
                    </span>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
    </Card>
  );
}

function TitleBarButton({ label, onClick, children, danger = false }: { label: string; onClick: () => void; children: ReactNode; danger?: boolean }) {
  return (
    <button
      type="button"
      aria-label={label}
      tabIndex={-1}
      onClick={onClick}
      className={`flex h-full w-[46px] items-center justify-center text-fg ${danger ? "hover:bg-close-hover" : "hover:bg-control"}`}
    >
      {children}
    </button>
  );
}

export function SettingsWindow() {
  const [settings, save] = useSettings();
  const [commands, setCommands] = useState<RegisteredCommand[]>([]);
  const [flowkey, setFlowkey] = useState<Extension[]>([]);
  const [failures, setFailures] = useState<Failure[]>([]);
  const [history, setHistory] = useState({ stack: ["general"], index: 0 });
  const [navFilter, setNavFilter] = useState("");
  const [extensionFilter, setExtensionFilter] = useState("all");
  const [commandFilter, setCommandFilter] = useState("");
  const [searchingCommands, setSearchingCommands] = useState(false);
  const escapeCloses = useRef(true);
  escapeCloses.current = settings?.escapeKeyClosesWindow ?? true;
  const page = history.stack[history.index];

  const reloadExtensions = () =>
    loadFlowKeyExtensions().then((loaded) => {
      setFlowkey(loaded);
      setFailures(flowKeyFailures);
      return loadCommands([...builtInExtensions, ...loaded]).then(setCommands);
    });

  useEffect(() => {
    reloadExtensions();
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !event.defaultPrevented && escapeCloses.current) getCurrentWindow().close();
    };
    const onFocus = () => void reloadExtensions();
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("focus", onFocus);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("focus", onFocus);
    };
  }, []);

  function navigate(id: string) {
    if (id === page) return;
    setHistory(({ stack, index }) => {
      const next = [...stack.slice(0, index + 1), id];
      return { stack: next, index: next.length - 1 };
    });
  }

  const q = navFilter.trim().toLowerCase();
  const matches = (title: string, keywords: string[] = []) => !q || [title, ...keywords].some((t) => t.toLowerCase().includes(q));
  const allExtensions = [...builtInExtensions, ...flowkey];
  const extensionPage = allExtensions.find((e) => `extension:${e.id}` === page);
  const flowkeyPage = extensionPage && flowKeyInstalled.get(extensionPage.id);
  const tableExtensions = extensionPage ? [extensionPage] : allExtensions.filter((e) => extensionFilter === "all" || e.id === extensionFilter);
  const filteredCommands = useMemo(() => {
    const term = commandFilter.trim().toLowerCase();
    return term ? commands.filter((c) => c.title.toLowerCase().includes(term)) : commands;
  }, [commands, commandFilter]);
  const recorder: RecorderFor = (owner) => ({
    findConflict: (hotkey) => {
      const found = settings && hotkeyOwner(settings, hotkey, owner);
      if (!found) return null;
      if (found === LAUNCHER_OWNER) return { name: "Openray", overwritable: false };
      return { name: commands.find((c) => c.key === found)?.title ?? found, overwritable: true };
    },
    onSave: (hotkey, overwrite) => (settings ? save(withHotkey(settings, owner, hotkey, overwrite)) : Promise.resolve()),
  });
  const title = PAGES.find((p) => p.id === page)?.title ?? extensionPage?.title ?? "";

  const navLink = (id: string, label: string, icon: ReactNode) => (
    <button
      key={id}
      type="button"
      aria-current={page === id ? "page" : undefined}
      onClick={() => navigate(id)}
      className={`flex h-8 items-center gap-2 rounded-md px-1.5 text-left font-medium outline-offset-[-2px] ${page === id ? "bg-nav-selection" : "hover:bg-list-hover"}`}
    >
      {icon}
      <span className="truncate">{label}</span>
    </button>
  );

  return (
    <div className="flex h-full flex-col text-13 text-fg">
      <header data-tauri-drag-region className="relative flex h-10 shrink-0 items-stretch bg-titlebar after:absolute after:inset-x-0 after:bottom-px after:h-px after:bg-titlebar-line">
        <span data-tauri-drag-region className="flex w-[230px] shrink-0 items-center pl-4 font-['Segoe_UI_Variable','Segoe_UI',sans-serif] text-fg-60">
          Settings
        </span>
        <span className="flex items-center gap-1 pl-4">
          <button
            type="button"
            aria-label="Back"
            disabled={history.index === 0}
            onClick={() => setHistory((h) => ({ ...h, index: h.index - 1 }))}
            className="rounded p-0.5 text-fg-60 hover:bg-control disabled:text-fg-40 disabled:hover:bg-transparent"
          >
            <ChevronLeft className="size-4" />
          </button>
          <button
            type="button"
            aria-label="Forward"
            disabled={history.index === history.stack.length - 1}
            onClick={() => setHistory((h) => ({ ...h, index: h.index + 1 }))}
            className="rounded p-0.5 text-fg-60 hover:bg-control disabled:text-fg-40 disabled:hover:bg-transparent"
          >
            <ChevronRight className="size-4" />
          </button>
        </span>
        <span data-tauri-drag-region className="flex-1" />
        {page === "shortcuts" && (
          <span className="flex items-center gap-2 pr-3">
            {searchingCommands ? (
              <input
                autoFocus
                value={commandFilter}
                onChange={(e) => setCommandFilter(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key !== "Escape") return;
                  e.preventDefault();
                  setCommandFilter("");
                  setSearchingCommands(false);
                }}
                placeholder="Search commands..."
                aria-label="Search commands"
                className="h-6 w-48 rounded-[5px] border border-control-hover bg-transparent px-2 text-fg outline-offset-2 placeholder:text-fg-40"
              />
            ) : (
              <button
                type="button"
                aria-label="Search commands"
                onClick={() => setSearchingCommands(true)}
                className="flex size-6 items-center justify-center rounded-[5px] border border-control-hover text-fg hover:bg-control"
              >
                <Search className="size-3.5" />
              </button>
            )}
            <span className="flex items-center">
              <span id="extension-filter" className="sr-only">
                Extension
              </span>
              <Select
                labelledBy="extension-filter"
                size="small"
                value={extensionFilter}
                onChange={setExtensionFilter}
                options={[{ value: "all", label: "All Extensions" }, ...allExtensions.map((e) => ({ value: e.id, label: e.title }))]}
              />
            </span>
          </span>
        )}
        <TitleBarButton label="Minimize" onClick={() => getCurrentWindow().minimize()}>
          <Minus className="size-4" strokeWidth={1.5} />
        </TitleBarButton>
        <TitleBarButton label="Close" danger onClick={() => getCurrentWindow().close()}>
          <X className="size-4" strokeWidth={1.5} />
        </TitleBarButton>
      </header>

      <div className="flex min-h-0 flex-1">
        <nav aria-label="Settings" className="flex w-[230px] shrink-0 flex-col gap-4 overflow-y-auto border-r border-sidebar-border px-[9px] pt-4 pb-2.5">
          <label className="relative flex items-center">
            <Search aria-hidden="true" className="pointer-events-none absolute left-2.5 size-4 text-fg-60" />
            <input
              value={navFilter}
              onChange={(e) => setNavFilter(e.target.value)}
              placeholder="Search settings..."
              aria-label="Search settings"
              spellCheck={false}
              className="h-8 w-full rounded-md bg-field pr-2 pl-8 text-fg outline-offset-[-2px] placeholder:text-fg-60"
            />
          </label>
          <div className="flex flex-col gap-0.5">{PAGES.filter((p) => matches(p.title, p.keywords)).map((p) => navLink(p.id, p.title, p.icon))}</div>
          <div className="flex flex-col gap-0.5">
            {allExtensions.filter((e) => matches(e.title)).map((e) => navLink(`extension:${e.id}`, e.title, extensionIcon(e)))}
          </div>
        </nav>

        <main className="min-w-0 flex-1 overflow-y-auto p-4">
          <h1 className="sr-only">{title}</h1>
          {settings && (
            <div className="flex flex-col gap-4">
              {page === "general" && <GeneralPage settings={settings} save={save} recorder={recorder} />}
              {page === "launcher" && <LauncherPage settings={settings} save={save} />}
              {page === "notch" && <NotchPage settings={settings} save={save} />}
              {page === "keyboard" && <KeyboardPage settings={settings} save={save} />}
              {page === "extensions" && <FlowKeyInstallCard failures={failures} onChanged={reloadExtensions} />}
              {flowkeyPage && (
                <FlowKeyDetails
                  key={flowkeyPage.manifest.id}
                  extension={flowkeyPage}
                  onChanged={() =>
                    reloadExtensions().then(() => {
                      if (!flowKeyInstalled.has(extensionPage!.id)) setHistory({ stack: ["extensions"], index: 0 });
                    })
                  }
                />
              )}
              {extensionPage?.id === "focus" && <FocusSettings />}
              {extensionPage?.id === "screenshots" && <ScreenshotSettings />}
              {(page === "shortcuts" || extensionPage) &&
                tableExtensions.map((extension) => (
                  <ExtensionTable
                    key={extension.id}
                    extension={extension}
                    commands={filteredCommands.filter((c) => c.extensionId === extension.id)}
                    settings={settings}
                    save={save}
                    recorder={recorder}
                  />
                ))}
            </div>
          )}
        </main>
      </div>
      <FlowKeyDialog />
    </div>
  );
}
