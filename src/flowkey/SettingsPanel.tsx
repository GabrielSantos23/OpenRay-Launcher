import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import type { ExtensionManifest, PreferenceSchema } from "@flowkey-cli/native-sdk";
import { useEffect, useState } from "react";
import { Dropdown as Select } from "../Dropdown";
import { Card, Row, Switch } from "../controls";
import { t } from "../i18n";
import { ask } from "./dialogs";
import { CHANGED_EVENT, describeGrants, forgetCached, grantConsent, grantsFor, listExtensions, manifestIssues, newerVersion, pendingConsent, preferencesOf, REMOVED_EVENT, setPreferences, dataSet, type Failure } from "./extensions";
import { grantsOf, NO_GRANTS } from "./gate";
import type { InstalledExtension } from "./runtime";

const BUTTON = "h-6 shrink-0 rounded-[5px] bg-control px-2.5 font-medium text-fg outline-offset-2 hover:bg-control-hover disabled:text-fg-40";
const FIELD = "h-6 w-[220px] rounded-[5px] bg-field px-2 text-fg outline-offset-2 placeholder:text-fg-40";

function message(error: unknown): string {
  return (error as { message?: string })?.message ?? String(error);
}

export function FlowKeyInstallCard({ failures, onChanged }: { failures: Failure[]; onChanged: () => void }) {
  const [status, setStatus] = useState<{ text: string; error: boolean } | null>(null);
  const [busy, setBusy] = useState(false);
  const [folder, setFolder] = useState("");
  useEffect(() => {
    invoke<string>("flowkey_extensions_dir").then(setFolder, () => setFolder(""));
  }, []);

  async function install() {
    setStatus(null);
    const [path] = await invoke<string[]>("flowkey_pick_path", { directory: false, multiple: false, extensions: ["flowkey", "zip"] });
    if (!path) return;
    setBusy(true);
    try {
      const manifest = await invoke<ExtensionManifest>("flowkey_package_manifest", { path });
      const issue = manifestIssues(manifest);
      if (issue) throw new Error(`This package is not a valid FlowKey extension: ${issue}`);
      const current = (await listExtensions()).extensions.find((e) => e.manifest.id === manifest.id && e.source === "installed");
      if (current && !newerVersion(manifest.version, current.manifest.version)) throw new Error(`${manifest.name} ${current.manifest.version} is already installed. Only a newer version can replace it.`);
      const allowed = await ask({
        title: t("Install “{name}” {version}?", { name: manifest.name, version: manifest.version }),
        message: describeGrants(grantsOf(manifest)).length ? t("It will be allowed to:") : t("It asks for no permissions."),
        items: describeGrants(grantsOf(manifest)),
        confirmTitle: t("Install"),
        cancelTitle: t("Cancel"),
        destructive: false,
      });
      if (!allowed) return;
      await invoke("flowkey_install", { path, id: manifest.id });
      await grantConsent(manifest);
      setStatus({ text: t("Installed {name} {version}.", { name: manifest.name, version: manifest.version }), error: false });
      onChanged();
    } catch (error) {
      setStatus({ text: t(message(error)), error: true });
    } finally {
      setBusy(false);
    }
  }

  return (
    <Card>
      <Row id="flowkey-install" title={t("Install Extension")} description={t("Add a .flowkey package built with the FlowKey CLI.")}>
        <button type="button" disabled={busy} onClick={install} className={BUTTON}>
          {t("Install from File…")}
        </button>
      </Row>
      <Row id="flowkey-folder" title={t("Extensions Folder")} description={folder || t("Unavailable")}>
        <button type="button" disabled={!folder} onClick={() => invoke("open_url", { url: folder }).catch((e) => setStatus({ text: t(message(e)), error: true }))} className={BUTTON}>
          {t("Open Folder")}
        </button>
      </Row>
      {failures.map((failure) => (
        <Row key={`${failure.source}:${failure.id}`} id={`flowkey-failure-${failure.id}`} title={t("{id} could not load", { id: failure.id })} description={t(failure.message)}>
          <span className="text-12 text-red">{failure.source === "bundled" ? t("Bundled") : t("Installed")}</span>
        </Row>
      ))}
      <p aria-live="polite" className={`px-3 text-12 empty:hidden ${status?.error ? "pb-2 text-red" : "pb-2 text-fg-60"}`}>
        {status?.text}
      </p>
    </Card>
  );
}

