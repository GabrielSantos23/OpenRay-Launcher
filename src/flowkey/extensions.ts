import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { validateManifest, type ExtensionManifest, type ManifestCommand, type PreferenceSchema } from "@flowkey-cli/native-sdk";
import { grantsOf, intersect, isEmpty, missingConsent, NO_GRANTS, type Grants } from "./gate";
import type { InstalledExtension } from "./runtime";

interface Listed {
  source: "installed" | "bundled";
  name: string;
  dir: string;
  manifest: unknown;
  error?: string;
}

export interface Failure {
  id: string;
  source: "installed" | "bundled";
  message: string;
}

export const FLOWKEY_ORIGIN = "http://flowkey.localhost/";
export const CHANGED_EVENT = "flowkey-changed";
export const REMOVED_EVENT = "flowkey-removed";

export function isNotchCommand(command: ManifestCommand): boolean {
  return command.notch === true && command.mode !== "background" && command.ui !== "web";
}

export function manifestIssues(manifest: unknown, folder?: string): string | null {
  const { errors } = validateManifest(manifest);
  if (errors.length) return errors.map((e) => `${e.field}: ${e.message}`).join("; ");
  const id = (manifest as ExtensionManifest).id;
  if (folder !== undefined && folder !== id) return `manifest id '${id}' does not match its folder '${folder}'`;
  return null;
}

export async function listExtensions(): Promise<{ extensions: InstalledExtension[]; failures: Failure[] }> {
  const listed = await invoke<Listed[]>("flowkey_list");
  const byId = new Map<string, InstalledExtension>();
  const failures: Failure[] = [];
  for (const entry of listed) {
    const issue = entry.error ?? manifestIssues(entry.manifest, entry.name);
    if (issue) {
      failures.push({ id: entry.name, source: entry.source, message: issue });
      continue;
    }
    const manifest = entry.manifest as ExtensionManifest;
    if (byId.get(manifest.id)?.source === "installed") continue;
    byId.set(manifest.id, { manifest, source: entry.source, dir: entry.dir, baseUrl: `${FLOWKEY_ORIGIN}${entry.source}/${encodeURIComponent(manifest.id)}/` });
  }
  return { extensions: [...byId.values()].sort((a, b) => a.manifest.name.localeCompare(b.manifest.name)), failures };
}

export function dataGet<T>(id: string, name: string): Promise<T | null> {
  return invoke<T | null>("flowkey_data_get", { extensionId: id, name });
}

export function dataSet(id: string, name: string, value: unknown): Promise<void> {
  return invoke("flowkey_data_set", { extensionId: id, name, value });
}

const consentCache = new Map<string, Grants | null>();

export async function consentOf(extension: InstalledExtension): Promise<Grants | null> {
  if (extension.source === "bundled") return grantsOf(extension.manifest);
  const id = extension.manifest.id;
  if (!consentCache.has(id)) consentCache.set(id, await dataGet<Grants>(id, "consent"));
  return consentCache.get(id) ?? null;
}

export async function grantsFor(extension: InstalledExtension): Promise<Grants> {
  const consent = await consentOf(extension);
  return consent ? intersect(grantsOf(extension.manifest), { ...NO_GRANTS, ...consent }) : NO_GRANTS;
}

export async function pendingConsent(extension: InstalledExtension): Promise<Grants | null> {
  if (extension.source === "bundled") return null;
  const missing = missingConsent(grantsOf(extension.manifest), (await consentOf(extension)) ?? undefined);
  return isEmpty(missing) ? null : missing;
}

export async function grantConsent(manifest: ExtensionManifest) {
  const grants = grantsOf(manifest);
  consentCache.set(manifest.id, grants);
  await dataSet(manifest.id, "consent", grants);
  await emit(CHANGED_EVENT, manifest.id);
}

export function forgetCached(id: string) {
  consentCache.delete(id);
  preferenceCache.delete(id);
}

const preferenceCache = new Map<string, Record<string, unknown>>();

export function preferenceDefaults(schema: PreferenceSchema[] = []): Record<string, unknown> {
  return Object.fromEntries(schema.filter((p) => p.default !== undefined).map((p) => [p.name, p.default]));
}

export async function storedPreferences(id: string): Promise<Record<string, unknown>> {
  if (!preferenceCache.has(id)) preferenceCache.set(id, (await dataGet<Record<string, unknown>>(id, "preferences")) ?? {});
  return preferenceCache.get(id)!;
}

export async function preferencesOf(manifest: ExtensionManifest): Promise<Record<string, unknown>> {
  return { ...preferenceDefaults(manifest.preferences), ...(await storedPreferences(manifest.id)) };
}

export async function setPreferences(id: string, values: Record<string, unknown>) {
  preferenceCache.set(id, values);
  await dataSet(id, "preferences", values);
  await emit(CHANGED_EVENT, id);
}

export function missingPreferences(manifest: ExtensionManifest, values: Record<string, unknown>): PreferenceSchema[] {
  return (manifest.preferences ?? []).filter((p) => p.required && p.type !== "checkbox" && (values[p.name] === undefined || values[p.name] === ""));
}

export function describeGrants(grants: Grants): string[] {
  return [
    ...grants.nativeMethods.map((m) => `Use ${m}`),
    ...grants.httpHosts.map((h) => `Connect to ${h.startsWith(".") ? `*${h}` : h}`),
    ...grants.oauth.map((p) => `Sign in to ${p.charAt(0).toUpperCase()}${p.slice(1)}`),
    ...grants.fsPaths.map((p) => `Files in ${p}`),
    ...grants.uriSchemes.map((s) => `Open ${s}: links in their app`),
  ];
}

export function newerVersion(next: string, current: string): boolean {
  const parse = (v: string) => {
    const [core, pre] = v.split("+")[0].split(/-(.*)/s);
    return { nums: core.split(".").map(Number), pre };
  };
  const a = parse(next);
  const b = parse(current);
  for (let i = 0; i < 3; i++) if (a.nums[i] !== b.nums[i]) return a.nums[i] > b.nums[i];
  if (a.pre === b.pre) return false;
  if (!a.pre) return true;
  if (!b.pre) return false;
  return a.pre.localeCompare(b.pre, undefined, { numeric: true }) > 0;
}
