import { describe, expect, it } from "vitest";
import { displayName, initials, matches, sectionsFor, type Person } from "./contacts";

const person = (name: string, extra: Partial<Person> = {}): Person => ({
  id: name,
  name,
  givenName: name.split(" ")[0] ?? "",
  familyName: name.split(" ").slice(1).join(" "),
  organization: "",
  phoneNumbers: [],
  emailAddresses: [],
  postalAddresses: [],
  ...extra,
});

describe("contacts", () => {
  it("matches names without accents, emails, organizations and phone digits", () => {
    const joao = person("João Silva", { organization: "Acme", emailAddresses: [{ label: "Work", value: "joao@acme.com" }], phoneNumbers: [{ label: "Mobile", value: "+55 (11) 98765-4321" }] });
    expect(matches(joao, "joao")).toBe(true);
    expect(matches(joao, "silva acme")).toBe(true);
    expect(matches(joao, "joao@acme")).toBe(true);
    expect(matches(joao, "98765")).toBe(true);
    expect(matches(joao, "011987")).toBe(true);
    expect(matches(joao, "maria")).toBe(false);
  });

  it("groups by family name initial with # last", () => {
    const sections = sectionsFor([person("Zoe Adams"), person("Ana Brito"), person("Carl Ávila"), person("", { organization: "123 Plumbing" })], "");
    expect(sections.map((s) => s.title)).toEqual(["A", "B", "#"]);
    expect(sections[0].people.map(displayName)).toEqual(["Carl Ávila", "Zoe Adams"]);
  });

  it("builds initials and falls back to the organization", () => {
    expect(initials(person("Ana Maria Brito"))).toBe("AB");
    expect(initials(person("", { organization: "Acme" }))).toBe("A");
    expect(displayName(person("", { organization: "Acme" }))).toBe("Acme");
    expect(displayName(person(""))).toBe("No Name");
  });
});
