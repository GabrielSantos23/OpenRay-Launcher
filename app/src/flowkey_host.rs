use crate::extension::{Async, Confirmation, Toast};
use crate::flowkey_core::{self as core, fail, FkError};
use crate::flowkey_gate::{self as gate, Grants};
use crate::flowkey_js::{self, Worker};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::Duration;

type Res<T> = Result<T, FkError>;

pub const IMAGE_ORIGIN: &str = "http://extimg.localhost/";
pub const FILE_ORIGIN: &str = "fkfile://";
const BACKGROUND_LIFETIME: Duration = Duration::from_secs(60);
const STORAGE_LIMITS: (usize, usize) = (256, 256 * 1024);
const CACHE_LIMITS: (usize, usize) = (512, 1024 * 1024);
const SECRET_LIMITS: (usize, usize) = (64, 8 * 1024);

pub fn data_base() -> PathBuf {
    std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_default().join("dev.openray.app")
}

pub fn cache_base() -> PathBuf {
    std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir).join("dev.openray.app")
}

pub fn image_file(uri: &str) -> Option<PathBuf> {
    let name = uri.strip_prefix(IMAGE_ORIGIN)?;
    let (hash, ext) = name.split_once('.')?;
    let valid = hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()) && core::IMAGE_TYPES.iter().any(|(_, e)| *e == ext);
    valid.then(|| core::image_dir(&cache_base()).join(name))
}

pub struct Installed {
    pub manifest: Value,
    pub source: &'static str,
    pub dir: PathBuf,
}

impl Installed {
    pub fn id(&self) -> &str {
        self.manifest["id"].as_str().unwrap_or_default()
    }

    pub fn name(&self) -> &str {
        self.manifest["name"].as_str().unwrap_or_default()
    }

    fn version(&self) -> &str {
        self.manifest["version"].as_str().unwrap_or_default()
    }

    pub fn commands(&self) -> Vec<Value> {
        self.manifest["commands"].as_array().cloned().unwrap_or_default()
    }

    pub fn command(&self, id: &str) -> Option<Value> {
        self.commands().into_iter().find(|c| c["id"] == id)
    }

    fn entry(&self) -> String {
        let name = self.manifest["entry"].as_str().unwrap_or("main.js");
        let path = std::fs::canonicalize(self.dir.join(name)).unwrap_or_else(|_| self.dir.join(name));
        path.to_string_lossy().trim_start_matches(r"\\?\").replace('\\', "/")
    }

    pub fn image(&self, src: &str) -> Option<String> {
        let lower = src.to_ascii_lowercase();
        if src.starts_with(IMAGE_ORIGIN) || src.starts_with("appicon://") {
            return Some(src.to_owned());
        }
        let image_file = [".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico", ".bmp"].iter().any(|e| lower.ends_with(e));
        if image_file && !src.contains(':') && !src.contains("..") && !src.starts_with(['/', '\\']) {
            return Some(format!("{FILE_ORIGIN}{}", self.dir.join(src).to_string_lossy()));
        }
        let url = url::Url::parse(src).ok()?;
        let hosts: Vec<String> = self.manifest["httpHosts"].as_array().map(|a| a.iter().filter_map(|h| h.as_str().map(String::from)).collect()).unwrap_or_default();
        core::url_allowed(&url, &hosts).ok().map(|()| url.to_string())
    }
}

fn manifest_issue(manifest: &Value, folder: &str) -> Option<String> {
    let text = |field: &str| manifest[field].as_str().filter(|s| !s.trim().is_empty());
    let Some(id) = text("id") else { return Some("id: is required".into()) };
    if !core::valid_id(id) {
        return Some(format!("id: '{id}' is not a valid extension id"));
    }
    if id != folder {
        return Some(format!("manifest id '{id}' does not match its folder '{folder}'"));
    }
    for field in ["name", "version"] {
        if text(field).is_none() {
            return Some(format!("{field}: is required"));
        }
    }
    let Some(commands) = manifest["commands"].as_array() else { return Some("commands: must be an array".into()) };
    if commands.iter().any(|c| c["id"].as_str().is_none_or(|s| s.is_empty()) || c["title"].as_str().is_none()) {
        return Some("commands: every command needs an id and a title".into());
    }
    for field in ["nativeMethods", "httpHosts", "oauth", "fsPaths", "uriSchemes"] {
        let value = &manifest[field];
        if !value.is_null() && !value.as_array().is_some_and(|a| a.iter().all(Value::is_string)) {
            return Some(format!("{field}: must be a list of strings"));
        }
    }
    None
}

fn bundled_roots() -> Vec<PathBuf> {
    let beside = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join("extensions")));
    match beside.filter(|dir| dir.is_dir()) {
        Some(dir) => std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect(),
        None if cfg!(debug_assertions) => {
            let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../extensions");
            std::fs::read_dir(repo).into_iter().flatten().flatten().map(|e| e.path().join("dist")).collect()
        }
        None => Vec::new(),
    }
}

