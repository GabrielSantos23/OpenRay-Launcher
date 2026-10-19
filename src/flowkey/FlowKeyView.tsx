import { invoke } from "@tauri-apps/api/core";
import type { Arguments, DetailTree, FormField, FormTree, GridTree, ListTree, ManifestCommand, UiAccessory, UiAction, UiField, UiItem, UiPane, UiShortcut, UiTree } from "@flowkey-cli/native-sdk";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Footer, Loader, useActionHost } from "../ActionHost";
import { matchesShortcut } from "../actions";
import { Dropdown } from "../Dropdown";
import type { Action, Item, Shortcut } from "../extension-api";
import { BackButton, ListView, type ListSection } from "../ListView";
import { actionIcon, FlowKeyIcon, resolveImage } from "./FlowKeyIcon";
import { Markdown } from "./Markdown";
import type { ProtocolError } from "./protocol";
import { hostFor, newViewId, type InstalledExtension } from "./runtime";

interface Entry {
  viewId: string;
  commandId: string;
  query: string;
  filterValue?: string;
  tree: UiTree | null;
  error: ProtocolError | null;
}

export interface WindowSignal {
  extensionId: string;
  viewId?: string;
  command: "closeMainWindow" | "popToRoot" | "clearSearchBar";
}

export const WINDOW_EVENT = "flowkey-window";

const KEY_NAMES: Record<string, string> = {
  enter: "Enter",
  return: "Enter",
  arrowright: "ArrowRight",
  rightarrow: "ArrowRight",
  arrowleft: "ArrowLeft",
  leftarrow: "ArrowLeft",
  arrowup: "ArrowUp",
  uparrow: "ArrowUp",
  arrowdown: "ArrowDown",
  downarrow: "ArrowDown",
  backspace: "Backspace",
  delete: "Delete",
  escape: "Escape",
  tab: "Tab",
  space: " ",
  pageup: "PageUp",
  pagedown: "PageDown",
  home: "Home",
  end: "End",
};

export function toShortcut(shortcut?: UiShortcut): Shortcut | undefined {
  if (!shortcut?.key) return undefined;
  const modifiers = (shortcut.modifiers ?? []).map((m) => m.toLowerCase());
  const lower = shortcut.key.toLowerCase();
  return {
    key: KEY_NAMES[lower] ?? (shortcut.key.length === 1 ? lower : shortcut.key),
    ctrl: modifiers.some((m) => ["cmd", "command", "ctrl", "control", "meta", "win"].includes(m)),
    alt: modifiers.some((m) => ["alt", "opt", "option"].includes(m)),
    shift: modifiers.includes("shift"),
  };
}

export function orderActions(actions: UiAction[] = []): UiAction[] {
  const primary = actions.findIndex((a) => a.primary);
  return primary <= 0 ? actions : [actions[primary], ...actions.filter((_, i) => i !== primary)];
}

function toActions(actions: UiAction[] | undefined, run: (action: UiAction) => Promise<void>): Action[][] {
  const mapped: Action[] = orderActions(actions).map((action, i) => ({
    id: action.id,
    title: action.title,
    icon: actionIcon(action.title) ?? undefined,
    shortcut: i === 0 ? { key: "Enter" } : (toShortcut(action.shortcut) ?? (i === 1 ? { key: "Enter", ctrl: true } : undefined)),
    destructive: action.style === "destructive",
    keepOpen: true,
    run: () => run(action),
  }));
  return [mapped.slice(0, 1), mapped.slice(1)].filter((group) => group.length > 0);
}

const ACCESSORY_TONES: Record<NonNullable<UiAccessory["color"]>, string> = {
  success: "text-green",
  danger: "text-red",
  accent: "text-accent",
  secondary: "text-fg-60",
};

