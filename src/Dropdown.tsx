import { ChevronDown } from "lucide-react";
import { useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { matchScore } from "./search";

export interface DropdownOption<T extends string> {
  value: T;
  label: string;
  icon?: ReactNode;
}

const MENU_MAX_HEIGHT = 280;
const MENU_GAP = 4;

export function Overlay({ onClose }: { onClose: () => void }) {
  return (
    <div
      className="fixed inset-0 z-30"
      onMouseDown={(e) => {
        e.preventDefault();
        onClose();
      }}
    />
  );
}

export function useAnchorRect<T extends HTMLElement>(open: boolean) {
  const ref = useRef<T>(null);
  const [rect, setRect] = useState<DOMRect | null>(null);
  useLayoutEffect(() => {
    setRect(open && ref.current ? ref.current.getBoundingClientRect() : null);
  }, [open]);
  return [ref, rect] as const;
}

export function Dropdown<T extends string>({
  value,
  options,
  onChange,
  labelledBy,
  size = "default",
}: {
  value: T;
  options: DropdownOption<T>[];
  onChange: (value: T) => void;
  labelledBy: string;
  size?: "default" | "small";
}) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [active, setActive] = useState(0);
  const [triggerRef, rect] = useAnchorRect<HTMLButtonElement>(open);
  const listRef = useRef<HTMLDivElement>(null);
  const selected = options.find((o) => o.value === value) ?? options[0];
  const visible = useMemo(() => options.filter((o) => matchScore(query, o.label) > 0), [options, query]);
  const current = Math.min(active, visible.length - 1);

  useEffect(() => {
    if (!open) return;
    setQuery("");
    setActive(Math.max(0, options.findIndex((o) => o.value === value)));
  }, [open, options, value]);

  useEffect(() => {
    listRef.current?.querySelector(`[data-index="${current}"]`)?.scrollIntoView({ block: "nearest" });
  }, [current, open]);

  function close() {
    setOpen(false);
    triggerRef.current?.focus();
  }

  function choose(option: DropdownOption<T> | undefined) {
    if (option && option.value !== value) onChange(option.value);
    close();
  }

  function onTriggerKeyDown(event: React.KeyboardEvent) {
    const step = event.key === "ArrowDown" ? 1 : event.key === "ArrowUp" ? -1 : 0;
    if (step) {
      event.preventDefault();
      const index = options.findIndex((o) => o.value === value);
      onChange(options[(index + step + options.length) % options.length].value);
    } else if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      setOpen(true);
    }
  }

  function onMenuKeyDown(event: React.KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      if (query) setQuery("");
      else close();
    } else if (event.key === "Enter") {
      event.preventDefault();
      choose(visible[current]);
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      const step = event.key === "ArrowDown" ? 1 : -1;
      setActive((a) => (Math.min(a, visible.length - 1) + step + visible.length) % visible.length);
    }
  }

  const below = rect ? window.innerHeight - rect.bottom - MENU_GAP >= Math.min(MENU_MAX_HEIGHT, 40 + options.length * 32 + 16) : true;
  const position = rect
    ? {
        right: window.innerWidth - rect.right,
        minWidth: rect.width,
        ...(below ? { top: rect.bottom + MENU_GAP } : { bottom: window.innerHeight - rect.top + MENU_GAP }),
      }
    : undefined;

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-labelledby={labelledBy}
        onPointerDown={(e) => {
          e.preventDefault();
          setOpen((o) => !o);
        }}
        onKeyDown={onTriggerKeyDown}
        className={`flex items-center gap-2 font-medium text-fg outline-offset-2 ${
          size === "small"
            ? "h-6 rounded-[5px] bg-list-hover pr-1.5 pl-2 shadow-[0_0_0_1px_var(--color-control-hover)]"
            : `h-8 rounded-md pr-2 pl-3 ${open ? "bg-control" : "hover:bg-list-hover"}`
        }`}
      >
        {selected?.icon}
        <span className="truncate">{selected?.label}</span>
        <ChevronDown aria-hidden="true" className="size-3 shrink-0 text-fg-60" />
      </button>
      {open && rect && (
        <>
          <Overlay onClose={close} />
          <div
            role="dialog"
            aria-labelledby={labelledBy}
            onKeyDown={onMenuKeyDown}
            style={position}
            className={`fixed z-40 flex max-w-[360px] flex-col rounded-panel border border-panel-border bg-panel text-13 shadow-panel ${below ? "" : "flex-col-reverse"}`}
          >
            <input
              autoFocus
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                setActive(0);
              }}
              placeholder="Search..."
              aria-label="Search options"
              spellCheck={false}
              role="combobox"
              aria-expanded="true"
              aria-controls="dropdown-options"
              aria-activedescendant={visible[current] ? `dropdown-option-${current}` : undefined}
              className={`h-10 shrink-0 bg-transparent px-4 text-fg caret-fg outline-none placeholder:text-fg-40 ${below ? "border-b" : "border-t"} border-panel-separator`}
            />
            <div ref={listRef} id="dropdown-options" role="listbox" className="max-h-[240px] overflow-y-auto p-[7px]">
              {visible.length === 0 && <p className="flex h-8 items-center px-2 text-fg-60">No results</p>}
              {visible.map((option, i) => (
                <div
                  key={option.value}
                  id={`dropdown-option-${i}`}
                  data-index={i}
                  role="option"
                  aria-selected={option.value === value}
                  onMouseMove={() => i !== current && setActive(i)}
                  onClick={() => choose(option)}
                  className={`flex h-8 cursor-default items-center gap-2 rounded-row px-2 font-medium whitespace-nowrap text-fg ${i === current ? "bg-panel-selection" : ""}`}
                >
                  {option.icon}
                  <span className="truncate">{option.label}</span>
                </div>
              ))}
            </div>
          </div>
        </>
      )}
    </>
  );
}