fn folder_name(dir: &Path, source: &str) -> String {
    let named = if source == "bundled" && dir.file_name().is_some_and(|n| n == "dist") { dir.parent().unwrap_or(dir) } else { dir };
    named.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn list() -> (Vec<Arc<Installed>>, Vec<String>) {
    let installed: Vec<PathBuf> = std::fs::read_dir(core::installed_root()).into_iter().flatten().flatten().map(|e| e.path()).collect();
    let mut by_id: HashMap<String, Arc<Installed>> = HashMap::new();
    let mut failures = Vec::new();
    for (source, dirs) in [("bundled", bundled_roots()), ("installed", installed)] {
        for dir in dirs {
            let name = folder_name(&dir, source);
            if name.starts_with('.') || !dir.is_dir() {
                continue;
            }
            let manifest = std::fs::read_to_string(dir.join("manifest.json")).map_err(|e| format!("manifest.json could not be read: {e}")).and_then(|t| serde_json::from_str::<Value>(&t).map_err(|e| format!("manifest.json is not valid JSON: {e}")));
            match manifest.and_then(|m| manifest_issue(&m, &name).map_or(Ok(m), Err)) {
                Ok(manifest) => {
                    by_id.insert(name, Arc::new(Installed { manifest, source, dir }));
                }
                Err(issue) => failures.push(format!("{name}: {issue}")),
            }
        }
    }
    let mut extensions: Vec<_> = by_id.into_values().collect();
    extensions.sort_by_key(|e| e.name().to_lowercase());
    (extensions, failures)
}

fn stored(ext: &Installed, name: &str) -> Option<Value> {
    core::data_get(&data_base(), ext.id(), name).ok().filter(|v| !v.is_null())
}

fn consent(ext: &Installed) -> Option<Grants> {
    stored(ext, "consent").and_then(|v| serde_json::from_value(v).ok())
}

pub fn grants(ext: &Installed) -> Grants {
    let declared = Grants::of(&ext.manifest);
    if ext.source == "bundled" {
        declared
    } else {
        declared.intersect(consent(ext).as_ref())
    }
}

pub fn pending_consent(ext: &Installed) -> Option<Grants> {
    if ext.source == "bundled" {
        return None;
    }
    Some(Grants::of(&ext.manifest).missing(consent(ext).as_ref())).filter(|m| !m.is_empty())
}

pub fn grant_consent(ext: &Installed) -> Result<(), String> {
    let grants = serde_json::to_value(Grants::of(&ext.manifest)).map_err(|e| e.to_string())?;
    core::data_set(&data_base(), ext.id(), "consent", &grants).map_err(|e| e.message)
}

pub fn preferences(ext: &Installed) -> Map<String, Value> {
    let mut values: Map<String, Value> = ext.manifest["preferences"].as_array().into_iter().flatten().filter_map(|p| Some((p["name"].as_str()?.to_owned(), p.get("default").filter(|d| !d.is_null())?.clone()))).collect();
    if let Some(Value::Object(saved)) = stored(ext, "preferences") {
        values.extend(saved);
    }
    values
}

pub fn missing_preferences(ext: &Installed) -> Vec<String> {
    let values = preferences(ext);
    ext.manifest["preferences"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["required"] == true && p["type"] != "checkbox")
        .filter(|p| p["name"].as_str().is_none_or(|n| values.get(n).is_none_or(|v| v.is_null() || v == "")))
        .map(|p| p["title"].as_str().or(p["name"].as_str()).unwrap_or_default().to_owned())
        .collect()
}

#[derive(Default)]
pub struct ViewState {
    pub tree: Option<Value>,
    pub error: Option<String>,
    pub busy: usize,
}

pub type Shared = Arc<Mutex<ViewState>>;

type Done = Box<dyn FnOnce(Result<(), String>) + Send>;

enum Readiness {
    Loading(Vec<Value>),
    Ready,
    Failed(String),
}

pub struct Host {
    ext: Arc<Installed>,
    worker: Worker,
    ready: Mutex<Readiness>,
    views: Mutex<HashMap<String, Weak<Mutex<ViewState>>>>,
    done: Mutex<HashMap<String, Done>>,
}

static HOSTS: Mutex<Vec<Arc<Host>>> = Mutex::new(Vec::new());
static CTX: OnceLock<egui::Context> = OnceLock::new();
static NEXT: AtomicU64 = AtomicU64::new(1);

pub fn init(ctx: &egui::Context) {
    let _ = CTX.set(ctx.clone());
}

pub fn context() -> Option<egui::Context> {
    CTX.get().cloned()
}

fn repaint() {
    if let Some(ctx) = CTX.get() {
        ctx.request_repaint();
    }
}

fn post(request: Async) {
    if let Some(ctx) = CTX.get() {
        crate::extension::post(ctx, request);
    }
}

fn next_id(prefix: &str) -> String {
    format!("{prefix}{}", NEXT.fetch_add(1, Ordering::SeqCst))
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

fn host(ext: &Arc<Installed>) -> Arc<Host> {
    let mut hosts = lock(&HOSTS);
    if let Some(existing) = hosts.iter().find(|h| h.ext.id() == ext.id()) {
        if existing.ext.dir == ext.dir && existing.ext.version() == ext.version() {
            return existing.clone();
        }
    }
    hosts.retain(|h| h.ext.id() != ext.id());
    let id = ext.id().to_owned();
    let host = Arc::new_cyclic(|weak: &Weak<Host>| {
        let weak = weak.clone();
        let tag = id.clone();
        let worker = flowkey_js::spawn(id.clone(), ext.entry(), move |text| {
            if let Some(host) = weak.upgrade() {
                host.receive(&text);
            }
        }, move |level, message| eprintln!("openray: [flowkey:{tag}] {level}: {message}"));
        Host { ext: ext.clone(), worker, ready: Mutex::new(Readiness::Loading(Vec::new())), views: Mutex::default(), done: Mutex::default() }
    });
    host.worker.send(&json!({ "type": "load", "requestId": next_id("load"), "extensionId": id, "url": ext.entry(), "manifest": ext.manifest, "preferences": preferences(ext), "isDevelopment": false }));
    hosts.push(host.clone());
    host
}

impl Host {
    fn send(&self, message: Value) {
        let mut ready = lock(&self.ready);
        match &mut *ready {
            Readiness::Loading(queue) => queue.push(message),
            Readiness::Ready => self.worker.send(&message),
            Readiness::Failed(error) => {
                let error = error.clone();
                drop(ready);
                self.reject(&message, &error);
            }
        }
    }

    fn reject(&self, message: &Value, error: &str) {
        if let Some(view) = message["viewId"].as_str().and_then(|id| self.view(id)) {
            lock(&view).error = Some(error.to_owned());
        }
        if let Some(done) = message["requestId"].as_str().and_then(|id| lock(&self.done).remove(id)) {
            done(Err(error.to_owned()));
        }
        repaint();
    }

    fn view(&self, id: &str) -> Option<Shared> {
        lock(&self.views).get(id).and_then(Weak::upgrade)
    }

    fn receive(self: &Arc<Self>, text: &str) {
        let Ok(message) = serde_json::from_str::<Value>(text) else { return };
        let view = || message["viewId"].as_str().and_then(|id| self.view(id));
        let error_text = |m: &Value| m["error"]["message"].as_str().unwrap_or("the extension failed").to_owned();
        match message["type"].as_str().unwrap_or_default() {
            "loaded" => {
                let next = if message["ok"] == true { Readiness::Ready } else { Readiness::Failed(error_text(&message)) };
                let previous = std::mem::replace(&mut *lock(&self.ready), next);
                if let Readiness::Loading(queue) = previous {
                    for queued in queue {
                        self.send(queued);
                    }
                }
            }
            "tree" => {
                if let Some(view) = view() {
                    let mut state = lock(&view);
                    state.tree = Some(message["tree"].clone());
                    state.error = None;
                }
                repaint();
            }
            "viewError" => {
                if let Some(view) = view() {
                    lock(&view).error = Some(error_text(&message));
                }
                repaint();
            }
            "done" => {
                let done = message["requestId"].as_str().and_then(|id| lock(&self.done).remove(id));
                if let Some(done) = done {
                    done(if message["ok"] == true { Ok(()) } else { Err(error_text(&message)) });
                }
                repaint();
            }
            "nativeCall" => native(self.clone(), message),
            "windowCommand" => match message["command"].as_str() {
                Some("closeMainWindow") => post(Async::Hide(true)),
                Some("popToRoot") => post(Async::PopToRoot),
                Some("clearSearchBar") => post(Async::SetQuery(String::new())),
                _ => {}
            },
            "launchCommand" => {
                if let Some(command) = message["commandId"].as_str() {
                    post(Async::Launch(format!("flowkey.{}/{command}", self.ext.id()), message["query"].as_str().map(String::from)));
                }
            }
            "log" => eprintln!("openray: [flowkey:{}] {}: {}", self.ext.id(), message["level"].as_str().unwrap_or("info"), message["message"].as_str().unwrap_or_default()),
            _ => {}
        }
    }

    fn request(&self, mut message: Value, done: Done) {
        let id = next_id("r");
        message["requestId"] = json!(id);
        lock(&self.done).insert(id, done);
        self.send(message);
    }
}

pub fn open_view(ext: &Arc<Installed>, command: &str, query: &str, arguments: Option<Value>) -> (String, Shared) {
    let host = host(ext);
    let id = next_id("v");
    let state: Shared = Arc::default();
    lock(&host.views).insert(id.clone(), Arc::downgrade(&state));
    render(ext, &id, command, query, None, arguments);
    (id, state)
}

pub fn render(ext: &Arc<Installed>, view: &str, command: &str, query: &str, filter: Option<&str>, arguments: Option<Value>) {
    host(ext).send(json!({ "type": "render", "viewId": view, "extensionId": ext.id(), "commandId": command, "query": query, "filterValue": filter, "arguments": arguments }));
}

pub fn action(ext: &Arc<Installed>, view: &str, action: &str, item: Option<Value>, form: Option<Value>, done: impl FnOnce(Result<(), String>) + Send + 'static) {
    host(ext).request(json!({ "type": "action", "viewId": view, "actionId": action, "item": item, "formValues": form }), Box::new(done));
}

pub fn close(ext: &Arc<Installed>, view: &str) {
    let host = host(ext);
    lock(&host.views).remove(view);
    host.send(json!({ "type": "close", "viewId": view }));
}

pub fn background(ext: &Arc<Installed>, command: &str, arguments: Option<Value>, done: impl FnOnce(Result<(), String>) + Send + 'static) {
    let host = host(ext);
    let id = next_id("r");
    let (view, extension) = (format!("bg:{id}"), ext.clone());
    lock(&host.done).insert(
        id.clone(),
        Box::new(move |result| {
            std::thread::spawn(move || {
                std::thread::sleep(BACKGROUND_LIFETIME);
                close(&extension, &view);
            });
            done(result);
        }),
    );
    host.send(json!({ "type": "background", "requestId": id, "extensionId": ext.id(), "commandId": command, "arguments": arguments }));
}

pub fn unload(id: &str) {
    lock(&HOSTS).retain(|h| h.ext.id() != id);
    if let Some(stores) = lock(&STORES).as_mut() {
        stores.retain(|key, _| !key.starts_with(&format!("{id}/")));
    }
}

pub fn set_preferences(ext: &Installed, values: Map<String, Value>) -> Result<(), String> {
    core::data_set(&data_base(), ext.id(), "preferences", &Value::Object(values.clone())).map_err(|e| e.message)?;
    let host = lock(&HOSTS).iter().find(|h| h.ext.id() == ext.id()).cloned();
    if let Some(host) = host {
        let values = preferences(&host.ext);
        host.send(json!({ "type": "preferences", "extensionId": ext.id(), "values": values }));
    }
    Ok(())
}

pub fn revoke_consent(ext: &Installed) -> Result<(), String> {
    core::data_set(&data_base(), ext.id(), "consent", &serde_json::to_value(Grants::default()).map_err(|e| e.to_string())?).map_err(|e| e.message)
}

pub fn check_package(manifest: &Value) -> Result<(), String> {
    let id = manifest["id"].as_str().unwrap_or_default();
    manifest_issue(manifest, id).map_or(Ok(()), |issue| Err(format!("This package is not a valid FlowKey extension: {issue}")))
}

pub fn newer_version(next: &str, current: &str) -> bool {
    let parse = |v: &str| {
        let core = v.split('+').next().unwrap_or_default();
        let (numbers, pre) = core.split_once('-').map_or((core, None), |(n, p)| (n, Some(p.to_owned())));
        let nums: Vec<u64> = numbers.split('.').map(|n| n.parse().unwrap_or(0)).chain(std::iter::repeat(0)).take(3).collect();
        (nums, pre)
    };
    let ((a, a_pre), (b, b_pre)) = (parse(next), parse(current));
    if a != b {
        return a > b;
    }
    match (a_pre, b_pre) {
        (None, None) => false,
        (None, Some(_)) => true,
        (Some(_), None) => false,
        (Some(x), Some(y)) => {
            let key = |s: &str| s.split('.').map(|p| p.parse::<u64>().map_or((1, 0, p.to_owned()), |n| (0, n, String::new()))).collect::<Vec<_>>();
            key(&x) > key(&y)
        }
    }
}

fn runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().thread_name("flowkey-native").build().expect("tokio runtime"))
}

