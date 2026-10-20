const SCHEME = /^[a-z][a-z0-9+.-]*:\/\//i;
const FILE_EXTENSIONS = new Set(["exe", "msc", "cpl", "bat", "cmd", "ps1", "lnk", "txt", "json", "js", "ts", "tsx", "py", "rs", "png", "jpg", "jpeg", "gif", "pdf", "docx", "xlsx", "zip", "dll", "log", "ini", "csv", "html"]);
const HOST = /^(localhost|\d{1,3}(\.\d{1,3}){3}|([a-z0-9-]+\.)+[a-z]{2,63})(:\d{1,5})?([/?#]\S*)?$/i;

export function toUrl(text: string): string | null {
  const value = text.trim();
  if (!value || /\s/.test(value)) return null;
  if (SCHEME.test(value)) {
    try {
      return new URL(value).host || value.startsWith("file:") ? value : null;
    } catch {
      return null;
    }
  }
  const host = value.split(/[/:?#]/)[0];
  if (!HOST.test(value) || (!value.includes("/") && FILE_EXTENSIONS.has(host.split(".").pop()!.toLowerCase()))) return null;
  const local = /^(localhost|\d{1,3}(\.\d{1,3}){3})/.test(value);
  return `${local ? "http" : "https"}://${value}`;
}
