export interface Labeled {
  label: string;
  value: string;
}

export interface Person {
  id: string;
  name: string;
  givenName: string;
  familyName: string;
  organization: string;
  phoneNumbers: Labeled[];
  emailAddresses: Labeled[];
  postalAddresses: Labeled[];
}

export interface PersonSection {
  title: string;
  people: Person[];
}

export function displayName(person: Person): string {
  return person.name || person.organization || "No Name";
}

export function initials(person: Person): string {
  const words = (person.name || person.organization).trim().split(/\s+/).filter(Boolean);
  if (!words.length) return "";
  return (words.length === 1 ? words[0][0] : words[0][0] + words[words.length - 1][0]).toUpperCase();
}

function fold(text: string): string {
  return text.normalize("NFD").replace(/[̀-ͯ]/g, "").toLowerCase();
}

function digits(text: string): string {
  return text.replace(/\D/g, "");
}

function phoneMatches(phone: string, term: string): boolean {
  const wanted = digits(term);
  if (!wanted) return false;
  const have = digits(phone);
  return have.includes(wanted) || (wanted.startsWith("0") && wanted.length > 1 && have.includes(wanted.slice(1)));
}

export function matches(person: Person, query: string): boolean {
  const terms = fold(query.trim()).split(/\s+/).filter(Boolean);
  const name = fold(person.name);
  const organization = fold(person.organization);
  const emails = person.emailAddresses.map((e) => fold(e.value));
  const addresses = person.postalAddresses.map((a) => fold(a.value));
  return terms.every(
    (t) => name.includes(t) || organization.includes(t) || emails.some((e) => e.includes(t)) || addresses.some((a) => a.includes(t)) || person.phoneNumbers.some((p) => phoneMatches(p.value, t)),
  );
}

export function sectionsFor(people: Person[], query: string): PersonSection[] {
  const groups = new Map<string, Person[]>();
  for (const person of people.filter((p) => matches(p, query))) {
    const key = fold(person.familyName || person.name.split(" ").at(-1) || person.organization)[0]?.toUpperCase() ?? "#";
    const letter = /[A-Z]/.test(key) ? key : "#";
    groups.set(letter, [...(groups.get(letter) ?? []), person]);
  }
  return [...groups.keys()]
    .sort((a, b) => (a === "#" ? 1 : b === "#" ? -1 : a.localeCompare(b)))
    .map((title) => ({ title, people: groups.get(title)!.sort((a, b) => displayName(a).localeCompare(displayName(b))) }));
}