fn native(host: Arc<Host>, message: Value) {
    runtime().spawn(async move {
        let method = message["method"].as_str().unwrap_or_default().to_owned();
        let params = message.get("params").filter(|p| p.is_object()).cloned().unwrap_or_else(|| json!({}));
        let reply = match dispatch(&host.ext, &method, &params).await {
            Ok(result) => json!({ "type": "nativeResult", "requestId": message["requestId"], "ok": true, "result": result }),
            Err(error) => json!({ "type": "nativeResult", "requestId": message["requestId"], "ok": false, "error": { "code": error.code, "message": error.message } }),
        };
        host.worker.send(&reply);
    });
}

fn text(params: &Value, name: &str) -> Res<String> {
    optional(params, name)?.ok_or_else(|| fail("invalidParams", format!("'{name}' is required")))
}

fn optional(params: &Value, name: &str) -> Res<Option<String>> {
    match &params[name] {
        Value::Null => Ok(None),
        Value::String(s) => Ok(Some(s.clone())),
        _ => Err(fail("invalidParams", format!("'{name}' must be a string"))),
    }
}

fn key(params: &Value) -> Res<String> {
    let value = text(params, "key")?;
    if value.is_empty() || value.chars().count() > 256 {
        return Err(fail("invalidParams", "keys must be 1–256 characters"));
    }
    Ok(value)
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, String> + Send + 'static) -> Res<T> {
    core::blocking(move || work().map_err(|e| fail("nativeFailed", e))).await
}

