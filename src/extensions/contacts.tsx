import { invoke } from "@tauri-apps/api/core";
import { Clipboard, Mail, MapPin, Phone, User } from "lucide-react";
import { useEffect, useMemo, useState, type ComponentType } from "react";
import { displayName, initials, sectionsFor, type Labeled, type Person } from "../contacts";
import { defineExtension, type Action, type IconSpec, type Shortcut, type ViewProps } from "../extension-api";
import { GlyphTile } from "../icons";
import { ListView, Metadata, type ListSection } from "../ListView";
import { copyAndClose } from "../output";

const ICON: IconSpec = { glyph: User, tone: "slate" };

function Avatar({ person, large = false }: { person: Person; large?: boolean }) {
  return (
    <span aria-hidden="true" className={`flex shrink-0 items-center justify-center rounded-full bg-control font-semibold text-fg-60 ${large ? "size-24 text-[32px]" : "size-6 text-[10px]"}`}>
      {initials(person) || <User className="size-1/2" />}
    </span>
  );
}

function multi(items: Labeled[], title: string, icon: ComponentType<{ className?: string }>, shortcut: Shortcut, run: (value: string) => Promise<void> | void): Action[] {
  if (!items.length) return [];
  const id = title.toLowerCase().replace(/\s+/g, "-");
  if (items.length === 1) return [{ id, title, icon, shortcut, run: () => run(items[0].value) }];
  return [
    {
      id,
      title: `${title}…`,
      icon,
      shortcut,
      keepOpen: true,
      run: () => {},
      submenu: () => items.map((item, i) => ({ id: `${id}-${i}`, title: item.value, icon, accessory: item.label, run: () => run(item.value) })),
    },
  ];
}

function personActions(person: Person): Action[][] {
  const open = (scheme: string) => (value: string) => invoke<void>("open_url", { url: `${scheme}:${scheme === "tel" ? value.replace(/[^\d+]/g, "") : value}`, appPath: null });
  const copied = (what: string) => (value: string) => copyAndClose(value, `Copied ${what}`);
  return [
    [
      ...multi(person.emailAddresses, "Send Email", Mail, { key: "e", ctrl: true, shift: true }, open("mailto")),
      ...multi(person.phoneNumbers, "Call via Phone", Phone, { key: "p", ctrl: true, shift: true }, open("tel")),
    ],
    [
      ...(person.name ? [{ id: "copy-name", title: "Copy Full Name", icon: Clipboard, shortcut: { key: "c", ctrl: true, shift: true }, run: () => copied("name")(person.name) }] : []),
      ...multi(person.phoneNumbers, "Copy Phone Number", Phone, { key: "c", ctrl: true, alt: true }, copied("phone number")),
      ...multi(person.emailAddresses, "Copy Email Address", Mail, { key: "c", alt: true, shift: true }, copied("email address")),
      ...multi(person.postalAddresses, "Copy Postal Address", MapPin, { key: "c", ctrl: true, alt: true, shift: true }, copied("postal address")),
    ],
  ];
}

function ContactDetail({ person }: { person: Person }) {
  const rows = [
    ...person.phoneNumbers.map((p) => ({ label: p.label, value: p.value })),
    ...person.emailAddresses.map((e) => ({ label: e.label, value: e.value })),
    ...person.postalAddresses.map((a) => ({ label: a.label, value: a.value })),
  ];
  return (
    <>
      <div className="flex min-h-0 flex-1 flex-col items-center justify-center gap-3 p-4 text-center">
        <Avatar person={person} large />
        <div className="flex flex-col gap-0.5">
          <span className="text-15 font-semibold text-fg">{displayName(person)}</span>
          {person.organization && person.name && <span className="text-13 text-fg-60">{person.organization}</span>}
        </div>
      </div>
      {rows.length > 0 && <Metadata rows={rows} />}
    </>
  );
}

function SearchContacts({ pop }: ViewProps) {
  const [people, setPeople] = useState<Person[] | null>(null);
  const [error, setError] = useState<string>();
  const [text, setText] = useState("");

  useEffect(() => {
    invoke<Person[]>("contacts_list")
      .then(setPeople)
      .catch((e) => {
        setPeople([]);
        setError(String(e));
      });
  }, []);

  const byId = useMemo(() => new Map((people ?? []).map((p) => [p.id, p])), [people]);
  const sections = useMemo<ListSection[]>(
    () =>
      sectionsFor(people ?? [], text).map((s) => ({
        title: s.title,
        items: s.people.map((p) => ({ id: p.id, title: displayName(p), subtitle: p.name ? p.organization : undefined, icon: <Avatar person={p} />, actions: personActions(p) })),
      })),
    [people, text],
  );

  const empty = error
    ? { text: "Couldn't read your contacts", description: error }
    : people?.length === 0
      ? { text: "No contacts found", description: "Windows has no contacts yet. Add an account with contacts in Settings > Accounts > Email & accounts." }
      : { text: "No Contacts Found", description: "Try another name, email or phone number." };

  return (
    <ListView
      placeholder="Search contacts…"
      sections={sections}
      onBack={pop}
      searchText={text}
      onSearchTextChange={setText}
      filtering={false}
      footerTitle="Search Contacts"
      footerIcon={<GlyphTile spec={ICON} small />}
      isLoading={people === null}
      emptyText={empty.text}
      emptyDescription={empty.description}
      detail={
        sections.length
          ? (item) => {
              const person = byId.get(item.id);
              return person ? <ContactDetail person={person} /> : null;
            }
          : undefined
      }
    />
  );
}

export default defineExtension({
  id: "contacts",
  title: "Contacts",
  icon: ICON,
  commands: [
    {
      id: "search",
      title: "Search Contacts",
      subtitle: "Contacts",
      keywords: ["contacts", "people", "phone", "email", "address book"],
      kind: "Command",
      primaryActionTitle: "Open Command",
      view: SearchContacts,
    },
  ],
});
