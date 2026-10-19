import { ArrowLeft } from "lucide-react";
import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Footer, Loader, useActionHost } from "./ActionHost";
import { matchesShortcut } from "./actions";
import type { Action, Item } from "./extension-api";
import { matchScore } from "./search";

export interface ListSection {
  title: string;
  items: Item[];
  showCount?: boolean;
}

export interface ArgumentField {
  placeholder: string;
  value: string;
  onChange: (value: string) => void;
}

interface ListViewProps {
  placeholder: string;
  sections: ListSection[];
  onBack: () => void;
  footerTitle: string;
  footerIcon?: ReactNode;
  searchText?: string;
  onSearchTextChange?: (text: string) => void;
  filtering?: boolean;
  layout?: "list" | "grid";
  columns?: number;
  detail?: (item: Item) => ReactNode;
  accessory?: ReactNode;
  argument?: ArgumentField;
  emptyText?: string;
  emptyDescription?: string;
  tileTitles?: boolean;
  isLoading?: boolean;
  selectedId?: string;
  onSelectionChange?: (item: Item | undefined) => void;
}

export function filterSections(sections: ListSection[], query: string): ListSection[] {
  if (!query.trim()) return sections;
  return sections
    .map((section) => ({ ...section, items: section.items.filter((item) => matchScore(query, item.title, item.keywords) > 0) }))
    .filter((section) => section.items.length > 0);
}

export function BackButton({ onBack }: { onBack: () => void }) {
  return (
    <button
      type="button"
      tabIndex={-1}
      aria-label="Back"
      title="Back"
      onClick={onBack}
      className="flex size-[22px] shrink-0 items-center justify-center rounded-[5px] bg-control text-fg hover:bg-control-hover"
    >
      <ArrowLeft className="size-3.5" />
    </button>
  );
}