fn ok() -> Value {
    json!({ "ok": true })
}

static STORES: Mutex<Option<HashMap<String, Map<String, Value>>>> = Mutex::new(None);

fn now_ms() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_millis() as f64)
}

pub fn kv(base: &Path, id: &str, name: &str, op: &str, params: &Value, (max_keys, max_bytes): (usize, usize)) -> Res<Value> {
    let mut stores = lock(&STORES);
    let entries = stores.get_or_insert_with(HashMap::new).entry(format!("{id}/{name}")).or_insert_with(|| core::data_get(base, id, name).ok().and_then(|v| v.as_object().cloned()).unwrap_or_default());
    let now = now_ms();
    entries.retain(|_, e| e["expiresAt"].as_f64().is_none_or(|at| at > now));
    let save = |entries: &Map<String, Value>| core::data_set(base, id, name, &Value::Object(entries.clone()));
    match op {
        "get" => Ok(json!({ "ok": true, "value": entries.get(&key(params)?).map_or(Value::Null, |e| e["value"].clone()) })),
        "keys" => Ok(json!({ "ok": true, "keys": entries.keys().collect::<Vec<_>>() })),
        "delete" => {
            entries.remove(&key(params)?);
            save(entries)?;
            Ok(ok())
        }
        "clear" => {
            entries.clear();
            save(entries)?;
            Ok(ok())
        }
        "set" => {
            let k = key(params)?;
            let ttl = &params["ttlSeconds"];
            if !ttl.is_null() && !ttl.as_f64().is_some_and(|t| t > 0.0) {
                return Err(fail("invalidParams", "ttlSeconds must be a positive number"));
            }
            let mut next = entries.clone();
            let mut entry = json!({ "value": params.get("value").cloned().unwrap_or(Value::Null) });
            if let Some(seconds) = ttl.as_f64() {
                entry["expiresAt"] = json!(now + seconds * 1000.0);
            }
            next.insert(k, entry);
            if next.len() > max_keys {
                return Err(fail("quotaExceeded", format!("{name} is limited to {max_keys} keys")));
            }
            if Value::Object(next.clone()).to_string().len() > max_bytes {
                return Err(fail("quotaExceeded", format!("{name} is limited to {} KB", max_bytes / 1024)));
            }
            save(&next)?;
            *entries = next;
            Ok(ok())
        }
        _ => Err(fail("methodNotSupported", format!("{name}.{op} is not supported"))),
    }
}