function Accessories({ accessories }: { accessories: UiAccessory[] }) {
  return (
    <span className="flex shrink-0 items-center gap-3">
      {accessories.map((accessory, i) => (
        <span key={i} title={accessory.tooltip} className={`max-w-[180px] truncate ${ACCESSORY_TONES[accessory.color ?? "secondary"]}`}>
          {accessory.text}
        </span>
      ))}
    </span>
  );
}

function FieldValue({ field }: { field: UiField }) {
  if (field.kind === "tags" || field.tags) {
    return (
      <span className="flex flex-wrap justify-end gap-1">
        {(field.tags ?? []).map((tag) => (
          <span key={tag} className="rounded-[4px] bg-control px-1.5 py-px text-12 font-medium">
            {tag}
          </span>
        ))}
      </span>
    );
  }
  if ((field.kind === "link" || field.href) && field.href && /^https?:\/\//i.test(field.href)) {
    return (
      <a
        href={field.href}
        onClick={(e) => {
          e.preventDefault();
          invoke("open_url", { url: field.href }).catch((err) => console.error("open_url failed", err));
        }}
        className="truncate text-accent hover:underline"
      >
        {field.value || field.href}
      </a>
    );
  }
  return <span className="truncate">{field.value}</span>;
}

function MetadataList({ fields, extension, valueIcons }: { fields: (UiField & { valueIconUri?: string })[]; extension: InstalledExtension; valueIcons?: boolean }) {
  return (
    <dl className="flex flex-col text-13">
      {fields.map((field, i) =>
        field.kind === "separator" ? (
          <hr key={i} className="my-1 border-separator" />
        ) : (
          <div key={i} className="flex min-h-[33px] flex-col justify-center gap-0.5 py-1.5">
            <dt className="text-12 text-fg-60">{field.label}</dt>
            <dd className="flex min-w-0 items-center gap-1.5 font-medium text-fg">
              {valueIcons && field.valueIconUri && <FlowKeyIcon fields={{ iconUri: field.valueIconUri }} extension={extension} size="small" />}
              <FieldValue field={field} />
            </dd>
          </div>
        ),
      )}
    </dl>
  );
}

function Pane({ pane, extension }: { pane: UiPane; extension: InstalledExtension }) {
  const image = resolveImage(pane.previewImageUri, extension);
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="min-h-0 flex-1 overflow-y-auto px-4 py-3">
        {image && <img src={image} alt="" className="mb-3 max-h-[220px] max-w-full rounded-[6px] object-contain" />}
        {pane.preview && <Markdown source={pane.preview} extension={extension} />}
      </div>
      {pane.fields && pane.fields.length > 0 && (
        <section aria-label="Information" className="max-h-[50%] shrink-0 overflow-y-auto border-t border-separator px-4 py-2">
          <MetadataList fields={pane.fields.map((f) => ({ ...f, kind: "text" as const }))} extension={extension} valueIcons />
        </section>
      )}
    </div>
  );
}

