import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { Clipboard, CopyPlus, Folder, Globe, List, Pencil, Pin, PinOff, Play, Plus, Settings2, Shield, SquareTerminal, Terminal, Trash2, Variable, Wrench } from "lucide-react";
import { useCallback, useEffect, useId, useMemo, useRef, useState, type ReactNode } from "react";
import { fail, Footer } from "../ActionHost";
import { matchesShortcut } from "../actions";
import { Dropdown } from "../Dropdown";
import { defineExtension, type Action, type Command, type IconSpec, type Item, type ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { BackButton, ListView } from "../ListView";
import { copyAndClose } from "../output";
import { isPathLike, joinCommand, KIND_LABELS, runCommandSections, runRows, splitCommand, type RunCommand, type RunEntry, type RunKind, type RunRow } from "../run";
import { showToast } from "../toast";

const ICON: IconSpec = { glyph: SquareTerminal, tone: "slate" };

const KIND_ICONS: Record<RunKind, typeof Terminal> = {
  command: Play,
  controlPanelApplet: Settings2,
  environmentVariable: Variable,
  executable: Terminal,
  managementConsole: Wrench,
  path: Folder,
  uri: Globe,
};

type Filter = "all" | RunKind;

const glyph = (Icon: typeof Terminal) => <Icon aria-hidden="true" className="size-4 shrink-0" />;
const FILTERS: { value: Filter; label: string; icon: ReactNode }[] = [
  { value: "all", label: "All Types", icon: glyph(List) },
  ...(Object.keys(KIND_LABELS) as RunKind[]).sort((a, b) => KIND_LABELS[a].localeCompare(KIND_LABELS[b])).map((kind) => ({ value: kind, label: KIND_LABELS[kind], icon: glyph(KIND_ICONS[kind]) })),
];

export async function loadRunCommands(): Promise<RunCommand[]> {
  const value = await invoke<RunCommand[] | null>("data_get", { name: "run-commands" });
  return Array.isArray(value) ? value : [];
}

function saveRunCommands(list: RunCommand[]) {
  return invoke("data_set", { name: "run-commands", value: list });
}

async function execute(text: string, admin = false, shell = false) {
  const { command, arguments: args } = splitCommand(text);
  await invoke("run_execute", { command, arguments: args, admin, shell });
}

async function executeRecord(record: RunCommand, extra = "", admin = false) {
  await invoke("run_execute", { command: record.command, arguments: [record.arguments, extra].filter((a) => a.trim()).join(" "), admin, shell: false });
  const list = await loadRunCommands();
  await saveRunCommands(list.map((c) => (c.id === record.id ? { ...c, lastExecutedAt: Date.now() } : c)));
}

function RowIcon({ row }: { row: RunRow }) {
  const [failed, setFailed] = useState(false);
  const path = row.kind === "path" || row.kind === "executable" || row.kind === "controlPanelApplet" || row.kind === "managementConsole" ? row.description : "";
  if (!path || failed) return <span className="flex size-[22px] shrink-0 items-center justify-center text-fg-60">{glyph(KIND_ICONS[row.kind])}</span>;
  return <img src={`${convertFileSrc(path, "appicon")}?size=${Math.round(22 * window.devicePixelRatio)}`} alt="" draggable={false} onError={() => setFailed(true)} className="size-[22px] shrink-0" />;
}

function RunCommandForm({ initial, initialCommand, onDone, onBack }: { initial?: RunCommand; initialCommand?: string; onDone: () => void; onBack: () => void }) {
  const [command, setCommand] = useState(initial ? joinCommand(initial.command, initial.arguments) : (initialCommand ?? ""));
  const [alias, setAlias] = useState(initial?.alias ?? "");
  const [description, setDescription] = useState(initial?.description ?? "");
  const [tags, setTags] = useState(initial?.tags.join(", ") ?? "");
  const [errors, setErrors] = useState<{ command?: string; alias?: string }>({});
  const commandRef = useRef<HTMLInputElement>(null);
  const aliasRef = useRef<HTMLInputElement>(null);
  const ids = useId();
  const editing = initial && !initial.id.startsWith("copy:");
  const title = editing ? "Edit Run Command" : "Create Run Command";

  async function save() {
    try {
      const list = await loadRunCommands();
      const name = alias.trim();
      const next = {
        command: command.trim() ? undefined : "Command is required",
        alias: !name ? "Alias is required" : list.some((c) => c.alias.toLowerCase() === name.toLowerCase() && c.id !== initial?.id) ? "Another run command uses this alias" : undefined,
      };
      setErrors(next);
      if (next.command) return commandRef.current?.focus();
      if (next.alias) return aliasRef.current?.focus();
      const parts = splitCommand(command);
      const record: RunCommand = {
        ...(editing ? initial : {}),
        id: editing ? initial.id : crypto.randomUUID(),
        alias: name,
        command: parts.command,
        arguments: parts.arguments,
        description: description.trim(),
        tags: tags.split(",").map((t) => t.trim()).filter(Boolean),
      };
      await saveRunCommands(editing ? list.map((c) => (c.id === record.id ? record : c)) : [...list, record]);
      showToast({ variant: "success", title: editing ? "Updated Run Command" : "Created Run Command" });
      onDone();
    } catch (e) {
      fail("Could not save Run Command", e);
    }
  }

  function onKeyDown(event: React.KeyboardEvent) {
    if (matchesShortcut(event, { key: "Enter", ctrl: true })) {
      event.preventDefault();
      save();
    } else if (event.key === "Escape" && !event.defaultPrevented) {
      event.preventDefault();
      onBack();
    }
  }

  const field = "h-[34px] w-full rounded-lg border bg-card px-3 text-13 text-fg caret-fg outline-none placeholder:text-fg-40";
  const border = (error?: string) => (error ? "border-red" : "border-key-border focus:border-fg-40");
  const label = "pt-2 text-right text-13 text-fg-60";
  return (
    <div className="flex min-h-0 flex-1 flex-col" onKeyDown={onKeyDown}>
      <header className="flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <BackButton onBack={onBack} />
        <h2 className="text-13 font-medium text-fg-60">{title}</h2>
      </header>
      <form
        className="grid min-h-0 flex-1 auto-rows-min grid-cols-[175px_360px] content-start gap-x-5 gap-y-5 overflow-y-auto pt-1.5"
        onSubmit={(e) => {
          e.preventDefault();
          save();
        }}
      >
        <label htmlFor={`${ids}-command`} className={label}>
          Command
        </label>
        <div className="flex flex-col gap-1.5">
          <input id={`${ids}-command`} ref={commandRef} autoFocus spellCheck={false} value={command} onChange={(e) => setCommand(e.target.value)} placeholder="E.g. ping 1.1.1.1 -t" aria-invalid={!!errors.command} className={`${field} ${border(errors.command)}`} />
          <p className={`text-12 ${errors.command ? "text-red" : "text-fg-60"}`}>{errors.command ?? "A program, file, folder, URI or %VARIABLE%, followed by its arguments."}</p>
        </div>
        <label htmlFor={`${ids}-alias`} className={label}>
          Alias
        </label>
        <div className="flex flex-col gap-1.5">
          <input id={`${ids}-alias`} ref={aliasRef} spellCheck={false} value={alias} onChange={(e) => setAlias(e.target.value)} placeholder="Command alias" aria-invalid={!!errors.alias} className={`${field} ${border(errors.alias)}`} />
          {errors.alias && <p className="text-12 text-red">{errors.alias}</p>}
        </div>
        <label htmlFor={`${ids}-description`} className={label}>
          Description
        </label>
        <input id={`${ids}-description`} spellCheck={false} value={description} onChange={(e) => setDescription(e.target.value)} placeholder="Description (optional)" className={`${field} ${border()}`} />
        <label htmlFor={`${ids}-tags`} className={label}>
          Tags
        </label>
        <div className="flex flex-col gap-1.5">
          <input id={`${ids}-tags`} spellCheck={false} value={tags} onChange={(e) => setTags(e.target.value)} placeholder="Tags (optional)" className={`${field} ${border()}`} />
          <p className="text-12 text-fg-60">Separate tags with commas.</p>
        </div>
      </form>
      <Footer
        left={
          <span className="flex items-center gap-2 text-fg-60">
            <GlyphTile spec={ICON} small />
            {title}
          </span>
        }
        primaryTitle="Save Run Command"
        primaryKeys={["Ctrl", "↵"]}
        showActions={false}
        panelOpen={false}
        onTogglePanel={() => {}}
      />
    </div>
  );
}

function useRunCommands() {
  const [list, setList] = useState<RunCommand[] | null>(null);
  const reload = useCallback(() => {
    loadRunCommands()
      .then(setList)
      .catch((e) => {
        setList([]);
        fail("Could not load Run Commands", e);
      });
  }, []);
  useEffect(reload, [reload]);
  return [list, reload] as const;
}

function OpenRun({ pop }: ViewProps) {
  const [text, setText] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [catalog, setCatalog] = useState<RunEntry[] | null>(null);
  const [history, setHistory] = useState<string[]>([]);
  const [completions, setCompletions] = useState<RunEntry[]>([]);
  const [commands, reloadCommands] = useRunCommands();
  const [form, setForm] = useState<{ initial?: RunCommand; command?: string } | null>(null);

  const reloadHistory = useCallback(() => {
    invoke<string[]>("run_history").then(setHistory).catch(() => setHistory([]));
  }, []);
  useEffect(() => {
    reloadHistory();
    invoke<RunEntry[]>("run_catalog")
      .then(setCatalog)
      .catch((e) => {
        setCatalog([]);
        fail("Could not load commands", e);
      });
  }, [reloadHistory]);

  useEffect(() => {
    if (!isPathLike(text)) return setCompletions([]);
    let live = true;
    invoke<RunEntry[]>("run_complete", { text }).then((list) => live && setCompletions(list)).catch(() => live && setCompletions([]));
    return () => {
      live = false;
    };
  }, [text]);

  const sections = useMemo(() => {
    const groups = runRows(text, history, catalog ?? [], commands ?? [], completions, filter === "all" ? undefined : [filter]);
    return groups.map((group) => ({
      title: group.title,
      items: group.rows.map((row): Item => {
        const full = row.record ? joinCommand(row.record.command, row.record.arguments) : row.command;
        const runnable = row.kind === "executable" || row.kind === "managementConsole" || row.kind === "command";
        const primary: Action[] = [
          { id: "run", title: "Run Command", icon: Play, run: () => (row.record ? executeRecord(row.record) : execute(row.command)) },
          ...(runnable ? [{ id: "run-admin", title: "Run Command as Administrator", icon: Shield, shortcut: { key: "Enter", ctrl: true, shift: true }, run: () => (row.record ? executeRecord(row.record, "", true) : execute(full, true)) }] : []),
          ...(row.kind === "executable" || row.kind === "command" ? [{ id: "run-shell", title: "Run in Shell", icon: Terminal, shortcut: { key: "Enter", ctrl: true }, run: () => execute(full, false, true) }] : []),
          { id: "copy", title: "Copy Command", icon: Clipboard, shortcut: { key: "c", ctrl: true, shift: true }, run: () => copyAndClose(full, "Copied to Clipboard") },
          ...(row.kind === "environmentVariable" && row.source !== "input" ? [{ id: "copy-value", title: "Copy Value", icon: Clipboard, shortcut: { key: "c", ctrl: true, alt: true }, run: () => copyAndClose(row.description, "Copied to Clipboard") }] : []),
          ...(row.source !== "input" && (row.kind === "path" || (row.kind === "executable" && row.description)) ? [{ id: "reveal", title: "Show in File Explorer", icon: Folder, run: () => invoke<void>("show_in_explorer", { path: row.description.replace(/\\$/, "") }) }] : []),
        ];
        const manage: Action[] = row.record
          ? [{ id: "edit", title: "Edit Run Command", icon: Pencil, shortcut: { key: "e", ctrl: true }, keepOpen: true, run: () => setForm({ initial: row.record }) }]
          : [{ id: "create", title: "Create Run Command", icon: Plus, shortcut: { key: "n", ctrl: true }, keepOpen: true, run: () => setForm({ command: row.command }) }];
        const destructive: Action[] = [
          ...(row.source === "history"
            ? [{ id: "forget", title: "Delete from History", icon: Trash2, destructive: true, keepOpen: true, shortcut: { key: "d", ctrl: true }, run: async () => {
                await invoke("run_forget", { command: row.command });
                showToast({ variant: "success", title: "Deleted from history" });
                reloadHistory();
              } }]
            : []),
          ...(row.record
            ? [{ id: "delete", title: "Delete Run Command", icon: Trash2, destructive: true, keepOpen: true, shortcut: { key: "d", ctrl: true, shift: true }, run: async () => {
                await saveRunCommands((commands ?? []).filter((c) => c.id !== row.record!.id));
                showToast({ variant: "success", title: "Deleted Run Command" });
                reloadCommands();
              } }]
            : []),
        ];
        return {
          id: row.id,
          title: row.command,
          subtitle: row.source === "input" ? undefined : row.description,
          icon: <RowIcon row={row} />,
          accessory: <span className="text-13 text-fg-60">{KIND_LABELS[row.kind]}</span>,
          actions: [primary, manage, destructive],
        };
      }),
    }));
  }, [text, history, catalog, commands, completions, filter, reloadHistory, reloadCommands]);

  if (form) {
    const close = () => {
      setForm(null);
      reloadCommands();
    };
    return <RunCommandForm initial={form.initial} initialCommand={form.command} onDone={close} onBack={() => setForm(null)} />;
  }

  return (
    <ListView
      placeholder="Type a command…"
      sections={sections}
      onBack={pop}
      searchText={text}
      onSearchTextChange={setText}
      filtering={false}
      footerTitle="Run"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={catalog === null}
      emptyText="No Commands"
      emptyDescription="Type a command above and press Enter to run it"
      accessory={
        <div className="w-[215px] shrink-0">
          <span id="run-kind" className="sr-only">
            Filter Results
          </span>
          <Dropdown size="field" labelledBy="run-kind" value={filter} options={FILTERS} onChange={setFilter} />
        </div>
      }
    />
  );
}

function SearchRunCommands({ pop }: ViewProps) {
  const [list, reload] = useRunCommands();
  const [form, setForm] = useState<{ initial?: RunCommand } | null>(null);

  const sections = useMemo(
    () =>
      runCommandSections(list ?? []).map((section) => ({
        title: section.title,
        items: section.items.map((record): Item => {
          const full = joinCommand(record.command, record.arguments);
          return {
            id: record.id,
            title: record.alias,
            subtitle: record.description || full,
            keywords: [record.command, record.arguments, ...record.tags],
            icon: <span className="flex size-[22px] shrink-0 items-center justify-center text-fg-60">{glyph(Play)}</span>,
            accessory: record.tags.length ? <span className="truncate text-13 text-fg-60">{record.tags.join(", ")}</span> : undefined,
            actions: [
              [
                { id: "execute", title: "Execute Command", icon: Play, run: () => executeRecord(record) },
                { id: "copy", title: "Copy to Clipboard", icon: Clipboard, shortcut: { key: "c", ctrl: true, shift: true }, run: () => copyAndClose(full, "Copied to Clipboard") },
              ],
              [
                {
                  id: "pin",
                  title: record.pinned ? "Unpin" : "Pin",
                  icon: record.pinned ? PinOff : Pin,
                  shortcut: { key: "p", ctrl: true, shift: true },
                  keepOpen: true,
                  run: async () => {
                    await saveRunCommands((list ?? []).map((c) => (c.id === record.id ? { ...c, pinned: !c.pinned } : c)));
                    showToast({ variant: "success", title: record.pinned ? "Unpinned" : "Pinned" });
                    reload();
                  },
                },
                { id: "edit", title: "Edit", icon: Pencil, shortcut: { key: "e", ctrl: true }, keepOpen: true, run: () => setForm({ initial: record }) },
                {
                  id: "duplicate",
                  title: "Duplicate",
                  icon: CopyPlus,
                  keepOpen: true,
                  run: () => setForm({ initial: { ...record, id: `copy:${record.id}`, alias: `${record.alias} copy`, pinned: false, lastExecutedAt: undefined } }),
                },
                { id: "create", title: "Create Run Command", icon: Plus, shortcut: { key: "n", ctrl: true }, keepOpen: true, run: () => setForm({}) },
              ],
              [
                {
                  id: "delete",
                  title: "Delete",
                  icon: Trash2,
                  destructive: true,
                  keepOpen: true,
                  shortcut: { key: "d", ctrl: true },
                  confirm: { title: "Delete Run Command?", message: "This action cannot be undone.", confirmTitle: "Delete" },
                  run: async () => {
                    await saveRunCommands((list ?? []).filter((c) => c.id !== record.id));
                    showToast({ variant: "success", title: "Deleted" });
                    reload();
                  },
                },
                {
                  id: "delete-all",
                  title: "Delete All",
                  icon: Trash2,
                  destructive: true,
                  keepOpen: true,
                  shortcut: { key: "d", ctrl: true, shift: true },
                  confirm: { title: "Delete All Run Commands?", message: "This action cannot be undone.", confirmTitle: "Delete All" },
                  run: async () => {
                    await saveRunCommands([]);
                    showToast({ variant: "success", title: "Deleted all" });
                    reload();
                  },
                },
              ],
            ],
          };
        }),
      })),
    [list, reload],
  );

  if (form) {
    const close = () => {
      setForm(null);
      reload();
    };
    return <RunCommandForm initial={form.initial} onDone={close} onBack={() => setForm(null)} />;
  }

  return (
    <ListView
      placeholder="Search run commands…"
      sections={sections}
      onBack={pop}
      footerTitle="Search Run Commands"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={list === null}
      emptyActions={[[{ id: "create", title: "Create Run Command", icon: Plus, shortcut: { key: "n", ctrl: true }, keepOpen: true, run: () => setForm({}) }]]}
      emptyText="No Run Commands"
      emptyDescription="Press Ctrl+N to create one."
    />
  );
}

function runCommandItem(record: RunCommand): Command {
  return {
    id: record.id,
    title: record.alias.charAt(0).toUpperCase() + record.alias.slice(1),
    subtitle: "Run",
    icon: { glyph: Play, tone: "slate" },
    keywords: ["run", "command", record.command, ...(record.arguments ? [record.arguments] : []), ...record.tags],
    kind: "Run Command",
    primaryActionTitle: "Run Command",
    argument: { placeholder: "Arguments", optional: true, run: (value) => executeRecord(record, value) },
  };
}

export default defineExtension({
  id: "windows-run",
  title: "Run",
  icon: ICON,
  commands: [
    { id: "windows-run", title: "Open Run", subtitle: "Run", keywords: ["run", "execute", "win+r", "command"], kind: "Command", primaryActionTitle: "Open Command", view: OpenRun },
    {
      id: "execute",
      title: "Execute Run Command",
      subtitle: "Run",
      icon: { glyph: Terminal, tone: "slate" },
      keywords: ["run", "execute", "command"],
      kind: "Command",
      primaryActionTitle: "Run Command",
      argument: { placeholder: "Command", run: (value) => execute(value) },
    },
    { id: "search", title: "Search Run Commands", subtitle: "Run", icon: { glyph: List, tone: "slate" }, keywords: ["run", "alias", "commands"], kind: "Command", primaryActionTitle: "Open Command", view: SearchRunCommands },
    {
      id: "create",
      title: "Create Run Command",
      subtitle: "Run",
      icon: { glyph: Plus, tone: "slate" },
      keywords: ["run", "alias", "new"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: ({ pop }) => <RunCommandForm onDone={pop} onBack={pop} />,
    },
  ],
  items: async () => (await loadRunCommands()).map(runCommandItem),
});