fn checked_path(params: &Value, name: &str, grants: &Grants, preferences: &Map<String, Value>) -> Res<String> {
    let path = text(params, name)?;
    if !gate::path_allowed(&path, &grants.fs_paths, preferences) {
        return Err(fail("pathNotAllowed", format!("'{path}' is outside the extension's declared fsPaths")));
    }
    Ok(path)
}

async fn fs(op: &'static str, path: String, to: Option<String>, content: Option<String>, append: bool) -> Res<Value> {
    core::blocking(move || core::fs_op(op, Path::new(&path), to.as_deref().map(Path::new), content, append, 20_000)).await
}

fn history() -> Res<&'static crate::clipboard::History> {
    crate::extensions::clipboard_history::HISTORY.get().ok_or_else(|| fail("nativeFailed", "clipboard history is not running"))
}

fn copy_entry(entry: &crate::clipboard::Entry) -> Result<(), String> {
    match &entry.content {
        crate::clipboard::Content::Text { text } => crate::clipboard::write_text(text),
        crate::clipboard::Content::Files { paths } => crate::clipboard::write_files(paths),
        crate::clipboard::Content::Image { .. } => crate::clipboard::write_image(&history().map_err(|e| e.message)?.image_png(entry)?),
    }
}

fn paste_after_hide() {
    post(Async::Hide(true));
    std::thread::sleep(Duration::from_millis(80));
    crate::clipboard::send_paste();
}

fn entry(params: &Value) -> Res<crate::clipboard::Entry> {
    let id = text(params, "id")?;
    history()?.get(&id).ok_or_else(|| fail("notFound", "the clipboard entry no longer exists"))
}