function CollectionView({
  tree,
  entry,
  extension,
  command,
  onBack,
  onQuery,
  onFilter,
  run,
  loadMore,
}: {
  tree: ListTree | GridTree;
  entry: Entry;
  extension: InstalledExtension;
  command: ManifestCommand;
  onBack: () => void;
  onQuery: (query: string) => void;
  onFilter: (value: string) => void;
  run: (action: UiAction, item?: UiItem) => Promise<void>;
  loadMore: (actionId: string) => void;
}) {
  const grid = tree.type === "grid";
  const rawSections = grid && !tree.sections?.length ? [{ title: "", items: tree.items }] : (tree.sections ?? []);
  const panes = useRef(new Map<string, UiPane>());
  const sections: ListSection[] = useMemo(() => {
    panes.current.clear();
    return rawSections.map((section, s) => ({
      title: section.title ?? "",
      items: section.items.map((item): Item => {
        const id = `${s}:${item.id}`;
        if (item.pane) panes.current.set(id, item.pane);
        return {
          id,
          title: item.title,
          subtitle: item.subtitle,
          keywords: item.keywords,
          icon: <FlowKeyIcon fields={item} extension={extension} size={grid ? "tile" : "row"} title={item.title} />,
          accessory: !grid && item.accessories?.length ? <Accessories accessories={item.accessories} /> : undefined,
          actions: toActions(item.actions, (action) => run(action, item)),
        };
      }),
    }));
  }, [tree, extension]);
  const hasPane = !grid && (tree.layout === "side-pane" || sections.some((s) => s.items.some((i) => panes.current.has(i.id))));
  const filter = tree.filter?.options ?? [];
  const pagination = tree.type === "list" ? tree.pagination : undefined;
  const requested = useRef<string | null>(null);
  const lastId = sections.at(-1)?.items.at(-1)?.id;
  const onSelectionChange = useCallback(
    (item: Item | undefined) => {
      if (!pagination?.hasNextPage || !item || item.id !== lastId || requested.current === lastId) return;
      requested.current = lastId;
      loadMore(pagination.moreActionId);
    },
    [pagination?.hasNextPage, pagination?.moreActionId, lastId, loadMore],
  );
  return (
    <ListView
      placeholder={tree.searchBarPlaceholder ?? (/^search/i.test(command.title) ? `${command.title}...` : `Search ${grid && tree.title ? tree.title : command.title}...`)}
      sections={sections}
      onBack={onBack}
      footerTitle={command.title}
      footerIcon={<FlowKeyIcon fields={{ icon: command.icon ?? extension.manifest.icon, iconColor: command.iconColor }} extension={extension} size="small" title={command.title} />}
      searchText={entry.query}
      onSearchTextChange={onQuery}
      filtering={false}
      layout={grid ? "grid" : "list"}
      columns={grid ? Math.max(1, Math.min(8, tree.columns || 5)) : undefined}
      tileTitles={grid}
      detail={hasPane ? (item) => (panes.current.get(item.id) ? <Pane pane={panes.current.get(item.id)!} extension={extension} /> : null) : undefined}
      accessory={
        filter.length > 0 ? (
          <>
            <span id="flowkey-filter" className="sr-only">
              Filter
            </span>
            <Dropdown value={entry.filterValue ?? filter[0].value} options={filter.map((o) => ({ value: o.value, label: o.label }))} onChange={onFilter} labelledBy="flowkey-filter" size="small" />
          </>
        ) : undefined
      }
      emptyText={entry.error ? "Something went wrong" : (tree.emptyView?.title ?? "No Results")}
      emptyDescription={entry.error ? entry.error.message : tree.emptyView?.description}
      isLoading={!!tree.isLoading}
      onSelectionChange={pagination ? onSelectionChange : undefined}
    />
  );
}

function keyHandler(groups: Action[][], runAction: (action: Action) => void, onBack: () => void, openPanel: () => void, panelOpen: boolean) {
  return (event: React.KeyboardEvent) => {
    if (panelOpen) return;
    if (matchesShortcut(event, { key: "k", ctrl: true })) {
      event.preventDefault();
      openPanel();
      return;
    }
    if (event.key === "Escape" || (event.key === "Backspace" && !(event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement))) {
      event.preventDefault();
      onBack();
      return;
    }
    const action = groups.flat().find((a) => a.shortcut && matchesShortcut(event, a.shortcut));
    if (action && !(event.key === "Enter" && event.target instanceof HTMLTextAreaElement && !event.ctrlKey)) {
      event.preventDefault();
      runAction(action);
    }
  };
}