export function ListView({
  placeholder,
  sections,
  onBack,
  footerTitle,
  footerIcon,
  searchText,
  onSearchTextChange,
  filtering = true,
  layout = "list",
  columns = 8,
  detail,
  accessory,
  argument,
  emptyText = "No Results",
  emptyDescription,
  tileTitles = false,
  isLoading = false,
  selectedId,
  onSelectionChange,
}: ListViewProps) {
  const [ownText, setOwnText] = useState("");
  const text = searchText ?? ownText;
  const setText = onSearchTextChange ?? setOwnText;
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const argumentRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const host = useActionHost(() => inputRef.current?.focus());

  const visible = useMemo(() => (filtering ? filterSections(sections, text) : sections), [sections, text, filtering]);
  const flat = useMemo(() => visible.flatMap((s) => s.items), [visible]);
  const active = Math.max(0, Math.min(selected, flat.length - 1));
  const current = flat[active];
  const pendingId = useRef(selectedId);

  useEffect(() => {
    if (!pendingId.current) return;
    const index = flat.findIndex((item) => item.id === pendingId.current);
    if (index >= 0) {
      setSelected(index);
      pendingId.current = undefined;
    }
  }, [flat]);

  useEffect(() => {
    onSelectionChange?.(current);
  }, [current, onSelectionChange]);

  useEffect(() => {
    listRef.current?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: "nearest" });
  }, [active, visible]);

  const rows = useMemo(() => {
    const result: { start: number; length: number }[] = [];
    let start = 0;
    for (const section of visible) {
      for (let i = 0; i < section.items.length; i += columns) result.push({ start: start + i, length: Math.min(columns, section.items.length - i) });
      start += section.items.length;
    }
    return result;
  }, [visible, columns]);

  function moveRow(step: number) {
    const row = rows.findIndex((r) => active >= r.start && active < r.start + r.length);
    const target = rows[row + step];
    if (!target) return;
    setSelected(target.start + Math.min(active - rows[row].start, target.length - 1));
  }

  const groups = current?.actions.filter((group) => group.length > 0) ?? [];
  const primary = groups[0]?.[0];

  function runAction(action: Action) {
    if (current) host.run(action, { failTitle: `${action.title.replace(/…$/, "")} failed` });
  }

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    if (matchesShortcut(event, { key: "k", ctrl: true })) {
      event.preventDefault();
      if (current) host.openPanel(current.title, groups);
      return;
    }
    const plainEnter = event.key === "Enter" && !event.ctrlKey && !event.altKey && !event.shiftKey;
    if (plainEnter) {
      event.preventDefault();
      if (primary) runAction(primary);
      return;
    }
    const action = groups.flat().find((a) => a.shortcut && matchesShortcut(event, a.shortcut));
    if (current && action) {
      event.preventDefault();
      runAction(action);
    } else if (event.key === "ArrowDown") {
      event.preventDefault();
      if (layout === "grid") moveRow(1);
      else setSelected(Math.min(active + 1, flat.length - 1));
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      if (layout === "grid") moveRow(-1);
      else setSelected(Math.max(active - 1, 0));
    } else if (layout === "grid" && (event.key === "ArrowLeft" || event.key === "ArrowRight") && !event.shiftKey) {
      event.preventDefault();
      setSelected(Math.max(0, Math.min(active + (event.key === "ArrowRight" ? 1 : -1), flat.length - 1)));
    } else if (event.key === "Escape") {
      event.preventDefault();
      if (event.currentTarget.value) event.currentTarget === argumentRef.current ? argument?.onChange("") : setText("");
      else onBack();
    } else if (event.key === "Backspace" && !event.currentTarget.value) {
      event.preventDefault();
      if (event.currentTarget === argumentRef.current) inputRef.current?.focus();
      else onBack();
    } else if (event.key === "Tab" && argument) {
      event.preventDefault();
      (event.currentTarget === argumentRef.current ? inputRef : argumentRef).current?.focus();
    }
  }

  let index = -1;
  const option = (item: Item, i: number) => `view-option-${i}-${item.id}`;

  return (
    <>
      <header className="relative flex h-header shrink-0 items-center gap-3 border-b border-separator px-search-inset">
        <BackButton onBack={onBack} />
        <input
          ref={inputRef}
          autoFocus
          spellCheck={false}
          value={text}
          onChange={(e) => {
            setText(e.target.value);
            setSelected(0);
          }}
          onKeyDown={onKeyDown}
          placeholder={placeholder}
          aria-label={placeholder.replace(/…$/, "")}
          role="combobox"
          aria-expanded="true"
          aria-controls="view-results"
          aria-activedescendant={current ? option(current, active) : undefined}
          style={argument ? { width: `${Math.max(text.length, placeholder.length) + 1}ch` } : undefined}
          className={`h-full min-w-0 bg-transparent text-18 text-fg caret-fg outline-none placeholder:text-fg-40 ${argument ? "max-w-[60%] shrink" : "flex-1"}`}
        />
        {argument && (
          <input
            ref={argumentRef}
            spellCheck={false}
            value={argument.value}
            onChange={(e) => argument.onChange(e.target.value)}
            onKeyDown={onKeyDown}
            placeholder={argument.placeholder}
            aria-label={argument.placeholder}
            style={{ width: `${Math.max(argument.value.length, argument.placeholder.length) + 2}ch` }}
            className="h-6 max-w-[200px] min-w-0 rounded-[5px] bg-control px-2 text-13 text-fg caret-fg outline-none placeholder:text-fg-60 focus:bg-control-hover"
          />
        )}
        <span className="flex-1" />
        {accessory}
        <Loader active={isLoading || host.busy} />
      </header>

      <div className="flex min-h-0 flex-1">
        <div
          ref={listRef}
          id="view-results"
          role="listbox"
          aria-label="Results"
          aria-busy={isLoading}
          className={`min-h-0 overflow-y-auto px-inset pb-inset ${detail ? "w-[285px] shrink-0 border-r border-separator" : "flex-1"}`}
        >
          {flat.length === 0 && !isLoading && (
            <div className="flex h-full flex-col items-center justify-center gap-1 px-6 text-center text-13">
              <p className={emptyDescription ? "font-medium text-fg" : "text-fg-60"}>{emptyText}</p>
              {emptyDescription && <p className="max-w-[420px] text-fg-60">{emptyDescription}</p>}
            </div>
          )}
          {visible.map((section) => (
            <div key={section.title} role="group" aria-label={section.title || "Results"}>
              {section.title ? (
                <div className="flex h-8 items-end gap-2 px-2 pb-[5px] text-11 font-semibold text-fg-60">
                  {section.title}
                  {section.showCount && <span className="font-medium text-fg-40">{section.items.length}</span>}
                </div>
              ) : (
                <div className="h-2" />
              )}
              <div className={layout === "grid" ? "grid gap-2" : ""} style={layout === "grid" ? { gridTemplateColumns: `repeat(${columns}, minmax(0, 1fr))` } : undefined}>
                {section.items.map((item) => {
                  index += 1;
                  const i = index;
                  const isSelected = i === active;
                  return (
                    <div
                      key={item.id}
                      id={option(item, i)}
                      data-index={i}
                      role="option"
                      aria-selected={isSelected}
                      aria-label={layout === "grid" ? item.title : undefined}
                      title={layout === "grid" ? item.title : undefined}
                      onMouseMove={() => i !== selected && setSelected(i)}
                      onClick={() => {
                        setSelected(i);
                        if (primary && i === active) runAction(primary);
                      }}
                      className={
                        layout === "grid" && tileTitles
                          ? "flex min-w-0 cursor-default flex-col gap-1.5 pb-1"
                          : layout === "grid"
                            ? `flex aspect-square cursor-default items-center justify-center rounded-row bg-list-hover text-[40px] leading-none ${isSelected ? "bg-list-selection shadow-[inset_0_0_0_2px_var(--color-fg-60)]" : ""}`
                            : `flex h-row cursor-default items-center gap-3 rounded-row px-2 text-13 ${isSelected ? "bg-list-selection" : ""}`
                      }
                    >
                      {layout === "grid" && tileTitles ? (
                        <>
                          <span
                            className={`flex aspect-square items-center justify-center overflow-hidden rounded-row bg-list-hover text-[40px] leading-none ${isSelected ? "bg-list-selection shadow-[inset_0_0_0_2px_var(--color-fg-60)]" : ""}`}
                          >
                            {item.icon}
                          </span>
                          <span className="truncate px-0.5 text-12 font-medium text-fg">{item.title}</span>
                          {item.subtitle && <span className="-mt-1 truncate px-0.5 text-11 text-fg-60">{item.subtitle}</span>}
                        </>
                      ) : layout === "grid" ? (
                        item.icon
                      ) : (
                        <>
                          {item.icon}
                          <span className="truncate font-medium">{item.title}</span>
                          {item.subtitle && <span className="truncate text-fg-60">{item.subtitle}</span>}
                          {item.accessory && <span className="ml-auto shrink-0 text-fg-60">{item.accessory}</span>}
                        </>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
        {detail && <div className="flex min-w-0 flex-1 flex-col">{current && detail(current)}</div>}
      </div>

      <Footer
        left={
          <span className="flex min-w-0 items-center gap-2 text-fg-60">
            {footerIcon}
            <span className="truncate">{footerTitle}</span>
          </span>
        }
        primaryTitle={primary?.title}
        panelOpen={host.panelOpen}
        onTogglePanel={() => (host.panelOpen ? host.closePanel() : current && host.openPanel(current.title, groups))}
      />
      {host.render(runAction)}
      <div aria-live="polite" className="sr-only">
        {flat.length} {flat.length === 1 ? "result" : "results"}
      </div>
    </>
  );
}

export function Metadata({ rows }: { rows: { label: string; value: ReactNode }[] }) {
  return (
    <section aria-labelledby="metadata-title" className="max-h-[148px] shrink-0 overflow-y-auto border-t border-separator px-4 pt-3 pb-2 text-13">
      <h2 id="metadata-title" className="pb-2 text-12 font-semibold text-fg-60">
        Information
      </h2>
      <dl>
        {rows.map((row) => (
          <div key={row.label} className="flex min-h-[33px] items-center justify-between gap-4 border-t border-separator first:border-t-0">
            <dt className="shrink-0 text-fg-60">{row.label}</dt>
            <dd className="flex min-w-0 items-center gap-1.5 truncate text-right font-medium text-fg">{row.value}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}
