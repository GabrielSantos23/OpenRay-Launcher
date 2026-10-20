import { invoke } from "@tauri-apps/api/core";
import { Clipboard, FileCode2, Folder, FolderPlus, Play, Plus, RefreshCw, ScrollText, SquarePen } from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useState } from "react";
import { fail, Footer, Loader, useActionHost } from "../ActionHost";
import { matchesShortcut } from "../actions";
import { Dropdown } from "../Dropdown";
import { defineExtension, type Action, type Command, type IconSpec, type Item, type ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { BackButton, ListView } from "../ListView";
import { copyAndClose } from "../output";
import { isEmojiIcon, lastLine, MODES, scriptFileName, scriptTemplate, TEMPLATES, type Script, type ScriptMode, type ScriptOutput, type ScriptTemplate } from "../scriptCommands";
import { Checkbox } from "../controls";
import { showHUD, showToast } from "../toast";

const ICON: IconSpec = { glyph: ScrollText, tone: "slate" };

async function loadDirectories(): Promise<string[]> {
  const value = await invoke<{ directories?: string[] } | null>("data_get", { name: "script-commands" });
  return Array.isArray(value?.directories) ? value.directories : [];
}

async function addDirectory(dir: string) {
  const dirs = await loadDirectories();
  if (!dirs.some((d) => d.toLowerCase() === dir.toLowerCase())) await invoke("data_set", { name: "script-commands", value: { directories: [...dirs, dir] } });
}

async function loadScripts(): Promise<Script[]> {
  return invoke<Script[]>("scripts_list", { directories: await loadDirectories() });
}

async function pickDirectory(): Promise<string | null> {
  const [folder] = await invoke<string[]>("flowkey_pick_path", { directory: true, multiple: false });
  return folder ?? null;
}

function failure(script: Script, output: ScriptOutput) {
  return new Error(lastLine(output.stderr) || lastLine(output.stdout) || `${script.title} exited with code ${output.code}`);
}

async function runQuietly(script: Script, args: string[]) {
  const output = await invoke<ScriptOutput>("script_run", { path: script.path, arguments: args });
  if (output.code !== 0) throw failure(script, output);
  const line = lastLine(output.stdout);
  if (script.mode === "silent") {
    await invoke("hide_launcher", { restoreFocus: true });
    showHUD({ variant: "success", title: line || `Ran ${script.title}` });
  } else showToast({ variant: "success", title: line || `Ran ${script.title}` });
}

const confirmation = (script: Script) => (script.needsConfirmation ? { title: `Run ${script.title}?`, message: "This script asks for confirmation before it runs.", confirmTitle: "Run Script" } : undefined);

function ScriptIcon({ script }: { script: Script }) {
  if (isEmojiIcon(script.icon)) return <span className="flex size-[22px] shrink-0 items-center justify-center text-[16px]">{script.icon}</span>;
  return <span className="flex size-[22px] shrink-0 items-center justify-center text-fg-60"><FileCode2 aria-hidden="true" className="size-4" /></span>;
}

function OutputView({ script, args, onBack }: { script: Script; args: string[]; onBack: () => void }) {
  const [output, setOutput] = useState<ScriptOutput | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [round, setRound] = useState(0);
  const host = useActionHost(() => {});

  useEffect(() => {
    let live = true;
    setOutput(null);
    setError(null);
    invoke<ScriptOutput>("script_run", { path: script.path, arguments: args })
      .then((result) => live && setOutput(result))
      .catch((e) => live && setError(String(e)));
    return () => {
      live = false;
    };
  }, [script.path, args, round]);

  const text = error ?? (output ? [output.stdout.trimEnd(), output.stderr.trimEnd()].filter(Boolean).join("\n") : "");
  const actions: Action[] = [
    { id: "copy", title: "Copy Output", icon: Clipboard, shortcut: { key: "c", ctrl: true, shift: true }, run: () => copyAndClose(text, "Copied Output") },
    { id: "again", title: "Run Again", icon: RefreshCw, shortcut: { key: "r", ctrl: true }, keepOpen: true, run: () => setRound((r) => r + 1) },
  ];

  return (
    <div
      tabIndex={-1}
      autoFocus
      ref={(el) => el?.focus()}
      className="flex min-h-0 flex-1 flex-col outline-none"
      onKeyDown={(event) => {
        const action = actions.find((a) => a.shortcut && matchesShortcut(event, a.shortcut));
        if (action) {
          event.preventDefault();
          host.run(action, { failTitle: `${action.title} failed` });
        } else if (matchesShortcut(event, { key: "k", ctrl: true })) {
          event.preventDefault();
          host.openPanel(script.title, [actions]);
        } else if (event.key === "Escape" && !event.defaultPrevented) {
          event.preventDefault();
          onBack();
        }
      }}
    >
      <header className="relative flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <BackButton onBack={onBack} />
        <h2 className="truncate text-13 font-medium text-fg">{script.title}</h2>
        <Loader active={!output && !error} />
      </header>
      <pre className="min-h-0 flex-1 overflow-auto p-4 font-mono text-13 break-words whitespace-pre-wrap text-fg">
        {output && output.code !== 0 && <span className="text-red">{`Exited with code ${output.code}\n`}</span>}
        {output || error ? text || "No output" : ""}
      </pre>
      <Footer
        left={
          <span className="flex items-center gap-2 text-fg-60">
            <GlyphTile spec={ICON} small />
            {script.packageName || "Script Commands"}
          </span>
        }
        primaryTitle="Copy Output"
        primaryKeys={["Ctrl", "Shift", "C"]}
        showActions
        panelOpen={host.panelOpen}
        onTogglePanel={() => (host.panelOpen ? host.closePanel() : host.openPanel(script.title, [actions]))}
      />
      {host.render((action) => host.run(action, { failTitle: `${action.title} failed` }))}
    </div>
  );
}

function SearchScripts({ pop, selectPath }: ViewProps & { selectPath?: string }) {
  const [scripts, setScripts] = useState<Script[] | null>(null);
  const [current, setCurrent] = useState<Item | undefined>();
  const [values, setValues] = useState<Record<string, string>>({});
  const [output, setOutput] = useState<{ script: Script; args: string[] } | null>(null);
  const [creating, setCreating] = useState(false);

  const reload = useCallback(() => {
    loadScripts()
      .then(setScripts)
      .catch((e) => {
        setScripts([]);
        fail("Could not load Script Commands", e);
      });
  }, []);
  useEffect(reload, [reload]);

  const selected = scripts?.find((s) => s.path === current?.id);
  const firstArgument = selected?.arguments[0];

  const addFolder: Action = {
    id: "add-directory",
    title: "Add Script Directory",
    icon: FolderPlus,
    keepOpen: true,
    run: async () => {
      const dir = await pickDirectory();
      if (!dir) return;
      await addDirectory(dir);
      showToast({ variant: "success", title: "Added Script Directory" });
      reload();
    },
  };
  const create: Action = { id: "create", title: "Create Script Command", icon: Plus, shortcut: { key: "n", ctrl: true }, keepOpen: true, run: () => setCreating(true) };

  const items = useMemo<Item[]>(
    () =>
      (scripts ?? []).map((script) => {
        const args = values[script.path]?.trim() ? [values[script.path].trim()] : [];
        const missing = script.arguments[0] && !script.arguments[0].optional && args.length === 0;
        const run: Action = {
          id: "run",
          title: "Run Script",
          icon: Play,
          keepOpen: script.mode !== "silent",
          confirm: confirmation(script),
          run: () => {
            if (missing) throw new Error(`Enter a value for ${script.arguments[0].placeholder ?? "the argument"}`);
            if (script.mode === "fullOutput") setOutput({ script, args });
            else return runQuietly(script, args);
          },
        };
        return {
          id: script.path,
          title: script.title,
          subtitle: script.packageName || undefined,
          keywords: ["script", script.packageName, script.description, script.path.split(/[\\/]/).pop() ?? ""],
          icon: <ScriptIcon script={script} />,
          accessory: <span className="text-13 text-fg-60">{MODES.find((m) => m.value === script.mode)?.label ?? script.mode}</span>,
          actions: [
            [run],
            [
              { id: "reveal", title: "Show in File Explorer", icon: Folder, shortcut: { key: "Enter", ctrl: true }, run: () => invoke<void>("show_in_explorer", { path: script.path }) },
              { id: "open-with", title: "Open With…", icon: SquarePen, shortcut: { key: "o", ctrl: true }, run: () => invoke<void>("open_with", { path: script.path }) },
              {
                id: "copy-contents",
                title: "Copy Script Contents",
                icon: Clipboard,
                shortcut: { key: "c", ctrl: true, alt: true },
                run: async () => copyAndClose((await invoke<{ content: string }>("flowkey_fs", { op: "readText", path: script.path })).content, "Copied Script Contents"),
              },
            ],
            [create, addFolder],
          ],
        };
      }),
    [scripts, values],
  );

  if (output) return <OutputView script={output.script} args={output.args} onBack={() => setOutput(null)} />;
  if (creating)
    return (
      <CreateScript
        onBack={() => {
          setCreating(false);
          reload();
        }}
      />
    );

  return (
    <ListView
      placeholder="Search script commands…"
      sections={[{ title: "Script Commands", items }]}
      onBack={pop}
      footerTitle="Search Script Commands"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={scripts === null}
      emptyText="No Script Commands"
      emptyDescription="Press Ctrl+N to create one, or add a folder with scripts from the actions."
      emptyActions={[[create, addFolder]]}
      selectedId={selectPath}
      onSelectionChange={setCurrent}
      argument={
        selected && firstArgument
          ? { placeholder: firstArgument.placeholder ?? "Argument", value: values[selected.path] ?? "", onChange: (value) => setValues((v) => ({ ...v, [selected.path]: value })) }
          : undefined
      }
    />
  );
}

function CreateScript({ onBack }: { onBack: () => void }) {
  const [template, setTemplate] = useState<ScriptTemplate>("powershell");
  const [mode, setMode] = useState<ScriptMode>("silent");
  const [title, setTitle] = useState("");
  const [description, setDescription] = useState("");
  const [packageName, setPackageName] = useState("");
  const [needsConfirmation, setNeedsConfirmation] = useState(false);
  const [withArgument, setWithArgument] = useState(false);
  const ids = useId();
  const inline = mode === "inline";

  async function create() {
    try {
      const dir = (await loadDirectories())[0] ?? (await pickDirectory());
      if (!dir) return;
      const path = `${dir.replace(/[\\/]$/, "")}\\${scriptFileName(title, template)}`;
      const exists = await invoke<{ exists?: boolean }>("flowkey_fs", { op: "exists", path }).catch(() => ({ exists: false }));
      if (exists.exists) throw new Error(`${path} already exists`);
      const content = scriptTemplate({ template, title, mode, description, packageName, needsConfirmation: needsConfirmation && !inline, withArgument: withArgument && !inline });
      await invoke("flowkey_fs", { op: "writeText", path, content });
      await addDirectory(dir);
      await invoke<void>("show_in_explorer", { path });
      showHUD({ variant: "success", title: "Script command created" });
      await invoke("hide_launcher", { restoreFocus: false });
    } catch (e) {
      fail("Failed to create script", e);
    }
  }

  const field = "h-[34px] w-full rounded-lg border border-key-border bg-card px-3 text-13 text-fg caret-fg outline-none placeholder:text-fg-40 focus:border-fg-40";
  const label = "pt-2 text-right text-13 text-fg-60";
  return (
    <div
      className="flex min-h-0 flex-1 flex-col"
      onKeyDown={(event) => {
        if (matchesShortcut(event, { key: "Enter", ctrl: true })) {
          event.preventDefault();
          create();
        } else if (event.key === "Escape" && !event.defaultPrevented) {
          event.preventDefault();
          onBack();
        }
      }}
    >
      <header className="flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <BackButton onBack={onBack} />
        <h2 className="text-13 font-medium text-fg-60">Create Script Command</h2>
      </header>
      <form
        className="grid min-h-0 flex-1 auto-rows-min grid-cols-[175px_360px] content-start gap-x-5 gap-y-4 overflow-y-auto pt-1.5 pb-4"
        onSubmit={(e) => {
          e.preventDefault();
          create();
        }}
      >
        <span id={`${ids}-template`} className={label}>
          Template
        </span>
        <Dropdown size="field" labelledBy={`${ids}-template`} value={template} options={TEMPLATES} onChange={setTemplate} />
        <span id={`${ids}-mode`} className={label}>
          Mode
        </span>
        <Dropdown size="field" labelledBy={`${ids}-mode`} value={mode} options={MODES} onChange={setMode} />
        <label htmlFor={`${ids}-title`} className={label}>
          Title
        </label>
        <input id={`${ids}-title`} autoFocus spellCheck={false} value={title} onChange={(e) => setTitle(e.target.value)} placeholder="Command Title" className={field} />
        <label htmlFor={`${ids}-description`} className={label}>
          Description
        </label>
        <input id={`${ids}-description`} spellCheck={false} value={description} onChange={(e) => setDescription(e.target.value)} placeholder="Descriptive summary" className={field} />
        <label htmlFor={`${ids}-package`} className={label}>
          Package Name
        </label>
        <input id={`${ids}-package`} spellCheck={false} value={packageName} onChange={(e) => setPackageName(e.target.value)} placeholder="E.g., Developer Utils" className={field} />
        {!inline && (
          <>
            <span />
            <label className="flex items-center gap-2 text-13 text-fg-60">
              <Checkbox checked={needsConfirmation} onChange={setNeedsConfirmation} label="Needs confirmation" />
              Needs confirmation
            </label>
            <span />
            <label className="flex items-center gap-2 text-13 text-fg-60">
              <Checkbox checked={withArgument} onChange={setWithArgument} label="With argument" />
              With argument
            </label>
          </>
        )}
      </form>
      <Footer
        left={
          <span className="flex items-center gap-2 text-fg-60">
            <GlyphTile spec={ICON} small />
            Create Script Command
          </span>
        }
        primaryTitle="Create Script"
        primaryKeys={["Ctrl", "↵"]}
        showActions={false}
        panelOpen={false}
        onTogglePanel={() => {}}
      />
    </div>
  );
}

function scriptCommand(script: Script): Command {
  const arg = script.arguments[0];
  const base: Command = {
    id: script.path,
    title: script.title,
    subtitle: script.packageName || "Script Commands",
    icon: isEmojiIcon(script.icon) ? <span className="flex size-[22px] shrink-0 items-center justify-center text-[16px]">{script.icon}</span> : ICON,
    keywords: ["script", script.packageName],
    kind: "Script Command",
    primaryActionTitle: "Run Script",
    confirm: confirmation(script),
  };
  if (script.mode === "fullOutput") return { ...base, view: (props: ViewProps) => (arg ? <SearchScripts {...props} selectPath={script.path} /> : <OutputView script={script} args={[]} onBack={props.pop} />) };
  if (arg) return { ...base, argument: { placeholder: arg.placeholder ?? "Argument", optional: !!arg.optional, run: (value) => runQuietly(script, value.trim() ? [value.trim()] : []) } };
  return { ...base, run: () => runQuietly(script, []) };
}

export default defineExtension({
  id: "script-commands",
  title: "Script Commands",
  icon: ICON,
  commands: [
    { id: "search", title: "Search Script Commands", subtitle: "Script Commands", keywords: ["find", "browse", "scripts"], kind: "Command", primaryActionTitle: "Open Command", view: SearchScripts },
    {
      id: "create",
      title: "Create Script Command",
      subtitle: "Script Commands",
      icon: { glyph: Plus, tone: "slate" },
      keywords: ["new", "create", "script", "template"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: ({ pop }) => <CreateScript onBack={pop} />,
    },
    {
      id: "reload",
      title: "Reload Script Commands",
      subtitle: "Script Commands",
      icon: { glyph: RefreshCw, tone: "slate" },
      keywords: ["refresh", "rescan", "reload"],
      kind: "Command",
      primaryActionTitle: "Reload",
      run: async () => {
        const scripts = await loadScripts();
        showHUD({ variant: "success", title: `Reloaded ${scripts.length} script command${scripts.length === 1 ? "" : "s"}` });
      },
    },
  ],
  items: async () => (await loadScripts()).map(scriptCommand),
});