function DetailView({ tree, extension, command, onBack, run }: { tree: DetailTree; extension: InstalledExtension; command: ManifestCommand; onBack: () => void; run: (action: UiAction) => Promise<void> }) {
  const rootRef = useRef<HTMLDivElement>(null);
  const host = useActionHost(() => rootRef.current?.focus());
  const groups = useMemo(() => toActions(tree.actions, run), [tree.actions, run]);
  const primary = groups[0]?.[0];
  const runAction = (action: Action) => host.run(action, { failTitle: `${action.title.replace(/…$/, "")} failed` });
  const onKeyDown = keyHandler(groups, runAction, onBack, () => groups.length && host.openPanel(tree.title, groups), host.panelOpen);
  const image = resolveImage(tree.imageUri, extension);
  useEffect(() => rootRef.current?.focus(), []);
  return (
    <div ref={rootRef} tabIndex={-1} onKeyDown={onKeyDown} className="flex min-h-0 flex-1 flex-col outline-none">
      <header className="relative flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <BackButton onBack={onBack} />
        <h2 className="min-w-0 truncate text-18 text-fg">{tree.title}</h2>
        {tree.subtitle && <span className="min-w-0 truncate text-13 text-fg-60">{tree.subtitle}</span>}
        <Loader active={host.busy} />
      </header>
      <div className="flex min-h-0 flex-1">
        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">
          {image && <img src={image} alt="" className="mb-4 max-h-[200px] max-w-full rounded-[6px] object-contain" />}
          {tree.description && <Markdown source={tree.description} extension={extension} />}
        </div>
        {tree.fields.length > 0 && (
          <aside aria-label="Information" className="w-[260px] shrink-0 overflow-y-auto border-l border-separator px-4 py-3">
            <MetadataList fields={tree.fields} extension={extension} />
          </aside>
        )}
      </div>
      <Footer
        left={
          <span className="flex min-w-0 items-center gap-2 text-fg-60">
            <FlowKeyIcon fields={{ icon: command.icon ?? extension.manifest.icon, iconColor: command.iconColor }} extension={extension} size="small" title={command.title} />
            <span className="truncate">{command.title}</span>
          </span>
        }
        primaryTitle={primary?.title}
        panelOpen={host.panelOpen}
        showActions={groups.flat().length > 1}
        onTogglePanel={() => (host.panelOpen ? host.closePanel() : groups.length && host.openPanel(tree.title, groups))}
      />
      {host.render(runAction)}
    </div>
  );
}

function initialValue(field: FormField): unknown {
  if (field.kind === "checkbox") return field.default === true;
  if (field.kind === "tagpicker") return field.defaults ?? [];
  if (field.kind === "filepicker") return [];
  if (field.kind === "dropdown") return typeof field.default === "string" ? field.default : (field.options?.[0]?.value ?? "");
  return typeof field.default === "string" ? field.default : "";
}

const INPUT = "h-[30px] w-full rounded-[6px] border border-panel-border bg-field px-2 text-13 text-fg outline-none placeholder:text-fg-40 focus:border-accent";

