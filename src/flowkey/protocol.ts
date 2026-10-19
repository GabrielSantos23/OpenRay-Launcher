import type { Arguments, ExtensionEnvironment, ExtensionManifest, UiItem, UiTree } from "@flowkey-cli/native-sdk";

export interface ProtocolError {
  code: string;
  message: string;
}

export type ToWorker =
  | { type: "load"; requestId: string; extensionId: string; url: string; manifest: ExtensionManifest; preferences: Record<string, unknown>; isDevelopment: boolean }
  | { type: "preferences"; extensionId: string; values: Record<string, unknown> }
  | { type: "render"; viewId: string; extensionId: string; commandId: string; query: string; filterValue?: string; arguments?: Arguments; reset?: boolean }
  | { type: "action"; requestId: string; viewId: string; actionId: string; item?: UiItem; formValues?: Record<string, unknown> }
  | { type: "background"; requestId: string; extensionId: string; commandId: string; arguments?: Arguments }
  | { type: "close"; viewId: string }
  | { type: "nativeResult"; requestId: string; ok: true; result?: unknown }
  | { type: "nativeResult"; requestId: string; ok: false; error: ProtocolError };

export type FromWorker =
  | { type: "loaded"; requestId: string; ok: true; kind: "react" | "functional" }
  | { type: "loaded"; requestId: string; ok: false; error: ProtocolError }
  | { type: "tree"; viewId: string; tree: UiTree }
  | { type: "viewError"; viewId: string; error: ProtocolError }
  | { type: "done"; requestId: string; ok: true }
  | { type: "done"; requestId: string; ok: false; error: ProtocolError }
  | { type: "nativeCall"; requestId: string; extensionId: string; viewId?: string; method: string; params?: Record<string, unknown> }
  | { type: "nativeAbort"; requestId: string }
  | { type: "windowCommand"; extensionId: string; viewId?: string; command: "closeMainWindow" | "popToRoot" | "clearSearchBar" }
  | { type: "launchCommand"; extensionId: string; commandId: string; query?: string }
  | { type: "log"; level: "debug" | "info" | "warn" | "error"; message: string };

export function toProtocolError(error: unknown): ProtocolError {
  if (error && typeof error === "object" && "code" in error && "message" in error) {
    return { code: String((error as ProtocolError).code), message: String((error as ProtocolError).message) };
  }
  return { code: "extensionError", message: error instanceof Error ? error.message : String(error) };
}

export function environmentFor(manifest: ExtensionManifest, isDevelopment: boolean, commandId?: string): ExtensionEnvironment {
  const command = manifest.commands.find((c) => c.id === commandId);
  return {
    extensionId: manifest.id,
    extensionName: manifest.name,
    extensionVersion: manifest.version,
    commandId,
    commandMode: command?.mode ?? (commandId ? "view" : undefined),
    isDevelopment,
  };
}
