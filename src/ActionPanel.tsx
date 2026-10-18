import { Fragment, useEffect, useMemo, useRef, useState } from "react";
import { matchesShortcut, shortcutLabels } from "./actions";
import type { Action } from "./extension-api";
import { matchScore } from "./search";

export function Keys({ labels }: { labels: string[] }) {
  return (
    <span className="flex shrink-0 gap-0.5">
      {labels.map((label) => (
        <kbd
          key={label}
          className="flex h-[18px] min-w-[18px] items-center justify-center rounded-[4px] border border-key-border px-1 font-sans text-11 text-fg-60"
        >
          {label}
        </kbd>
      ))}
    </span>
  );
}

export interface PanelPlacement {
  className: string;
  searchFirst?: boolean;
}

const FOOTER_PLACEMENT: PanelPlacement = { className: "absolute right-2 bottom-[51px] w-[350px]" };

interface ActionPanelProps {
  title: string;
  groups: Action[][];
  onRun: (action: Action) => void;
  onClose: () => void;
  placement?: PanelPlacement;
}

export function ActionPanel({ title, groups, onRun, onClose, placement = FOOTER_PLACEMENT }: ActionPanelProps) {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);
  const visible = useMemo(
    () => groups.map((group) => group.filter((action) => matchScore(query, action.title) > 0)).filter((group) => group.length > 0),
    [groups, query],
  );
  const flat = visible.flat();
  const active = Math.min(selected, flat.length - 1);

  useEffect(() => {
    listRef.current?.querySelector(`[data-index="${active}"]`)?.scrollIntoView({ block: "nearest" });
  }, [active]);

  function onKeyDown(event: React.KeyboardEvent<HTMLInputElement>) {
    const plain = !event.ctrlKey && !event.altKey && !event.shiftKey;
    if (event.key === "Escape" || matchesShortcut(event, { key: "k", ctrl: true })) {
      event.preventDefault();
      onClose();
    } else if (plain && event.key === "Enter") {
      event.preventDefault();
      if (flat[active] && !flat[active].disabled) onRun(flat[active]);
    } else if (plain && event.key === "ArrowDown") {
      event.preventDefault();
      setSelected(Math.min(active + 1, flat.length - 1));
    } else if (plain && event.key === "ArrowUp") {
      event.preventDefault();
      setSelected(Math.max(active - 1, 0));
    } else {
      const action = flat.find((a) => a.shortcut && !a.disabled && matchesShortcut(event, a.shortcut));
      if (action) {
        event.preventDefault();
        onRun(action);
      }
    }
  }

  const search = (
    <input
      autoFocus
      spellCheck={false}
      value={query}
      onChange={(e) => {
        setQuery(e.target.value);
        setSelected(0);
      }}
      onKeyDown={onKeyDown}
      placeholder="Search for actions..."
      aria-label="Search for actions"
      role="combobox"
      aria-expanded="true"
      aria-controls="actions"
      aria-activedescendant={flat[active] ? `action-${active}` : undefined}
      className={`h-[38px] shrink-0 border-panel-separator bg-transparent px-[15px] text-13 text-fg caret-fg outline-none placeholder:text-fg-40 ${placement.searchFirst ? "border-b" : "border-t"}`}
    />
  );

  let index = -1;
  return (
    <>
      <div className="fixed inset-0 z-10" onMouseDown={onClose} />
      <div
        role="dialog"
        aria-label={`Actions for ${title}`}
        className={`z-20 flex flex-col rounded-panel border border-panel-border bg-panel shadow-panel ${placement.className}`}
      >
        {placement.searchFirst && search}
        <div ref={listRef} id="actions" role="listbox" aria-label="Actions" className="max-h-[239px] overflow-y-auto [scrollbar-width:none] px-[7px] pt-[7px] pb-2">
          <div className="flex h-6 items-center px-2 text-11 font-semibold text-fg-60">{title}</div>
          {flat.length === 0 && <p className="flex h-8 items-center px-2 text-13 text-fg-60">No matching actions</p>}
          {visible.map((group, g) => (
            <Fragment key={group[0].id}>
              {g > 0 && <div role="separator" className="-mx-[7px] mt-2 mb-[7px] h-px bg-panel-separator" />}
              {group.map((action) => {
                index += 1;
                const i = index;
                const Icon = action.icon;
                return (
                  <div
                    key={action.id}
                    id={`action-${i}`}
                    data-index={i}
                    role="option"
                    aria-selected={i === active}
                    aria-disabled={action.disabled || undefined}
                    onMouseMove={() => i !== active && setSelected(i)}
                    onClick={() => !action.disabled && onRun(action)}
                    className={`flex h-8 cursor-default items-center gap-2 rounded-row px-2 text-13 font-medium ${i === active ? "bg-panel-selection" : ""} ${action.destructive ? "text-destructive" : "text-fg"} ${action.disabled ? "opacity-40" : ""}`}
                  >
                    {Icon ? <Icon className="size-4 shrink-0" /> : <span className="size-4 shrink-0" />}
                    <span className="truncate">{action.title}</span>
                    {action.accessory && <span className="ml-auto min-w-0 truncate pl-2 font-normal text-fg-60">{action.accessory}</span>}
                    {action.shortcut && (
                      <span className="ml-auto">
                        <Keys labels={shortcutLabels(action.shortcut)} />
                      </span>
                    )}
                  </div>
                );
              })}
            </Fragment>
          ))}
        </div>
        {!placement.searchFirst && search}
      </div>
    </>
  );
}