function FormInput({ field, value, onChange, invalid }: { field: FormField; value: unknown; onChange: (value: unknown) => void; invalid: boolean }) {
  const id = `form-${field.id}`;
  switch (field.kind) {
    case "textarea":
      return <textarea id={id} value={String(value ?? "")} placeholder={field.placeholder} aria-invalid={invalid} onChange={(e) => onChange(e.target.value)} className={`${INPUT} h-[96px] resize-none py-1.5`} />;
    case "checkbox":
      return (
        <label className="flex h-[30px] items-center gap-2 text-13 text-fg">
          <input id={id} type="checkbox" checked={value === true} onChange={(e) => onChange(e.target.checked)} className="size-4 accent-accent" />
          {field.placeholder}
        </label>
      );
    case "dropdown":
      return <Dropdown value={String(value ?? "")} options={(field.options ?? []).map((o) => ({ value: o.value, label: o.title }))} onChange={onChange} labelledBy={`${id}-label`} size="field" />;
    case "tagpicker": {
      const selected = Array.isArray(value) ? (value as string[]) : [];
      return (
        <div role="group" aria-labelledby={`${id}-label`} className="flex flex-wrap gap-1.5">
          {(field.options ?? []).map((option) => {
            const on = selected.includes(option.value);
            return (
              <button
                key={option.value}
                type="button"
                aria-pressed={on}
                onClick={() => onChange(on ? selected.filter((v) => v !== option.value) : [...selected, option.value])}
                className={`h-[26px] rounded-[6px] px-2 text-12 font-medium ${on ? "bg-accent text-white" : "bg-control text-fg hover:bg-control-hover"}`}
              >
                {option.title}
              </button>
            );
          })}
        </div>
      );
    }
    case "filepicker": {
      const paths = Array.isArray(value) ? (value as string[]) : [];
      return (
        <div className="flex min-w-0 items-center gap-2">
          <button
            id={id}
            type="button"
            onClick={async () => {
              const picked = await invoke<string[]>("flowkey_pick_path", { directory: field.canChooseDirectories === true && field.canChooseFiles === false, multiple: field.allowMultipleSelection === true });
              if (picked.length) onChange(picked);
            }}
            className="h-[30px] shrink-0 rounded-[6px] bg-control px-3 text-13 font-medium text-fg hover:bg-control-hover"
          >
            {field.canChooseDirectories && field.canChooseFiles === false ? "Choose Folder…" : "Choose File…"}
          </button>
          <span className="min-w-0 truncate text-12 text-fg-60">{paths.join(", ") || "Nothing selected"}</span>
        </div>
      );
    }
    default:
      return (
        <input
          id={id}
          type={field.kind === "password" ? "password" : field.kind === "datepicker" ? "date" : "text"}
          value={String(value ?? "")}
          placeholder={field.placeholder}
          aria-invalid={invalid}
          spellCheck={false}
          onChange={(e) => onChange(e.target.value)}
          className={INPUT}
        />
      );
  }
}

