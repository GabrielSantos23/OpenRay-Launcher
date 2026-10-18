import { invoke } from "@tauri-apps/api/core";
import { NotebookPen, Plus, SquarePen } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { fail } from "../ActionHost";
import { defineExtension, type IconSpec, type Item, type ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { ListView, Metadata } from "../ListView";
import { browseSections, countLabel, loadNotes, noteTitle, type Note, type NotesRequest } from "../notes";
import { updatedAgo } from "./calculator";

const ICON: IconSpec = { glyph: NotebookPen, tone: "pink" };

function showNotes(request: NotesRequest) {
  return invoke<void>("notes_show", { request });
}

function SearchNotes({ pop }: ViewProps) {
  const [notes, setNotes] = useState<Note[] | null>(null);

  useEffect(() => {
    loadNotes().then(setNotes, (e) => {
      setNotes([]);
      fail("Could not load notes", e);
    });
  }, []);

  const sections = useMemo(
    () =>
      browseSections(notes ?? []).map((section) => ({
        title: section.title,
        items: section.notes.map(
          (note): Item => ({
            id: note.id,
            title: noteTitle(note.text),
            keywords: [note.text],
            actions: [
              [
                { id: "open", title: "Open Note", icon: SquarePen, run: () => showNotes({ id: note.id }) },
                { id: "create", title: "Create Note", icon: Plus, shortcut: { key: "n", ctrl: true }, run: () => showNotes({ create: true }) },
              ],
            ],
          }),
        ),
      })),
    [notes],
  );

  return (
    <ListView
      placeholder="Search for notes..."
      sections={sections}
      onBack={pop}
      footerTitle="Search Notes"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={notes === null}
      emptyText="No notes yet. Press Ctrl+N to create one."
      detail={(item) => {
        const note = notes?.find((n) => n.id === item.id);
        if (!note) return null;
        return (
          <>
            <div className="min-h-0 flex-1 overflow-y-auto p-4 text-13 break-words whitespace-pre-wrap text-fg">{note.text}</div>
            <Metadata
              rows={[
                { label: "Characters", value: countLabel(note.text, "characters") },
                { label: "Words", value: countLabel(note.text, "words") },
                { label: "Last opened", value: updatedAgo(note.lastAccessedAt, Date.now(), "Opened") },
              ]}
            />
          </>
        );
      }}
    />
  );
}

export default defineExtension({
  id: "notes",
  title: "Notes",
  icon: ICON,
  commands: [
    {
      id: "open",
      title: "Openray Notes",
      subtitle: "Notes",
      keywords: ["notes", "scratchpad", "markdown"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      run: () => showNotes({}),
    },
    {
      id: "search",
      title: "Search Notes",
      subtitle: "Notes",
      keywords: ["notes", "find"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: SearchNotes,
    },
    {
      id: "create",
      title: "Create Note",
      subtitle: "Notes",
      keywords: ["notes", "new"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      run: () => showNotes({ create: true }),
    },
  ],
});
