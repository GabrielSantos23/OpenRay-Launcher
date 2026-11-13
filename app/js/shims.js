const rust = globalThis.__rust;

globalThis.self = globalThis;

let listener = null;
globalThis.addEventListener = (type, fn) => {
  if (type === "message") listener = fn;
};
globalThis.removeEventListener = () => {};
globalThis.postMessage = (message) => rust.post(JSON.stringify(message));
globalThis.__deliver = (text) => listener?.({ data: JSON.parse(text) });

const timers = new Map();
let nextTimer = 1;
function schedule(fn, ms, args, repeat) {
  const id = nextTimer++;
  const delay = Math.max(repeat ? 1 : 0, Number(ms) || 0);
  timers.set(id, { fn, args, repeat: repeat ? delay : -1 });
  rust.timer(id, delay);
  return id;
}
globalThis.setTimeout = (fn, ms, ...args) => schedule(fn, ms, args, false);
globalThis.setInterval = (fn, ms, ...args) => schedule(fn, ms, args, true);
globalThis.clearTimeout = globalThis.clearInterval = (id) => void timers.delete(id);
globalThis.__fire = (id) => {
  const timer = timers.get(id);
  if (!timer) return;
  if (timer.repeat < 0) timers.delete(id);
  else rust.timer(id, timer.repeat);
  if (typeof timer.fn === "function") timer.fn(...timer.args);
};
globalThis.queueMicrotask ??= (fn) => void Promise.resolve().then(fn);

const started = Date.now();
globalThis.performance ??= { now: () => Date.now() - started };

function describe(value) {
  if (typeof value === "string") return value;
  if (value instanceof Error) return value.stack ? `${value.message}\n${value.stack}` : value.message;
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}
const log = (level) => (...args) => rust.log(level, args.map(describe).join(" "));
globalThis.console = { log: log("info"), info: log("info"), debug: log("debug"), warn: log("warn"), error: log("error"), trace: log("debug") };

class DOMException extends Error {
  constructor(message = "", name = "Error") {
    super(message);
    this.name = name;
  }
}
globalThis.DOMException ??= DOMException;