function FormView({ tree, extension, command, onBack, run }: { tree: FormTree; extension: InstalledExtension; command: ManifestCommand; onBack: () => void; run: (action: UiAction, values: Record<string, unknown>) => Promise<void> }) {
  const rootRef = useRef<HTMLFormElement>(null);
  const host = useActionHost(() => rootRef.current?.querySelector<HTMLElement>("input,textarea,button")?.focus());
  const [values, setValues] = useState<Record<string, unknown>>(() => Object.fromEntries(tree.fields.filter((f) => f.id).map((f) => [f.id, initialValue(f)])));
  const [invalid, setInvalid] = useState<string[]>([]);
  const valuesRef = useRef(values);
  valuesRef.current = values;
  useEffect(() => {
    setValues((current) => Object.fromEntries(tree.fields.filter((f) => f.id).map((f) => [f.id, f.id in current ? current[f.id] : initialValue(f)])));
  }, [tree.fields]);
  const actions = useMemo(() => {
    const declared = tree.actions ?? [];
    const submit = tree.submitActionId && !declared.some((a) => a.id === tree.submitActionId) ? [{ id: tree.submitActionId, title: "Submit", primary: true }] : [];
    return [...submit, ...declared];
  }, [tree.actions, tree.submitActionId]);
  const groups = useMemo(
    () =>
      toActions(actions, async (action) => {
        const missing = tree.fields.filter((f) => f.required && f.id && (valuesRef.current[f.id] === "" || (Array.isArray(valuesRef.current[f.id]) && (valuesRef.current[f.id] as unknown[]).length === 0))).map((f) => f.id);
        setInvalid(missing);
        if (missing.length) {
          rootRef.current?.querySelector<HTMLElement>(`#form-${CSS.escape(missing[0])}`)?.focus();
          return;
        }
        await run(action, valuesRef.current);
      }).map((group, g) => group.map((action, i) => (g === 0 && i === 0 ? { ...action, shortcut: { key: "Enter", ctrl: true } } : action))),
    [actions, run, tree.fields],
  );
  const primary = groups[0]?.[0];
  const runAction = (action: Action) => host.run(action, { failTitle: `${action.title.replace(/…$/, "")} failed` });
  const onKeyDown = keyHandler(groups, runAction, onBack, () => groups.length && host.openPanel(tree.title, groups), host.panelOpen);
  useEffect(() => {
    rootRef.current?.querySelector<HTMLElement>("input,textarea,button[id]")?.focus();
  }, []);
  return (
    <form ref={rootRef} onSubmit={(e) => e.preventDefault()} onKeyDown={onKeyDown} className="flex min-h-0 flex-1 flex-col">
      <header className="relative flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <BackButton onBack={onBack} />
        <h2 className="min-w-0 truncate text-18 text-fg">{tree.title}</h2>
        <Loader active={host.busy} />
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto px-6 py-4">
        <div className="mx-auto flex max-w-[560px] flex-col gap-3">
          {tree.fields.map((field, i) =>
            field.kind === "separator" ? (
              <hr key={i} className="border-separator" />
            ) : field.kind === "description" ? (
              <p key={i} className="text-13 text-fg-60">
                {field.label}
              </p>
            ) : (
              <div key={field.id || i} className="grid grid-cols-[150px_minmax(0,1fr)] items-start gap-3">
                <label id={`form-${field.id}-label`} htmlFor={`form-${field.id}`} className="flex h-[30px] items-center justify-end text-right text-13 text-fg-60">
                  {field.label}
                  {field.required && <span aria-hidden="true">&nbsp;*</span>}
                </label>
                <div className="min-w-0">
                  <FormInput field={field} value={values[field.id]} invalid={invalid.includes(field.id)} onChange={(value) => setValues((v) => ({ ...v, [field.id]: value }))} />
                  {invalid.includes(field.id) && <p className="pt-1 text-12 text-red">Required</p>}
                </div>
              </div>
            ),
          )}
        </div>
      </div>
      <Footer
        left={
          <span className="flex min-w-0 items-center gap-2 text-fg-60">
            <FlowKeyIcon fields={{ icon: command.icon ?? extension.manifest.icon, iconColor: command.iconColor }} extension={extension} size="small" title={command.title} />
            <span className="truncate">{command.title}</span>
          </span>
        }
        primaryTitle={primary?.title}
        primaryKeys={["Ctrl", "↵"]}
        panelOpen={host.panelOpen}
        showActions={groups.flat().length > 1}
        onTogglePanel={() => (host.panelOpen ? host.closePanel() : groups.length && host.openPanel(tree.title, groups))}
      />
      {host.render(runAction)}
    </form>
  );
}

export function LoadingView({ command, onBack, error }: { command: ManifestCommand; onBack: () => void; error?: ProtocolError | null }) {
  const rootRef = useRef<HTMLDivElement>(null);
  useEffect(() => rootRef.current?.focus(), []);
  return (
    <div
      ref={rootRef}
      tabIndex={-1}
      onKeyDown={(e) => {
        if (e.key === "Escape" || e.key === "Backspace") {
          e.preventDefault();
          onBack();
        }
      }}
      className="flex min-h-0 flex-1 flex-col outline-none"
    >
      <header className="relative flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <BackButton onBack={onBack} />
        <h2 className="min-w-0 truncate text-18 text-fg-40">{command.title}</h2>
        <Loader active={!error} />
      </header>
      <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-1 px-6 text-center text-13">
        {error && (
          <>
            <p className="font-medium text-fg">{command.title} failed to load</p>
            <p className="max-w-[460px] text-fg-60 select-text">{error.message}</p>
          </>
        )}
      </div>
      <Footer left={<span className="truncate text-fg-60">{command.title}</span>} panelOpen={false} onTogglePanel={() => {}} />
    </div>
  );
}

