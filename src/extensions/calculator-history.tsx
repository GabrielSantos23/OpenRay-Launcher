import { invoke } from "@tauri-apps/api/core";
import { ArrowDown, ArrowLeftRight, ArrowUp, Calculator, Clipboard, ClipboardPaste, Clock, FileText, Pin, PinOff, TextCursorInput, Trash2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import { fail } from "../ActionHost";
import { groupHistory, movePinned, search, setPinned, upsert, type Calculation, type CalculationKind } from "../calculatorHistory";
import type { Action, IconSpec, Item, ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { ListView } from "../ListView";
import { copyAndClose, frontmostApp, pasteTitle, type AppInfo } from "../output";
import { calculate, type CalculatorAnswer } from "./calculator";

const ICON: IconSpec = { glyph: Calculator, tone: "orange" };
const NAME = "calculator-history";

async function load(): Promise<Calculation[]> {
  const value = await invoke<Calculation[] | null>("data_get", { name: NAME });
  return Array.isArray(value) ? value : [];
}

async function save(list: Calculation[]) {
  await invoke("data_set", { name: NAME, value: list });
}

export async function saveCalculation(expression: string, result: string, kind: CalculationKind): Promise<void> {
  try {
    await save(upsert(await load(), expression, result, kind, Date.now()).list);
  } catch (e) {
    console.error("could not save calculation", e);
  }
}

const KIND_ICONS = { math: Calculator, unitConversion: ArrowLeftRight, dateTime: Clock };

function kindIcon(c: Calculation) {
  const Icon = /[+\-*/^%×÷]$/.test(c.expression.trim()) ? FileText : KIND_ICONS[c.kind] ?? FileText;
  return <Icon aria-hidden="true" className="size-4 shrink-0 text-fg-60" />;
}

function copyPaste(answer: string, app: AppInfo | null, after?: () => Promise<void>): Action[] {
  const then = (run: () => Promise<unknown>) => async () => {
    await run();
    await after?.();
  };
  return [
    { id: "copy-answer", title: "Copy Answer", icon: Clipboard, shortcut: { key: "c", ctrl: true, shift: true }, run: then(() => copyAndClose(answer, "Copied answer to clipboard")) },
    { id: "paste-answer", title: pasteTitle(app), icon: ClipboardPaste, shortcut: { key: "Enter", ctrl: true }, run: then(() => invoke("paste_text", { text: answer })) },
  ];
}

export function CalculatorHistory({ pop }: ViewProps) {
  const [text, setText] = useState("");
  const [list, setList] = useState<Calculation[] | null>(null);
  const [live, setLive] = useState<CalculatorAnswer | null>(null);
  const [app, setApp] = useState<AppInfo | null>(null);

  const reload = useCallback(() => {
    load()
      .then(setList)
      .catch((e) => fail("Could not load calculator history", e));
  }, []);

  useEffect(() => {
    reload();
    frontmostApp().then(setApp);
  }, [reload]);

  useEffect(() => {
    let current = true;
    calculate(text).then((result) => current && setLive(result));
    return () => {
      current = false;
    };
  }, [text]);

  const update = useCallback(
    (change: (list: Calculation[]) => Calculation[]) => async () => {
      await save(change(await load()));
      reload();
    },
    [reload],
  );

  const sections = useMemo(() => {
    const all = list ?? [];
    const order = all.filter((c) => c.pinned !== undefined).sort((a, b) => a.pinned! - b.pinned!);
    const historyActions = (c: Calculation): Action[][] => {
      const isPinned = c.pinned !== undefined;
      const index = order.findIndex((p) => p.id === c.id);
      return [
        [
          ...copyPaste(c.result, app),
          { id: "copy-question", title: "Copy Question", icon: Clipboard, shortcut: { key: "c", ctrl: true, alt: true }, run: () => copyAndClose(c.expression) },
          { id: "copy-question-and-answer", title: "Copy Question and Answer", icon: Clipboard, shortcut: { key: "c", alt: true, shift: true }, run: () => copyAndClose(`${c.expression} = ${c.result}`) },
        ],
        [
          { id: "put-question", title: "Put Question in Search Bar", icon: TextCursorInput, shortcut: { key: "Enter", alt: true, shift: true }, keepOpen: true, run: () => setText(`${c.expression} `) },
          { id: "put-answer", title: "Put Answer in Search Bar", icon: TextCursorInput, shortcut: { key: "Enter", ctrl: true, shift: true }, keepOpen: true, run: () => setText(c.result) },
        ],
        [
          isPinned
            ? { id: "unpin", title: "Unpin Entry", icon: PinOff, shortcut: { key: ".", ctrl: true }, keepOpen: true, run: update((l) => setPinned(l, c.id, false)) }
            : { id: "pin", title: "Pin Entry", icon: Pin, shortcut: { key: ".", ctrl: true }, keepOpen: true, run: update((l) => setPinned(l, c.id, true)) },
          ...(isPinned && index > 0 ? [{ id: "move-up", title: "Move Up in Pinned", icon: ArrowUp, shortcut: { key: "ArrowUp", ctrl: true, alt: true }, keepOpen: true, run: update((l) => movePinned(l, c.id, -1)) }] : []),
          ...(isPinned && index < order.length - 1 ? [{ id: "move-down", title: "Move Down in Pinned", icon: ArrowDown, shortcut: { key: "ArrowDown", ctrl: true, alt: true }, keepOpen: true, run: update((l) => movePinned(l, c.id, 1)) }] : []),
        ],
        [
          { id: "delete", title: "Delete Entry", icon: Trash2, destructive: true, keepOpen: true, shortcut: { key: "d", ctrl: true }, run: update((l) => l.filter((e) => e.id !== c.id)) },
          {
            id: "delete-all",
            title: "Delete All Entries",
            icon: Trash2,
            destructive: true,
            keepOpen: true,
            shortcut: { key: "d", ctrl: true, alt: true },
            confirm: { title: "Delete All Entries", message: "Are you sure you want to proceed with deleting all calculator history entries? Only pinned entries will remain.", confirmTitle: "Delete All" },
            run: update((l) => l.filter((e) => e.pinned !== undefined)),
          },
        ],
      ];
    };

    const result: { title: string; items: Item[] }[] = [];
    if (live) {
      const saveLive = async () => {
        await saveCalculation(live.input.text, live.output.text, live.kind);
        reload();
      };
      const liveSaved = all.find((c) => c.expression === live.input.text.trim() && c.result === live.output.text);
      result.push({
        title: "",
        items: [
          {
            id: "live",
            title: live.input.text,
            subtitle: live.output.label,
            accessory: <span className="text-13 text-fg">{live.output.text}</span>,
            icon: <Calculator aria-hidden="true" className="size-4 shrink-0 text-fg-60" />,
            actions: [
              [
                ...copyPaste(live.output.text, app, saveLive),
                { id: "copy-question-and-answer", title: "Copy Question and Answer", icon: Clipboard, shortcut: { key: "c", alt: true, shift: true }, run: async () => { await copyAndClose(`${live.input.text} = ${live.output.text}`); await saveLive(); } },
              ],
              [
                { id: "put-answer", title: "Put Answer in Search Bar", icon: TextCursorInput, shortcut: { key: "Enter", ctrl: true, shift: true }, keepOpen: true, run: () => setText(live.output.text) },
                liveSaved?.pinned !== undefined
                  ? { id: "unpin", title: "Unpin Entry", icon: PinOff, shortcut: { key: ".", ctrl: true }, keepOpen: true, run: update((l) => setPinned(l, liveSaved!.id, false)) }
                  : {
                      id: "pin",
                      title: "Pin Entry",
                      icon: Pin,
                      shortcut: { key: ".", ctrl: true },
                      keepOpen: true,
                      run: async () => {
                        const saved = upsert(await load(), live.input.text, live.output.text, live.kind, Date.now());
                        await save(setPinned(saved.list, saved.id, true));
                        reload();
                      },
                    },
              ],
            ],
          },
        ],
      });
    }
    for (const section of groupHistory(search(all, text), Date.now())) {
      result.push({
        title: section.title,
        items: section.items.map((c) => ({
          id: c.id,
          title: c.expression,
          icon: kindIcon(c),
          accessory: <span className="text-13 text-fg-60">{c.result}</span>,
          actions: historyActions(c),
        })),
      });
    }
    return result;
  }, [list, live, text, app, reload, update]);

  const empty = (list?.length ?? 0) === 0 && !text;
  return (
    <ListView
      placeholder="Do math, convert units, or search your past calculations..."
      sections={sections}
      onBack={pop}
      searchText={text}
      onSearchTextChange={setText}
      filtering={false}
      footerTitle="Calculator History"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={list === null}
      emptyText={empty ? "You have no recent calculations" : "No results"}
      emptyDescription={empty ? 'Try "200 + 10%", "100 usd in eur", or "Tokyo time"' : "Try a different expression"}
    />
  );
}