class EventTarget {
  #listeners = new Map();
  addEventListener(type, fn, options) {
    if (!fn) return;
    const list = this.#listeners.get(type) ?? [];
    if (!list.some((l) => l.fn === fn)) list.push({ fn, once: options === true ? false : !!options?.once });
    this.#listeners.set(type, list);
  }
  removeEventListener(type, fn) {
    this.#listeners.set(type, (this.#listeners.get(type) ?? []).filter((l) => l.fn !== fn));
  }
  dispatchEvent(event) {
    for (const l of [...(this.#listeners.get(event.type) ?? [])]) {
      if (l.once) this.removeEventListener(event.type, l.fn);
      typeof l.fn === "function" ? l.fn.call(this, event) : l.fn.handleEvent(event);
    }
    return true;
  }
}
globalThis.EventTarget ??= EventTarget;

class AbortSignal extends EventTarget {
  aborted = false;
  reason = undefined;
  onabort = null;
  throwIfAborted() {
    if (this.aborted) throw this.reason;
  }
  static abort(reason) {
    const controller = new AbortController();
    controller.abort(reason);
    return controller.signal;
  }
  static timeout(ms) {
    const controller = new AbortController();
    setTimeout(() => controller.abort(new DOMException("signal timed out", "TimeoutError")), ms);
    return controller.signal;
  }
  static any(signals) {
    const controller = new AbortController();
    for (const signal of signals) {
      if (signal.aborted) {
        controller.abort(signal.reason);
        break;
      }
      signal.addEventListener("abort", () => controller.abort(signal.reason), { once: true });
    }
    return controller.signal;
  }
}

class AbortController {
  signal = new AbortSignal();
  abort(reason) {
    const signal = this.signal;
    if (signal.aborted) return;
    signal.aborted = true;
    signal.reason = reason ?? new DOMException("This operation was aborted", "AbortError");
    const event = { type: "abort", target: signal };
    signal.onabort?.(event);
    signal.dispatchEvent(event);
  }
}
globalThis.AbortSignal = AbortSignal;
globalThis.AbortController = AbortController;

const encodeForm = (text) => encodeURIComponent(text).replace(/[!'()~]/g, (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`).replace(/%20/g, "+");
const decodeForm = (text) => {
  try {
    return decodeURIComponent(text.replace(/\+/g, " "));
  } catch {
    return text;
  }
};

class URLSearchParams {
  #pairs = [];
  #onChange = null;
  constructor(init, onChange) {
    if (typeof init === "string") this.#parse(init);
    else if (init && typeof init[Symbol.iterator] === "function") for (const [k, v] of init) this.#pairs.push([String(k), String(v)]);
    else if (init) for (const k of Object.keys(init)) this.#pairs.push([k, String(init[k])]);
    this.#onChange = onChange ?? null;
  }
  #parse(text) {
    this.#pairs = [];
    for (const part of text.replace(/^\?/, "").split("&")) {
      if (!part) continue;
      const i = part.indexOf("=");
      this.#pairs.push([decodeForm(i < 0 ? part : part.slice(0, i)), decodeForm(i < 0 ? "" : part.slice(i + 1))]);
    }
  }
  _reset(text) {
    this.#parse(text);
  }
  #changed() {
    this.#onChange?.();
  }
  get size() {
    return this.#pairs.length;
  }
  append(k, v) {
    this.#pairs.push([String(k), String(v)]);
    this.#changed();
  }
  set(k, v) {
    const i = this.#pairs.findIndex(([key]) => key === k);
    if (i < 0) this.#pairs.push([String(k), String(v)]);
    else this.#pairs = this.#pairs.filter(([key], j) => j === i || key !== k).map((p, j) => (j === i ? [p[0], String(v)] : p));
    this.#changed();
  }
  get(k) {
    return this.#pairs.find(([key]) => key === k)?.[1] ?? null;
  }
  getAll(k) {
    return this.#pairs.filter(([key]) => key === k).map(([, v]) => v);
  }
  has(k) {
    return this.#pairs.some(([key]) => key === k);
  }
  delete(k) {
    this.#pairs = this.#pairs.filter(([key]) => key !== k);
    this.#changed();
  }
  sort() {
    this.#pairs.sort((a, b) => (a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0));
    this.#changed();
  }
  forEach(fn, self) {
    for (const [k, v] of this.#pairs) fn.call(self, v, k, this);
  }
  *entries() {
    yield* this.#pairs.map((p) => [...p]);
  }
  *keys() {
    for (const [k] of this.#pairs) yield k;
  }
  *values() {
    for (const [, v] of this.#pairs) yield v;
  }
  [Symbol.iterator]() {
    return this.entries();
  }
  toString() {
    return this.#pairs.map(([k, v]) => `${encodeForm(k)}=${encodeForm(v)}`).join("&");
  }
}

const DEFAULT_PORTS = { "http:": "80", "https:": "443", "ws:": "80", "wss:": "443", "ftp:": "21" };
const PATTERN = /^([a-z][a-z0-9+.-]*:)(?:\/\/(?:([^:@/?#]*)(?::([^@/?#]*))?@)?(\[[^\]]*\]|[^:/?#]*)(?::(\d*))?)?([^?#]*)(\?[^#]*)?(#.*)?$/i;

function normalizePath(path) {
  const out = [];
  for (const segment of path.split("/")) {
    if (segment === "..") out.length > 1 && out.pop();
    else if (segment !== ".") out.push(segment);
  }
  if (/\/\.\.?$/.test(path)) out.push("");
  return out.join("/") || "/";
}

class URL {
  #raw = "";
  #params = null;
  constructor(input, base) {
    let text = String(input).trim();
    if (!/^[a-z][a-z0-9+.-]*:/i.test(text)) {
      if (base === undefined) throw new TypeError(`Invalid URL: ${text}`);
      const b = new URL(base);
      if (text.startsWith("//")) text = `${b.protocol}${text}`;
      else if (text.startsWith("/")) text = `${b.origin}${text}`;
      else if (text.startsWith("?")) text = `${b.origin}${b.pathname}${text}`;
      else if (text.startsWith("#")) text = `${b.origin}${b.pathname}${b.search}${text}`;
      else text = `${b.origin}${b.pathname.replace(/[^/]*$/, "")}${text}`;
    }
    const m = PATTERN.exec(text);
    if (!m) throw new TypeError(`Invalid URL: ${text}`);
    this.protocol = m[1].toLowerCase();
    this.username = m[2] ?? "";
    this.password = m[3] ?? "";
    this.hostname = (m[4] ?? "").toLowerCase();
    this.port = m[5] && m[5] !== DEFAULT_PORTS[this.protocol] ? m[5] : "";
    const special = this.protocol in DEFAULT_PORTS;
    if (special && !this.hostname) throw new TypeError(`Invalid URL: ${text}`);
    this.pathname = special ? normalizePath(m[6] || "/") : m[6];
    this.#raw = m[7] && m[7] !== "?" ? m[7] : "";
    this.hash = m[8] && m[8] !== "#" ? m[8] : "";
  }
  get searchParams() {
    this.#params ??= new URLSearchParams(this.#raw, () => {
      const s = this.#params.toString();
      this.#raw = s ? `?${s}` : "";
    });
    return this.#params;
  }
  get search() {
    return this.#raw;
  }
  set search(value) {
    const text = String(value);
    this.#raw = text && text !== "?" ? (text.startsWith("?") ? text : `?${text}`) : "";
    this.#params?._reset(this.#raw);
  }
  get host() {
    return this.port ? `${this.hostname}:${this.port}` : this.hostname;
  }
  get origin() {
    return this.protocol in DEFAULT_PORTS ? `${this.protocol}//${this.host}` : "null";
  }
  get href() {
    const auth = this.username ? `${this.username}${this.password ? `:${this.password}` : ""}@` : "";
    const authority = this.hostname || this.protocol in DEFAULT_PORTS ? `//${auth}${this.host}` : "";
    return `${this.protocol}${authority}${this.pathname}${this.#raw}${this.hash}`;
  }
  toString() {
    return this.href;
  }
  toJSON() {
    return this.href;
  }
  static canParse(input, base) {
    try {
      new URL(input, base);
      return true;
    } catch {
      return false;
    }
  }
}
globalThis.URL ??= URL;
globalThis.URLSearchParams ??= URLSearchParams;

class TextEncoder {
  encoding = "utf-8";
  encode(text = "") {
    const out = [];
    for (const ch of String(text)) {
      let c = ch.codePointAt(0);
      if (c >= 0xd800 && c <= 0xdfff) c = 0xfffd;
      if (c < 0x80) out.push(c);
      else if (c < 0x800) out.push(0xc0 | (c >> 6), 0x80 | (c & 63));
      else if (c < 0x10000) out.push(0xe0 | (c >> 12), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
      else out.push(0xf0 | (c >> 18), 0x80 | ((c >> 12) & 63), 0x80 | ((c >> 6) & 63), 0x80 | (c & 63));
    }
    return new Uint8Array(out);
  }
}

class TextDecoder {
  encoding = "utf-8";
  decode(input) {
    if (!input) return "";
    const bytes = input instanceof Uint8Array ? input : new Uint8Array(input.buffer ?? input);
    let out = "";
    for (let i = 0; i < bytes.length; ) {
      const b = bytes[i];
      const need = b < 0x80 ? 0 : b >> 5 === 6 ? 1 : b >> 4 === 14 ? 2 : b >> 3 === 30 ? 3 : -1;
      if (need < 0 || i + need >= bytes.length + (need ? 0 : 1)) {
        out += "�";
        i++;
        continue;
      }
      let c = need === 0 ? b : b & (0x3f >> need);
      for (let j = 1; j <= need; j++) c = (c << 6) | (bytes[i + j] & 63);
      out += String.fromCodePoint(c);
      i += need + 1;
    }
    return out;
  }
}
globalThis.TextEncoder ??= TextEncoder;
globalThis.TextDecoder ??= TextDecoder;

const hexBytes = (n) => {
  const hex = rust.random(n);
  return Array.from({ length: n }, (_, i) => parseInt(hex.slice(i * 2, i * 2 + 2), 16));
};
globalThis.crypto ??= {
  getRandomValues(array) {
    const bytes = hexBytes(array.byteLength);
    new Uint8Array(array.buffer, array.byteOffset, array.byteLength).set(bytes);
    return array;
  },
  randomUUID() {
    const b = hexBytes(16);
    b[6] = (b[6] & 0x0f) | 0x40;
    b[8] = (b[8] & 0x3f) | 0x80;
    const h = b.map((x) => x.toString(16).padStart(2, "0")).join("");
    return `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
  },
};

globalThis.structuredClone ??= (value) => (value === undefined ? undefined : JSON.parse(JSON.stringify(value)));