export function FlowKeyView({ extension, command, args, initialQuery = "", pop }: { extension: InstalledExtension; command: ManifestCommand; args?: Arguments; initialQuery?: string; pop: () => void }) {
  const host = useMemo(() => hostFor(extension), [extension]);
  const [stack, setStack] = useState<Entry[]>(() => [{ viewId: newViewId(), commandId: command.id, query: initialQuery, tree: null, error: null }]);
  const stackRef = useRef(stack);
  stackRef.current = stack;
  const top = stack[stack.length - 1];

  const patch = useCallback((viewId: string, change: Partial<Entry>) => {
    setStack((entries) => entries.map((e) => (e.viewId === viewId ? { ...e, ...change } : e)));
  }, []);

  const render = useCallback(
    (entry: Entry) => {
      host
        .render(entry.viewId, entry.commandId, { query: entry.query, filterValue: entry.filterValue, arguments: args }, {
          onTree: (tree) => patch(entry.viewId, { tree, error: null }),
          onError: (error) => patch(entry.viewId, { error }),
        })
        .catch((error: ProtocolError) => patch(entry.viewId, { error }));
    },
    [host, args, patch],
  );

  useEffect(() => {
    render(stackRef.current[0]);
    return () => stackRef.current.forEach((entry) => host.close(entry.viewId));
  }, [host]);

  useEffect(() => {
    const onSignal = (event: Event) => {
      const signal = (event as CustomEvent<WindowSignal>).detail;
      if (signal.extensionId !== extension.manifest.id || (signal.viewId && !stackRef.current.some((e) => e.viewId === signal.viewId))) return;
      if (signal.command === "popToRoot") pop();
      else if (signal.command === "clearSearchBar") {
        const current = stackRef.current[stackRef.current.length - 1];
        const next = { ...current, query: "" };
        patch(current.viewId, { query: "" });
        render(next);
      }
    };
    window.addEventListener(WINDOW_EVENT, onSignal);
    return () => window.removeEventListener(WINDOW_EVENT, onSignal);
  }, [extension, pop, patch, render]);

  const back = useCallback(() => {
    const entries = stackRef.current;
    if (entries.length <= 1) {
      pop();
      return;
    }
    host.close(entries[entries.length - 1].viewId);
    setStack(entries.slice(0, -1));
  }, [host, pop]);

  const push = useCallback(
    (commandId: string) => {
      const entry: Entry = { viewId: newViewId(), commandId, query: "", tree: null, error: null };
      setStack((entries) => [...entries, entry]);
      render(entry);
    },
    [render],
  );

  const run = useCallback(
    async (entry: Entry, action: UiAction, item?: UiItem, formValues?: Record<string, unknown>) => {
      try {
        await host.action(entry.viewId, action.id, item, formValues);
      } catch (error) {
        throw new Error((error as ProtocolError)?.message ?? String(error));
      }
      if (action.push) push(action.push);
    },
    [host, push],
  );

  const tree = top.tree;
  if (!tree) return <LoadingView key={top.viewId} command={command} onBack={back} error={top.error} />;
  if (tree.type === "detail") return <DetailView key={top.viewId} tree={tree} extension={extension} command={command} onBack={back} run={(action) => run(top, action)} />;
  if (tree.type === "form") return <FormView key={top.viewId} tree={tree} extension={extension} command={command} onBack={back} run={(action, values) => run(top, action, undefined, values)} />;
  return (
    <CollectionView
      key={top.viewId}
      tree={tree}
      entry={top}
      extension={extension}
      command={command}
      onBack={back}
      onQuery={(query) => {
        patch(top.viewId, { query });
        render({ ...top, query });
      }}
      onFilter={(filterValue) => {
        patch(top.viewId, { filterValue });
        render({ ...top, filterValue });
      }}
      run={(action, item) => run(top, action, item)}
      loadMore={(actionId) => {
        host.action(top.viewId, actionId).catch((error: ProtocolError) => patch(top.viewId, { error }));
      }}
    />
  );
}