async fn dispatch(ext: &Arc<Installed>, method: &str, p: &Value) -> Res<Value> {
    let grants = grants(ext);
    if !gate::method_allowed(method, &grants.native_methods) {
        return Err(fail("methodNotDeclared", format!("native method '{method}' is not declared in the manifest or was not granted")));
    }
    let id = ext.id().to_owned();
    let prefs = preferences(ext);
    let provider = |p: &Value| -> Res<String> {
        let provider = text(p, "provider")?;
        if !grants.oauth.contains(&provider) {
            return Err(fail("providerNotDeclared", format!("extension did not declare oauth provider '{provider}' in its manifest")));
        }
        Ok(provider)
    };
    match method {
        "http.fetch" | "http.upload" => {
            let auth = optional(p, "auth")?;
            if auth.as_ref().is_some_and(|a| !grants.oauth.contains(a)) {
                return Err(fail("providerNotDeclared", format!("extension did not declare oauth provider '{}'", auth.unwrap_or_default())));
            }
            let mut request = json!({ "url": text(p, "url")?, "method": optional(p, "method")?, "headers": p.get("headers").filter(|h| h.is_object()).cloned().unwrap_or_else(|| json!({})), "body": optional(p, "body")?, "auth": auth, "discardBody": p["discardBody"] == true });
            if method == "http.upload" {
                request["method"] = json!(optional(p, "method")?.unwrap_or_else(|| "POST".into()));
                request["uploadBytes"] = json!(p["bytes"].as_f64().map_or(0, |b| b.clamp(0.0, 512.0 * 1024.0 * 1024.0) as u64));
                request["timeoutMs"] = p["timeoutMs"].clone();
                request["body"] = Value::Null;
                request["auth"] = Value::Null;
            }
            let request = serde_json::from_value(request).map_err(|e| fail("invalidParams", e.to_string()))?;
            core::http(data_base(), id, request, grants.http_hosts.clone()).await
        }
        "clipboard.read" => Ok(blocking(|| Ok(crate::clipboard::read_rich())).await.map(|(text, html, paths)| json!({ "text": text, "html": html, "paths": paths }))?),
        "clipboard.write" => {
            let paths: Option<Vec<String>> = p["paths"].as_array().map(|a| a.iter().map(|v| v.as_str().map_or_else(|| v.to_string(), String::from)).collect());
            let (text, html) = (optional(p, "text")?, optional(p, "html")?);
            blocking(move || crate::clipboard::write_rich(text.as_deref(), html.as_deref(), paths.as_deref())).await.map(|()| ok())
        }
        "clipboard.paste" => {
            let value = text(p, "text")?;
            blocking(move || {
                crate::clipboard::write_text(&value)?;
                paste_after_hide();
                Ok(ok())
            })
            .await
        }
        "clipboard.clear" => blocking(|| crate::clipboard::write_rich(None, None, None)).await.map(|()| ok()),
        "clipboard.history" => {
            let query = optional(p, "query")?.unwrap_or_default().to_lowercase();
            let limit = p["limit"].as_f64().map_or(100, |l| l.max(0.0) as usize);
            let items: Vec<Value> = history()?
                .list()
                .into_iter()
                .map(|e| {
                    let text = match &e.content {
                        crate::clipboard::Content::Text { text } => text.clone(),
                        crate::clipboard::Content::Files { paths } => paths.join("\n"),
                        crate::clipboard::Content::Image { .. } => "Image".into(),
                    };
                    (e.id, text)
                })
                .filter(|(_, t)| t.to_lowercase().contains(&query))
                .take(limit)
                .map(|(id, text)| json!({ "id": id, "text": text }))
                .collect();
            Ok(json!({ "items": items }))
        }
        "clipboard.clearHistory" => history()?.delete_all().map(|()| ok()).map_err(|e| fail("nativeFailed", e)),
        "clipboard.deleteEntry" => history()?.delete(&text(p, "id")?).map(|()| ok()).map_err(|e| fail("nativeFailed", e)),
        "clipboard.copyEntry" => {
            let entry = entry(p)?;
            blocking(move || copy_entry(&entry)).await.map(|()| ok())
        }
        "clipboard.pasteEntry" => {
            let entry = entry(p)?;
            blocking(move || {
                copy_entry(&entry)?;
                paste_after_hide();
                Ok(ok())
            })
            .await
        }
        "clipboard.editEntry" => {
            let entry = entry(p)?;
            let target = match &entry.content {
                crate::clipboard::Content::Text { text } => {
                    use sha2::Digest;
                    let path = std::env::temp_dir().join(format!("openray-clipboard-{}.txt", &core::hex(&sha2::Sha256::digest(entry.id.as_bytes()))[..16]));
                    std::fs::write(&path, text)?;
                    path
                }
                crate::clipboard::Content::Files { paths } => PathBuf::from(paths.first().ok_or_else(|| fail("notFound", "the entry has no files"))?),
                crate::clipboard::Content::Image { .. } => history()?.image_path(&entry.id).ok_or_else(|| fail("notFound", "the image file is missing"))?,
            };
            crate::apps::open_url(&target.to_string_lossy(), None).map(|()| ok()).map_err(|e| fail("nativeFailed", e))
        }
        "storage.get" | "storage.set" | "storage.delete" | "storage.keys" | "cache.get" | "cache.set" | "cache.delete" | "cache.clear" => {
            let (name, op) = method.split_once('.').unwrap_or_default();
            let limits = if name == "storage" { STORAGE_LIMITS } else { CACHE_LIMITS };
            let (name, op, params) = (name.to_owned(), op.to_owned(), p.clone());
            core::blocking(move || kv(&data_base(), &id, &name, &op, &params, limits)).await
        }
        "secrets.get" => {
            let k = key(p)?;
            Ok(json!({ "ok": true, "value": core::secret(&data_base(), &id, "get", k, None, None)? }))
        }
        "secrets.set" => {
            let value = text(p, "value")?;
            if value.len() > SECRET_LIMITS.1 {
                return Err(fail("quotaExceeded", "secret values are limited to 8 KB"));
            }
            core::secret(&data_base(), &id, "set", key(p)?, Some(value), Some(SECRET_LIMITS.0)).map(|_| ok())
        }
        "secrets.delete" => core::secret(&data_base(), &id, "delete", key(p)?, None, None).map(|_| ok()),
        "oauth.authorize" | "oauth.status" | "oauth.disconnect" => {
            let op = method.trim_start_matches("oauth.").to_owned();
            core::oauth(data_base(), id, op, provider(p)?, optional(p, "clientId")?).await
        }
        "shell.openUrl" => {
            let url = text(p, "url")?;
            if !gate::scheme_allowed(&url, &grants.uri_schemes) {
                return Err(fail("schemeNotAllowed", format!("'{url}' is not an http(s) link or a declared uriScheme")));
            }
            crate::apps::open_url(&url, None).map(|()| ok()).map_err(|e| fail("nativeFailed", e))
        }
        "shell.open" => {
            let target = match optional(p, "target")? {
                Some(t) => t,
                None => match optional(p, "url")? {
                    Some(u) => u,
                    None => text(p, "path")?,
                },
            };
            let lower = target.to_ascii_lowercase();
            let web = (lower.starts_with("https://") || lower.starts_with("http://")) && gate::scheme_allowed(&target, &[]);
            let bytes = target.as_bytes();
            let absolute = (bytes.len() > 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && matches!(bytes[2], b'\\' | b'/')) || target.starts_with(r"\\");
            if !web && !(absolute && Path::new(&target).exists()) {
                return Err(fail("invalidTarget", "shell.open accepts http(s) links or existing absolute paths"));
            }
            crate::apps::open_url(&target, None).map(|()| ok()).map_err(|e| fail("nativeFailed", e))
        }
        "shell.openPath" => crate::apps::open_url(&checked_path(p, "path", &grants, &prefs)?, None).map(|()| ok()).map_err(|e| fail("nativeFailed", e)),
        "shell.revealPath" => crate::apps::show_in_explorer(&checked_path(p, "path", &grants, &prefs)?).map(|()| ok()).map_err(|e| fail("nativeFailed", e)),
        "fs.readText" => Ok(json!({ "ok": true, "content": fs("readText", checked_path(p, "path", &grants, &prefs)?, None, None, false).await?["content"] })),
        "fs.writeText" => fs("writeText", checked_path(p, "path", &grants, &prefs)?, None, Some(text(p, "content")?), p["append"] == true).await.map(|_| ok()),
        "fs.delete" => fs("delete", checked_path(p, "path", &grants, &prefs)?, None, None, false).await.map(|_| ok()),
        "fs.stat" => Ok(fs("stat", checked_path(p, "path", &grants, &prefs)?, None, None, false).await?["entry"].clone()),
        "fs.mkdir" => fs("mkdir", checked_path(p, "path", &grants, &prefs)?, None, None, false).await.map(|_| ok()),
        "fs.exists" => Ok(json!({ "ok": true, "exists": fs("exists", checked_path(p, "path", &grants, &prefs)?, None, None, false).await?["exists"] == true })),
        "fs.copy" => fs("copy", checked_path(p, "from", &grants, &prefs)?, Some(checked_path(p, "to", &grants, &prefs)?), None, false).await.map(|_| ok()),
        "fs.move" => fs("move", checked_path(p, "from", &grants, &prefs)?, Some(checked_path(p, "to", &grants, &prefs)?), None, false).await.map(|_| ok()),
        "fs.trash" => fs("trash", checked_path(p, "path", &grants, &prefs)?, None, None, false).await.map(|_| ok()),
        "fs.glob" => {
            let pattern = text(p, "pattern")?;
            let limit = p["limit"].as_f64().map_or(500, |l| l.clamp(1.0, 5000.0) as usize);
            let bytes = pattern.as_bytes();
            let absolute = (bytes.len() > 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && matches!(bytes[2], b'\\' | b'/')) || pattern.starts_with(r"\\");
            let mut roots: Vec<String> = grants.fs_paths.iter().filter_map(|scope| gate::scope_root(scope, &prefs)).filter(|r| !r.is_empty()).collect();
            roots.dedup();
            let mut entries: Vec<Value> = Vec::new();
            for root in roots {
                let full = if absolute { pattern.clone() } else { format!("{}/{pattern}", root.trim_end_matches(['\\', '/'])) };
                let walked = core::blocking({
                    let root = root.clone();
                    move || core::fs_op("walk", Path::new(&root), None, None, false, 20_000)
                })
                .await?;
                for entry in walked["entries"].as_array().into_iter().flatten() {
                    if entries.len() >= limit {
                        break;
                    }
                    let path = entry["path"].as_str().unwrap_or_default();
                    if gate::glob_matches(&full, path) && gate::path_allowed(path, &grants.fs_paths, &prefs) && !entries.iter().any(|e| e["path"] == path) {
                        entries.push(entry.clone());
                    }
                }
            }
            Ok(json!({ "ok": true, "entries": entries }))
        }
        "apps.list" => {
            let query = optional(p, "query")?.unwrap_or_default().to_lowercase();
            let apps: Vec<Value> = lock(&crate::extensions::APPS).iter().filter(|a| a.name.to_lowercase().contains(&query)).map(|a| json!({ "id": a.path, "name": a.name, "iconUri": format!("appicon://64/{}", a.path) })).collect();
            Ok(json!({ "apps": apps }))
        }
        "apps.launch" => {
            let target = text(p, "id")?;
            let path = {
                let apps = lock(&crate::extensions::APPS);
                apps.iter().find(|a| a.path == target).or_else(|| apps.iter().find(|a| a.name.eq_ignore_ascii_case(&target))).map(|a| a.path.clone())
            };
            let path = path.ok_or_else(|| fail("appNotFound", format!("no installed application matches '{target}'")))?;
            crate::apps::launch(&path, false).map(|()| ok()).map_err(|e| fail("nativeFailed", e))
        }
        "apps.frontmost" => {
            let previous = crate::launcher::previous_window();
            let app = (!previous.is_invalid()).then(|| crate::clipboard::window_app(previous)).flatten();
            Ok(json!({ "app": app.map(|(name, path)| json!({ "id": path, "name": name, "path": path })) }))
        }
        "apps.default" => Ok(json!({ "path": core::default_app(text(p, "path")?) })),
        "media.current" => core::blocking(core::media::current).await,
        "media.control" => {
            let command = text(p, "command")?;
            if !["playPause", "next", "previous"].contains(&command.as_str()) {
                return Err(fail("invalidParams", "command must be playPause, next or previous"));
            }
            core::blocking(move || core::media::control(&command)).await.map(|()| ok())
        }
        "image.fetch" => Ok(json!({ "ok": true, "uri": core::image(cache_base(), text(p, "url")?, grants.http_hosts.clone()).await? })),
        "system.selectedText" => {
            if p["allowFallback"] != true {
                return Ok(json!({ "text": null }));
            }
            post(Async::Hide(true));
            Ok(json!({ "text": core::blocking(|| Ok(core::copy_selection())).await? }))
        }
        "hud.show" => {
            post(Async::Hide(true));
            post(Async::Hud(Toast::Success(text(p, "title")?)));
            Ok(ok())
        }
        "toast.show" => {
            let title = text(p, "title")?;
            post(Async::Toast(if p["style"] == "failure" { Toast::Failure(title, optional(p, "message")?.unwrap_or_default()) } else { Toast::Success(title) }));
            Ok(ok())
        }
        "alert.confirm" => {
            let confirmation = Confirmation { title: text(p, "title")?, message: optional(p, "message")?.unwrap_or_default(), confirm_title: optional(p, "confirmTitle")?.unwrap_or_else(|| "Confirm".into()) };
            let (reply, answer) = std::sync::mpsc::channel();
            post(Async::Show);
            post(Async::Confirm(confirmation, Box::new(move |yes| {
                let _ = reply.send(yes);
            })));
            let confirmed = core::blocking(move || Ok(answer.recv().unwrap_or(false))).await?;
            Ok(json!({ "confirmed": confirmed }))
        }
        _ => Err(fail("methodNotSupported", format!("native method '{method}' is not supported by Openray"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_manifests_that_do_not_match_their_folder() {
        let good = json!({ "id": "demo", "name": "Demo", "version": "1.0.0", "commands": [{ "id": "c", "title": "C" }], "nativeMethods": ["http.fetch"] });
        assert_eq!(manifest_issue(&good, "demo"), None);
        assert!(manifest_issue(&good, "other").unwrap().contains("does not match"));
        assert!(manifest_issue(&json!({ "id": "Demo" }), "Demo").unwrap().contains("not a valid"));
        assert!(manifest_issue(&json!({ "id": "demo", "name": "D", "version": "1", "commands": [{ "id": "c" }] }), "demo").is_some());
        assert!(manifest_issue(&json!({ "id": "demo", "name": "D", "version": "1", "commands": [], "httpHosts": [1] }), "demo").unwrap().contains("httpHosts"));
    }

    #[test]
    fn resolves_extension_images_safely() {
        let ext = Installed { manifest: json!({ "id": "demo", "httpHosts": ["i.scdn.co"] }), source: "installed", dir: PathBuf::from(r"C:\ext\demo") };
        assert_eq!(ext.image("icon.png").as_deref(), Some(r"fkfile://C:\ext\demo\icon.png"));
        assert_eq!(ext.image("../x.png"), None);
        assert_eq!(ext.image("C:/Windows/x.png"), None);
        assert_eq!(ext.image("https://i.scdn.co/image/a").as_deref(), Some("https://i.scdn.co/image/a"));
        assert_eq!(ext.image("https://evil.example/a.png"), None);
        assert!(image_file(&format!("{IMAGE_ORIGIN}{}.png", "a".repeat(64))).is_some());
        assert!(image_file(&format!("{IMAGE_ORIGIN}..\\x.png")).is_none());
    }

    #[test]
    fn compares_versions_like_the_tauri_installer() {
        assert!(newer_version("1.2.0", "1.1.9"));
        assert!(!newer_version("1.2.0", "1.2.0"));
        assert!(newer_version("1.2.0", "1.2.0-beta.2"));
        assert!(!newer_version("1.2.0-beta.2", "1.2.0"));
        assert!(newer_version("1.2.0-beta.10", "1.2.0-beta.9"));
        assert!(!newer_version("0.9", "1.0.0"));
    }

    #[test]
    fn enforces_storage_quotas_and_ttl() {
        let dir = tempfile::tempdir().unwrap();
        let (base, id) = (dir.path(), "kv-test");
        assert_eq!(kv(base, id, "storage", "set", &json!({ "key": "a", "value": 1 }), (2, 1024)).unwrap(), ok());
        assert_eq!(kv(base, id, "storage", "get", &json!({ "key": "a" }), (2, 1024)).unwrap()["value"], 1);
        kv(base, id, "storage", "set", &json!({ "key": "b", "value": 2 }), (2, 1024)).unwrap();
        assert_eq!(kv(base, id, "storage", "set", &json!({ "key": "c", "value": 3 }), (2, 1024)).unwrap_err().code, "quotaExceeded");
        assert_eq!(kv(base, id, "storage", "set", &json!({ "key": "a", "value": "x".repeat(2000) }), (2, 1024)).unwrap_err().code, "quotaExceeded");
        assert_eq!(kv(base, id, "storage", "set", &json!({ "key": "a", "ttlSeconds": -1 }), (2, 1024)).unwrap_err().code, "invalidParams");
        assert_eq!(kv(base, id, "storage", "get", &json!({ "key": "" }), (2, 1024)).unwrap_err().code, "invalidParams");
        let saved = core::data_get(base, id, "storage").unwrap();
        assert_eq!(saved["b"]["value"], 2);
    }
}