function PreferenceInput({ schema, value, onChange, id }: { schema: PreferenceSchema; value: unknown; onChange: (value: unknown) => void; id: string }) {
  const [apps, setApps] = useState<{ name: string; path: string }[]>([]);
  useEffect(() => {
    if (schema.type === "appPicker") invoke<{ name: string; path: string }[]>("list_apps").then(setApps, () => setApps([]));
  }, [schema.type]);
  if (schema.type === "checkbox") {
    return (
      <span className="flex items-center gap-2">
        {schema.label && <span className="text-fg-60">{schema.label}</span>}
        <Switch checked={value === true} onChange={onChange} labelledBy={id} />
      </span>
    );
  }
  if (schema.type === "dropdown" || schema.type === "appPicker") {
    const options = schema.type === "dropdown" ? (schema.options ?? []).map((o) => ({ value: o.value, label: o.title })) : apps.map((a) => ({ value: a.path, label: a.name }));
    const current = typeof value === "string" ? value : "";
    const withEmpty = options.some((o) => o.value === current) ? options : [{ value: current, label: current ? current : t("Choose…") }, ...options];
    return <Select labelledBy={id} size="small" value={current} options={withEmpty} onChange={onChange} />;
  }
  const browse = schema.type === "file" || schema.type === "directory";
  return (
    <span className="flex items-center gap-2">
      <input
        key={String(value ?? "")}
        aria-labelledby={id}
        type={schema.type === "password" ? "password" : "text"}
        defaultValue={typeof value === "string" ? value : ""}
        placeholder={schema.placeholder}
        spellCheck={false}
        onKeyDown={(e) => e.key === "Enter" && e.currentTarget.blur()}
        onBlur={(e) => e.target.value !== (value ?? "") && onChange(e.target.value)}
        className={FIELD}
      />
      {browse && (
        <button
          type="button"
          className={BUTTON}
          onClick={async () => {
            const [picked] = await invoke<string[]>("flowkey_pick_path", { directory: schema.type === "directory", multiple: false });
            if (picked) onChange(picked);
          }}
        >
          {t("Browse…")}
        </button>
      )}
    </span>
  );
}

export function FlowKeyDetails({ extension, onChanged }: { extension: InstalledExtension; onChanged: () => void }) {
  const { manifest } = extension;
  const [values, setValues] = useState<Record<string, unknown> | null>(null);
  const [granted, setGranted] = useState(NO_GRANTS);
  const [pending, setPending] = useState<string[]>([]);
  const [error, setError] = useState("");

  async function refresh() {
    forgetCached(manifest.id);
    setValues(await preferencesOf(manifest));
    setGranted(await grantsFor(extension));
    const missing = await pendingConsent(extension);
    setPending(missing ? describeGrants(missing) : []);
  }

  useEffect(() => {
    refresh().catch((e) => setError(message(e)));
  }, [extension]);

  async function save(name: string, value: unknown) {
    if (!values) return;
    const next = { ...values, [name]: value };
    setValues(next);
    try {
      await setPreferences(manifest.id, next);
      setError("");
    } catch (e) {
      setError(message(e));
    }
  }

  async function uninstall() {
    const confirmed = await ask({
      title: t("Uninstall “{name}”?", { name: manifest.name }),
      message: t("This deletes the extension and its saved data, secrets and sign-ins from this computer. You can install it again later."),
      confirmTitle: t("Uninstall"),
      cancelTitle: t("Cancel"),
      destructive: true,
    });
    if (!confirmed) return;
    try {
      await invoke("flowkey_uninstall", { id: manifest.id });
      await emit(REMOVED_EVENT, manifest.id);
      onChanged();
    } catch (e) {
      setError(message(e));
    }
  }

  async function revoke() {
    try {
      await dataSet(manifest.id, "consent", NO_GRANTS);
      await emit(CHANGED_EVENT, manifest.id);
      await refresh();
    } catch (e) {
      setError(message(e));
    }
  }

  const grantedList = describeGrants(granted);
  return (
    <>
      <Card>
        <Row id={`fk-${manifest.id}-about`} title={`${manifest.name} ${manifest.version}`} description={manifest.description}>
          <span className="text-12 text-fg-60">{extension.source === "bundled" ? t("Bundled with Openray") : t("Installed")}</span>
        </Row>
        {extension.source === "installed" && (
          <Row id={`fk-${manifest.id}-uninstall`} title={t("Uninstall")} description={extension.dir}>
            <button type="button" onClick={uninstall} className={`${BUTTON} text-destructive`}>
              {t("Uninstall…")}
            </button>
          </Row>
        )}
      </Card>
      {(manifest.preferences?.length ?? 0) > 0 && values && (
        <Card>
          {manifest.preferences!.map((schema) => {
            const id = `fk-${manifest.id}-pref-${schema.name}`;
            return (
              <Row key={schema.name} id={id} title={schema.required ? t("{title} (required)", { title: schema.title }) : schema.title} description={schema.description}>
                <PreferenceInput id={id} schema={schema} value={values[schema.name]} onChange={(value) => save(schema.name, value)} />
              </Row>
            );
          })}
        </Card>
      )}
      <Card>
        <Row id={`fk-${manifest.id}-permissions`} title={t("Permissions")} description={grantedList.length ? grantedList.map((g) => t(g)).join(" · ") : t("No permissions granted")}>
          {pending.length > 0 && (
            <button
              type="button"
              className={BUTTON}
              onClick={async () => {
                await grantConsent(manifest);
                await refresh();
              }}
            >
              {t("Allow {count} Pending", { count: pending.length })}
            </button>
          )}
          {extension.source === "installed" && grantedList.length > 0 && (
            <button type="button" className={BUTTON} onClick={revoke}>
              {t("Revoke All")}
            </button>
          )}
        </Row>
        {pending.length > 0 && <Row id={`fk-${manifest.id}-pending`} title={t("Waiting for Approval")} description={pending.map((p) => t(p)).join(" · ")}>{null}</Row>}
      </Card>
      {error && (
        <p role="alert" className="px-3 text-12 text-red">
          {error}
        </p>
      )}
    </>
  );
}
