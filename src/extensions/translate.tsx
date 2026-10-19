import { invoke } from "@tauri-apps/api/core";
import { ArrowRightLeft, Clipboard, ClipboardPaste, Flag, Languages, WandSparkles } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { fail, Footer, Loader, useActionHost } from "../ActionHost";
import { matchesShortcut } from "../actions";
import { Dropdown, type DropdownOption } from "../Dropdown";
import { defineExtension, type Action, type IconSpec, type Shortcut, type ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { BackButton } from "../ListView";
import { copyAndClose, frontmostApp, pasteTitle, type AppInfo } from "../output";
import { TipButton } from "../Tooltip";
import {
  AUTO,
  countLabel,
  DEBOUNCE_MS,
  LANGUAGES,
  languageLabel,
  restoreLanguages,
  sameLanguage,
  swapLanguages,
  type LanguagePair,
} from "../translate";

interface Translation {
  text: string;
  detectedSourceLanguage: string | null;
}

interface Result {
  text: string;
  translated: string;
  pair: LanguagePair;
  detected?: string;
}

const ICON: IconSpec = { glyph: Languages, tone: "blue" };
const STORE = "translate";
const SWAP: Shortcut = { key: "s", ctrl: true };
const flag = <Flag aria-hidden="true" className="size-4 shrink-0" />;
const TARGETS: DropdownOption<string>[] = LANGUAGES.map((l) => ({ ...l, icon: flag }));

function Translate({ pop }: ViewProps) {
  const [pair, setPair] = useState<LanguagePair | null>(null);
  const [text, setText] = useState("");
  const [result, setResult] = useState<Result | null>(null);
  const [pending, setPending] = useState(false);
  const [app, setApp] = useState<AppInfo | null>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const seq = useRef(0);
  const host = useActionHost(() => inputRef.current?.focus());

  useEffect(() => {
    invoke<unknown>("data_get", { name: STORE })
      .then((stored) => setPair(restoreLanguages(stored)))
      .catch(() => setPair(restoreLanguages(null)));
    frontmostApp().then(setApp);
  }, []);

  const choose = useCallback((next: LanguagePair) => {
    setPair(next);
    invoke("data_set", { name: STORE, value: next }).catch((e) => console.error("could not save translate languages", e));
  }, []);

  useEffect(() => {
    if (!pair) return;
    const id = ++seq.current;
    if (!text.trim()) {
      setResult(null);
      setPending(false);
      return;
    }
    if (sameLanguage(pair)) {
      setResult({ text, translated: text, pair });
      setPending(false);
      return;
    }
    setPending(true);
    const timer = window.setTimeout(() => {
      invoke<Translation>("translate", { text, source: pair.source, target: pair.target })
        .then((t) => id === seq.current && setResult({ text, translated: t.text, pair, detected: t.detectedSourceLanguage ?? undefined }))
        .catch((e) => id === seq.current && fail("Translation failed", e))
        .finally(() => id === seq.current && setPending(false));
    }, DEBOUNCE_MS);
    return () => window.clearTimeout(timer);
  }, [text, pair]);

  const translation = result && text.trim() ? result.translated : "";
  const detected = pair?.source === AUTO && result?.pair.source === AUTO ? result.detected : undefined;
  const swapped = pair && (pair.source !== AUTO || result?.text === text) ? swapLanguages(pair, detected) : null;

  const swap = useCallback(() => {
    if (!swapped) return;
    if (translation) {
      setText(translation);
      setResult(result && { text: translation, translated: result.text, pair: swapped });
    }
    choose(swapped);
    inputRef.current?.focus();
  }, [swapped, translation, result, choose]);

  const sources = useMemo<DropdownOption<string>[]>(
    () => [
      {
        value: AUTO,
        label: detected ? `${languageLabel(detected)} (Detected)` : "Detect Language",
        icon: <WandSparkles aria-hidden="true" className="size-4 shrink-0" />,
      },
      ...TARGETS,
    ],
    [detected],
  );

  const groups = useMemo<Action[][]>(() => {
    if (!pair) return [];
    const pick = (side: keyof LanguagePair, options: DropdownOption<string>[]): Action[] =>
      options.map((o) => ({ id: o.value, title: o.label, accessory: o.value === pair[side] ? "Selected" : undefined, run: () => choose({ ...pair, [side]: o.value }), keepOpen: true }));
    return [
      [
        { id: "copy", title: "Copy Translation", icon: Clipboard, shortcut: { key: "Enter", ctrl: true }, disabled: !translation, run: () => copyAndClose(translation, "Copied translation to clipboard") },
        { id: "paste", title: pasteTitle(app), icon: ClipboardPaste, shortcut: { key: "Enter", ctrl: true, alt: true }, disabled: !translation, run: () => invoke("paste_text", { text: translation }) },
        { id: "copySource", title: "Copy Source Text", icon: Clipboard, shortcut: { key: "Enter", ctrl: true, shift: true }, disabled: !text, run: () => copyAndClose(text, "Copied source text to clipboard") },
      ],
      [
        { id: "swap", title: "Swap Languages", icon: ArrowRightLeft, shortcut: SWAP, disabled: !swapped, keepOpen: true, run: swap },
        { id: "source", title: "Change Source Language…", icon: Flag, shortcut: { key: "p", ctrl: true, shift: true }, keepOpen: true, run: () => {}, submenu: () => pick("source", sources) },
        { id: "target", title: "Change Target Language…", icon: Flag, shortcut: { key: "p", ctrl: true }, keepOpen: true, run: () => {}, submenu: () => pick("target", TARGETS) },
      ],
    ];
  }, [pair, translation, text, app, swapped, swap, sources, choose]);

  function runAction(action: Action) {
    if (!action.disabled) host.run(action, { failTitle: `${action.title.replace(/…$/, "")} failed` });
  }

  function onKeyDown(event: React.KeyboardEvent) {
    if (event.defaultPrevented || host.panelOpen) return;
    if (matchesShortcut(event, { key: "k", ctrl: true })) {
      event.preventDefault();
      host.openPanel("Translate", groups);
      return;
    }
    const action = groups.flat().find((a) => a.shortcut && matchesShortcut(event, a.shortcut));
    if (action) {
      event.preventDefault();
      runAction(action);
    } else if (event.key === "Escape" || (event.key === "Backspace" && event.target === inputRef.current && !text)) {
      event.preventDefault();
      pop();
    }
  }

  if (!pair) return null;
  const primary = groups[0][0];

  return (
    <div onKeyDown={onKeyDown} className="relative flex min-h-0 flex-1 flex-col">
      <header className="absolute inset-x-0 top-0 z-10 grid h-[54px] grid-cols-2 items-center">
        <span id="translate-source" className="sr-only">
          Source language
        </span>
        <span id="translate-target" className="sr-only">
          Target language
        </span>
        <div className="absolute left-search-inset">
          <BackButton onBack={pop} />
        </div>
        <div className="flex justify-center">
          <Dropdown labelledBy="translate-source" value={pair.source} options={sources} onChange={(source) => choose({ ...pair, source })} />
        </div>
        <div className="flex justify-center">
          <Dropdown labelledBy="translate-target" value={pair.target} options={TARGETS} onChange={(target) => choose({ ...pair, target })} />
        </div>
        <Loader active={pending || host.busy} />
      </header>

      <div className="relative grid min-h-0 flex-1 grid-cols-2 p-2">
        <div className="relative flex min-h-0 flex-col gap-4 rounded-[2px] p-2 pt-10 before:absolute before:inset-0 before:rounded-[inherit] before:bg-list-hover before:[mask-image:radial-gradient(circle_22px_at_100%_50%,transparent_22px,black_22px)]">
          <textarea
            ref={inputRef}
            autoFocus
            spellCheck={false}
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder="Speak now"
            aria-label="Text to translate"
            lang={pair.source === AUTO ? detected : pair.source}
            className="relative min-h-0 flex-1 resize-none bg-transparent py-2 pr-[46px] pl-6 text-[24px] leading-[1.25] text-fg caret-fg outline-none placeholder:text-fg-40"
          />
          <p aria-live="polite" className="relative flex h-7 shrink-0 items-center justify-end pr-3.5 text-12 text-fg-40 tabular-nums">
            {text && countLabel(text)}
          </p>
        </div>
        <div className="flex min-h-0 flex-col gap-4 p-2 pt-10">
          <output
            aria-label="Translation"
            aria-busy={pending}
            lang={pair.target}
            className="min-h-0 flex-1 overflow-y-auto py-2 pr-[22px] pl-[38px] text-[24px] leading-[1.25] break-words whitespace-pre-wrap text-fg select-text"
          >
            {translation || pending ? `${translation}${pending ? "…" : ""}` : <span className="text-fg-40">Translation</span>}
          </output>
          <span className="h-7 shrink-0" />
        </div>
        <TipButton
          label="Swap languages"
          shortcut={SWAP}
          onClick={swap}
          className={`absolute top-1/2 left-1/2 z-10 flex size-9 -translate-1/2 items-center justify-center rounded-full bg-list-hover text-fg ${swapped ? "hover:bg-control" : "opacity-40"}`}
        >
          <ArrowRightLeft aria-hidden="true" className="size-4" />
        </TipButton>
      </div>

      <Footer
        left={
          <span className="flex min-w-0 items-center gap-2 text-fg-60">
            <GlyphTile spec={ICON} small />
            <span className="truncate">Translator</span>
          </span>
        }
        primaryTitle={primary.title}
        primaryKeys={["Ctrl", "↵"]}
        panelOpen={host.panelOpen}
        onTogglePanel={() => (host.panelOpen ? host.closePanel() : host.openPanel("Translate", groups))}
      />
      {host.render(runAction)}
    </div>
  );
}

export default defineExtension({
  id: "translator",
  title: "Translator",
  icon: ICON,
  commands: [
    {
      id: "translate",
      title: "Translate",
      subtitle: "Translator",
      keywords: ["translate", "translator", "language", "google"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: Translate,
    },
  ],
});
