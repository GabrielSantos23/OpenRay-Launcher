use crate::store::{read_json, write_json};
use base64::Engine;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::http::{Request, Response};
use tauri::{AppHandle, Manager};
use url::Url;

#[derive(Debug, Serialize, PartialEq)]
pub struct FkError {
    code: String,
    message: String,
}

fn fail(code: &str, message: impl Into<String>) -> FkError {
    FkError { code: code.into(), message: message.into() }
}

impl From<std::io::Error> for FkError {
    fn from(error: std::io::Error) -> Self {
        fail("ioError", error.to_string())
    }
}

impl From<windows::core::Error> for FkError {
    fn from(error: windows::core::Error) -> Self {
        fail("nativeFailed", error.message())
    }
}

type Res<T> = Result<T, FkError>;

const SOURCES: [&str; 2] = ["bundled", "installed"];
const DATA_NAMES: [&str; 4] = ["storage", "cache", "consent", "preferences"];
const MAX_ENTRY_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PACKAGE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PACKAGE_ENTRIES: usize = 512;
const MAX_BODY_BYTES: u64 = 16 * 1024 * 1024;
const MAX_IMAGE_BYTES: u64 = 10 * 1024 * 1024;
const IMAGE_TYPES: [(&str, &str); 8] = [
    ("image/png", "png"),
    ("image/jpeg", "jpg"),
    ("image/gif", "gif"),
    ("image/webp", "webp"),
    ("image/svg+xml", "svg"),
    ("image/x-icon", "ico"),
    ("image/vnd.microsoft.icon", "ico"),
    ("image/bmp", "bmp"),
];
const WEB_CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob: http://extimg.localhost https:; font-src 'self' data:; media-src 'self' data: blob: https:; connect-src 'none'; form-action 'none'; base-uri 'none'";
const CLIENT_ID_DOCS: &str = "Spotify client id is not configured. Set the FLOWKEY_SPOTIFY_CLIENT_ID environment variable, create 'spotify-auth.json' with {\"clientId\":\"...\"} in Openray's data folder, or enter it in the extension's preferences.";

pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && !id.contains("..")
        && id.bytes().next().is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"._-".contains(&b))
}

fn check_id(id: &str) -> Res<()> {
    if valid_id(id) {
        Ok(())
    } else {
        Err(fail("invalidId", format!("'{id}' is not a valid extension id")))
    }
}

fn valid_entry(name: &str) -> bool {
    name.len() > 3
        && name.ends_with(".js")
        && name.bytes().next().is_some_and(|b| b.is_ascii_alphanumeric())
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

pub fn installed_root() -> PathBuf {
    if let Some(dir) = std::env::var_os("FLOWKEY_EXTENSIONS_DIR").filter(|v| !v.is_empty()) {
        return PathBuf::from(dir);
    }
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    local.join("FlowKey.Shell").join("extensions")
}

fn root(app: &AppHandle, source: &str) -> Option<PathBuf> {
    match source {
        "installed" => Some(installed_root()),
        "bundled" => app.path().resource_dir().ok().map(|dir| dir.join("extensions")),
        _ => None,
    }
}

fn data_dir(app: &AppHandle, id: &str) -> Res<PathBuf> {
    check_id(id)?;
    let base = app.path().app_data_dir().map_err(|e| fail("ioError", e.to_string()))?;
    Ok(base.join("flowkey").join(id))
}

fn image_dir(app: &AppHandle) -> PathBuf {
    app.path().app_cache_dir().unwrap_or_else(|_| std::env::temp_dir()).join("flowkey-images")
}

fn now_secs() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

fn millis(time: std::io::Result<SystemTime>) -> u64 {
    time.ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0, |d| d.as_millis() as u64)
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Res<T> + Send + 'static) -> Res<T> {
    tauri::async_runtime::spawn_blocking(work).await.map_err(|e| fail("nativeFailed", e.to_string()))?
}

#[derive(Serialize)]
pub struct Listed {
    source: &'static str,
    name: String,
    dir: String,
    manifest: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[tauri::command]
pub fn flowkey_list(app: AppHandle) -> Vec<Listed> {
    let mut listed = Vec::new();
    for source in SOURCES {
        let Some(entries) = root(&app, source).and_then(|dir| std::fs::read_dir(dir).ok()) else { continue };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let dir = entry.path();
            if name.starts_with('.') || !dir.is_dir() {
                continue;
            }
            let (manifest, error) = if !valid_id(&name) {
                (Value::Null, Some(format!("'{name}' is not a valid extension folder name")))
            } else {
                match std::fs::read_to_string(dir.join("manifest.json")) {
                    Ok(text) => match serde_json::from_str::<Value>(&text) {
                        Ok(value) => (value, None),
                        Err(e) => (Value::Null, Some(format!("manifest.json is not valid JSON: {e}"))),
                    },
                    Err(e) => (Value::Null, Some(format!("manifest.json could not be read: {e}"))),
                }
            };
            listed.push(Listed { source, name, dir: dir.to_string_lossy().into_owned(), manifest, error });
        }
    }
    listed
}

#[tauri::command]
pub fn flowkey_extensions_dir() -> String {
    installed_root().to_string_lossy().into_owned()
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        Some("txt" | "md") => "text/plain; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("svg") => "image/svg+xml",
        Some("ico") => "image/x-icon",
        Some("bmp") => "image/bmp",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        Some("mp3") => "audio/mpeg",
        _ => "application/octet-stream",
    }
}

fn respond(status: u16, content_type: &str, body: Vec<u8>, csp: Option<&str>) -> Response<Vec<u8>> {
    let mut builder = Response::builder()
        .status(status)
        .header("Content-Type", content_type)
        .header("Access-Control-Allow-Origin", "*")
        .header("Cache-Control", "no-cache")
        .header("X-Content-Type-Options", "nosniff");
    if let Some(csp) = csp {
        builder = builder.header("Content-Security-Policy", csp);
    }
    builder.body(body).expect("valid flowkey response")
}

fn not_found() -> Response<Vec<u8>> {
    respond(404, "text/plain; charset=utf-8", b"not found".to_vec(), None)
}

fn web_shim(id: &str) -> String {
    format!("window.__FLOWKEY_EXTENSION_ID__={};\n{}", serde_json::to_string(id).unwrap_or_default(), include_str!("flowkey_web.js"))
}

pub fn serve(app: &AppHandle, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let segments: Vec<String> = request
        .uri()
        .path()
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| percent_encoding::percent_decode_str(s).decode_utf8_lossy().into_owned())
        .collect();
    if segments.len() < 3 || segments.iter().any(|s| s == "." || s == ".." || s.contains(['\\', ':', '\0'])) {
        return not_found();
    }
    let (source, id, rest) = (&segments[0], &segments[1], &segments[2..]);
    let Some(base) = root(app, source).filter(|_| valid_id(id)).map(|dir| dir.join(id)) else { return not_found() };
    if rest.len() == 1 && rest[0] == "__shim__.js" {
        return respond(200, "text/javascript; charset=utf-8", web_shim(id).into_bytes(), None);
    }
    if rest.len() == 2 && rest[0] == "__host__" && valid_entry(&rest[1]) {
        let html = format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width\"><style>html,body{{margin:0;padding:0;height:100%;background:var(--fk-window-background,transparent);color:var(--fk-text-primary);overflow:hidden}}body{{font-family:Inter,'Segoe UI Variable','Segoe UI',sans-serif;font-size:14px}}:root{{--fk-radius:10px}}#root{{min-height:100vh}}</style><style id=\"fk-theme\"></style><script src=\"../__shim__.js\"></script></head><body><div id=\"root\"></div><script src=\"../{}\"></script></body></html>",
            rest[1]
        );
        return respond(200, "text/html; charset=utf-8", html.into_bytes(), Some(WEB_CSP));
    }
    let file = rest.iter().fold(base.clone(), |path, segment| path.join(segment));
    match (std::fs::canonicalize(&base), std::fs::canonicalize(&file)) {
        (Ok(base), Ok(file)) if file.starts_with(&base) && file.is_file() => match std::fs::read(&file) {
            Ok(bytes) => respond(200, content_type(&file), bytes, None),
            Err(_) => not_found(),
        },
        _ => not_found(),
    }
}

pub fn serve_image(app: &AppHandle, request: Request<Vec<u8>>) -> Response<Vec<u8>> {
    let name = request.uri().path().trim_start_matches('/');
    let valid = name.split_once('.').is_some_and(|(hash, ext)| hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()) && IMAGE_TYPES.iter().any(|(_, e)| *e == ext));
    if !valid {
        return not_found();
    }
    let path = image_dir(app).join(name);
    match std::fs::read(&path) {
        Ok(bytes) => respond(200, content_type(&path), bytes, None),
        Err(_) => not_found(),
    }
}

fn open_zip(path: &str) -> Res<zip::ZipArchive<std::fs::File>> {
    let file = std::fs::File::open(path)?;
    zip::ZipArchive::new(file).map_err(|e| fail("invalidPackage", format!("not a zip archive: {e}")))
}

fn zip_manifest(zip: &mut zip::ZipArchive<std::fs::File>) -> Res<Value> {
    let mut file = zip.by_name("manifest.json").map_err(|_| fail("invalidPackage", "manifest.json is missing from the package root"))?;
    if file.size() > 1024 * 1024 {
        return Err(fail("invalidPackage", "manifest.json is larger than 1 MB"));
    }
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    serde_json::from_str(&text).map_err(|e| fail("invalidPackage", format!("manifest.json is not valid JSON: {e}")))
}

#[tauri::command]
pub fn flowkey_package_manifest(path: String) -> Res<Value> {
    zip_manifest(&mut open_zip(&path)?)
}

fn check_entries(zip: &mut zip::ZipArchive<std::fs::File>) -> Res<()> {
    if zip.len() > MAX_PACKAGE_ENTRIES {
        return Err(fail("tooManyEntries", format!("packages are limited to {MAX_PACKAGE_ENTRIES} entries")));
    }
    let mut total = 0;
    for index in 0..zip.len() {
        let file = zip.by_index(index).map_err(|e| fail("invalidPackage", e.to_string()))?;
        let name = file.name().to_string();
        if file.enclosed_name().is_none() || name.contains(':') || name.starts_with(['/', '\\']) || name.split(['/', '\\']).any(|s| s == "..") {
            return Err(fail("zipSlip", format!("entry '{name}' escapes the install directory")));
        }
        if file.size() > MAX_ENTRY_BYTES {
            return Err(fail("entryTooLarge", format!("entry '{name}' is larger than 32 MB")));
        }
        total += file.size();
    }
    if total > MAX_PACKAGE_BYTES {
        return Err(fail("packageTooLarge", "packages are limited to 64 MB"));
    }
    Ok(())
}

fn extract(zip: &mut zip::ZipArchive<std::fs::File>, target: &Path) -> Res<()> {
    let mut written = 0;
    for index in 0..zip.len() {
        let mut file = zip.by_index(index).map_err(|e| fail("invalidPackage", e.to_string()))?;
        let Some(relative) = file.enclosed_name() else { return Err(fail("zipSlip", "entry escapes the install directory")) };
        let out = target.join(relative);
        if file.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut writer = std::fs::File::create(&out)?;
        let copied = std::io::copy(&mut (&mut file).take(MAX_ENTRY_BYTES + 1), &mut writer)?;
        written += copied;
        if copied > MAX_ENTRY_BYTES || written > MAX_PACKAGE_BYTES {
            return Err(fail("packageTooLarge", "the package expands beyond its size limits"));
        }
    }
    Ok(())
}

pub fn install_package(package: &str, id: &str, root: &Path) -> Res<PathBuf> {
    check_id(id)?;
    let mut zip = open_zip(package)?;
    check_entries(&mut zip)?;
    if zip_manifest(&mut zip)?.get("id").and_then(Value::as_str) != Some(id) {
        return Err(fail("idMismatch", "the package manifest id changed during install"));
    }
    std::fs::create_dir_all(root)?;
    let staging = root.join(format!(".staging-{id}"));
    let old = root.join(format!(".old-{id}"));
    let target = root.join(id);
    let _ = std::fs::remove_dir_all(&staging);
    let _ = std::fs::remove_dir_all(&old);
    std::fs::create_dir_all(&staging)?;
    if let Err(e) = extract(&mut zip, &staging) {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }
    if target.exists() {
        std::fs::rename(&target, &old)?;
    }
    if let Err(e) = std::fs::rename(&staging, &target) {
        if old.exists() {
            let _ = std::fs::rename(&old, &target);
        }
        return Err(e.into());
    }
    let _ = std::fs::remove_dir_all(&old);
    Ok(target)
}

#[tauri::command]
pub async fn flowkey_install(path: String, id: String) -> Res<String> {
    blocking(move || install_package(&path, &id, &installed_root()).map(|dir| dir.to_string_lossy().into_owned())).await
}

#[tauri::command]
pub fn flowkey_uninstall(app: AppHandle, id: String) -> Res<()> {
    check_id(&id)?;
    let dir = installed_root().join(&id);
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    let data = data_dir(&app, &id)?;
    if data.exists() {
        std::fs::remove_dir_all(data)?;
    }
    Ok(())
}

#[tauri::command]
pub fn flowkey_data_get(app: AppHandle, extension_id: String, name: String) -> Res<Value> {
    if !DATA_NAMES.contains(&name.as_str()) {
        return Err(fail("invalidParams", format!("unknown data file {name}")));
    }
    Ok(read_json(&data_dir(&app, &extension_id)?.join(format!("{name}.json"))))
}

#[tauri::command]
pub fn flowkey_data_set(app: AppHandle, extension_id: String, name: String, value: Value) -> Res<()> {
    if !DATA_NAMES.contains(&name.as_str()) {
        return Err(fail("invalidParams", format!("unknown data file {name}")));
    }
    let dir = data_dir(&app, &extension_id)?;
    std::fs::create_dir_all(&dir)?;
    Ok(write_json(&dir.join(format!("{name}.json")), &value)?)
}

mod dpapi {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB};

    pub fn seal(data: &[u8], protect: bool) -> windows::core::Result<Vec<u8>> {
        let input = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
        let mut output = CRYPT_INTEGER_BLOB::default();
        unsafe {
            if protect {
                CryptProtectData(&input, PCWSTR::null(), None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)?;
            } else {
                CryptUnprotectData(&input, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output)?;
            }
            let bytes = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
            let _ = LocalFree(Some(HLOCAL(output.pbData as _)));
            Ok(bytes)
        }
    }
}

fn read_sealed<T: DeserializeOwned + Default>(path: &Path) -> T {
    std::fs::read(path).ok().and_then(|bytes| dpapi::seal(&bytes, false).ok()).and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or_default()
}

fn write_sealed<T: Serialize>(path: &Path, value: &T) -> Res<()> {
    let plain = serde_json::to_vec(value).map_err(|e| fail("nativeFailed", e.to_string()))?;
    let sealed = dpapi::seal(&plain, true)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, sealed)?;
    Ok(std::fs::rename(tmp, path)?)
}

static SECRETS: Mutex<()> = Mutex::new(());

#[tauri::command]
pub fn flowkey_secret(app: AppHandle, extension_id: String, op: String, key: String, value: Option<String>, max_keys: Option<usize>) -> Res<Option<String>> {
    let path = data_dir(&app, &extension_id)?.join("secrets.bin");
    let _guard = SECRETS.lock().unwrap();
    let mut map: HashMap<String, String> = read_sealed(&path);
    match op.as_str() {
        "get" => Ok(map.get(&key).cloned()),
        "set" => {
            let value = value.ok_or_else(|| fail("invalidParams", "'value' is required"))?;
            if !map.contains_key(&key) && map.len() >= max_keys.unwrap_or(64) {
                return Err(fail("quotaExceeded", "secrets are limited to 64 keys"));
            }
            map.insert(key, value);
            write_sealed(&path, &map)?;
            Ok(None)
        }
        "delete" => {
            if map.remove(&key).is_some() {
                write_sealed(&path, &map)?;
            }
            Ok(None)
        }
        _ => Err(fail("invalidParams", format!("unknown secrets operation {op}"))),
    }
}

pub fn blocked_address(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            o[0] == 127 || o[0] == 10 || o[0] == 0 || (o[0] == 169 && o[1] == 254) || (o[0] == 172 && (o[1] & 0xF0) == 16) || (o[0] == 192 && o[1] == 168) || (o[0] == 100 && (o[1] & 0xC0) == 64) || o[0] >= 224
        }
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return blocked_address(IpAddr::V4(v4));
            }
            let s = v6.segments();
            v6.is_loopback() || v6.is_unspecified() || (s[0] & 0xffc0) == 0xfe80 || (s[0] & 0xfe00) == 0xfc00 || (s[0] & 0xff00) == 0xff00
        }
    }
}

pub fn url_allowed(url: &Url, hosts: &[String]) -> Res<()> {
    if url.scheme() != "https" {
        return Err(fail("insecureBlocked", format!("only https is allowed, got '{}'", url.scheme())));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(fail("userInfoBlocked", "URLs with embedded credentials are not allowed"));
    }
    let host = url.host_str().unwrap_or_default().to_ascii_lowercase();
    let port = url.port_or_known_default().unwrap_or(443);
    let allowed = hosts.iter().any(|entry| {
        let entry = entry.to_ascii_lowercase();
        let (name, entry_port) = match entry.rsplit_once(':').and_then(|(n, p)| p.parse::<u16>().ok().map(|p| (n.to_string(), p))) {
            Some(pair) => pair,
            None => (entry, 443),
        };
        entry_port == port && if name.starts_with('.') { host.ends_with(&name) && host.len() > name.len() } else { host == name }
    });
    if !allowed {
        return Err(fail("hostNotAllowed", format!("host '{host}' is not in the extension's httpHosts allowlist")));
    }
    let literal = match url.host() {
        Some(url::Host::Ipv4(ip)) => Some(IpAddr::V4(ip)),
        Some(url::Host::Ipv6(ip)) => Some(IpAddr::V6(ip)),
        _ => None,
    };
    if literal.is_some_and(blocked_address) {
        return Err(fail("privateAddressBlocked", format!("host '{host}' is a blocked address")));
    }
    Ok(())
}

struct SafeResolver;

impl reqwest::dns::Resolve for SafeResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_string();
        Box::pin(async move {
            let addresses: Vec<SocketAddr> = tokio::net::lookup_host((host.as_str(), 0)).await?.collect();
            if let Some(bad) = addresses.iter().find(|a| blocked_address(a.ip())) {
                return Err(format!("host '{host}' resolves to a blocked address ({})", bad.ip()).into());
            }
            Ok(Box::new(addresses.into_iter()) as reqwest::dns::Addrs)
        })
    }
}

fn client(hosts: Vec<String>, timeout: Duration) -> Res<reqwest::Client> {
    reqwest::Client::builder()
        .dns_resolver(Arc::new(SafeResolver))
        .redirect(reqwest::redirect::Policy::custom(move |attempt| {
            if attempt.previous().len() >= 10 {
                return attempt.error("too many redirects");
            }
            match url_allowed(attempt.url(), &hosts) {
                Ok(()) => attempt.follow(),
                Err(e) => attempt.error(e.message),
            }
        }))
        .timeout(timeout)
        .user_agent("Openray-FlowKey/1.0")
        .build()
        .map_err(|e| fail("networkError", e.to_string()))
}

fn network(error: reqwest::Error) -> FkError {
    let mut message = error.to_string();
    let mut source = std::error::Error::source(&error);
    while let Some(inner) = source {
        message = format!("{message}: {inner}");
        source = inner.source();
    }
    let code = if message.contains("blocked address") {
        "privateAddressBlocked"
    } else if message.contains("allowlist") || message.contains("only https") {
        "hostNotAllowed"
    } else if error.is_timeout() {
        "timeout"
    } else {
        "networkError"
    };
    fail(code, message)
}

async fn read_body(mut response: reqwest::Response, keep: bool, cap: u64) -> Res<(Vec<u8>, u64)> {
    let mut body = Vec::new();
    let mut received = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(network)? {
        received += chunk.len() as u64;
        if keep {
            if received > cap {
                return Err(fail("bodyTooLarge", format!("response bodies are limited to {} MB", cap / 1024 / 1024)));
            }
            body.extend_from_slice(&chunk);
        }
    }
    Ok((body, received))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpRequest {
    url: String,
    method: Option<String>,
    #[serde(default)]
    headers: HashMap<String, String>,
    body: Option<String>,
    auth: Option<String>,
    #[serde(default)]
    discard_body: bool,
    upload_bytes: Option<u64>,
    timeout_ms: Option<u64>,
}

#[tauri::command]
pub async fn flowkey_http(app: AppHandle, extension_id: String, request: HttpRequest, hosts: Vec<String>) -> Res<Value> {
    check_id(&extension_id)?;
    let url = Url::parse(&request.url).map_err(|e| fail("invalidUrl", e.to_string()))?;
    url_allowed(&url, &hosts)?;
    let timeout = Duration::from_millis(request.timeout_ms.unwrap_or(60_000).clamp(1_000, 600_000));
    let client = client(hosts, timeout)?;
    let method = reqwest::Method::from_bytes(request.method.as_deref().unwrap_or("GET").to_ascii_uppercase().as_bytes()).map_err(|_| fail("invalidParams", "invalid HTTP method"))?;
    let token = match &request.auth {
        Some(provider) => {
            let definition = provider_def(provider).ok_or_else(|| fail("unknownProvider", format!("oauth provider '{provider}' is not implemented")))?;
            if url.host_str() != Some(definition.pinned_host) {
                return Err(fail("authHostMismatch", format!("{provider} tokens are only sent to {}", definition.pinned_host)));
            }
            Some(valid_token(&app, &extension_id, provider, false).await?.ok_or_else(|| fail("notAuthorized", format!("not signed in to {provider}")))?.access_token)
        }
        None => None,
    };
    let send = |token: Option<&str>| {
        let mut builder = client.request(method.clone(), url.clone());
        for (name, value) in &request.headers {
            if name.eq_ignore_ascii_case("host") || (token.is_some() && name.eq_ignore_ascii_case("authorization")) {
                continue;
            }
            builder = builder.header(name.as_str(), value.as_str());
        }
        if let Some(token) = token {
            builder = builder.bearer_auth(token);
        }
        if let Some(bytes) = request.upload_bytes {
            builder = builder.body(vec![0u8; bytes as usize]);
        } else if let Some(body) = &request.body {
            builder = builder.body(body.clone());
        } else if method != reqwest::Method::GET && method != reqwest::Method::HEAD {
            builder = builder.body(Vec::new());
        }
        builder.send()
    };
    let started = Instant::now();
    let mut response = send(token.as_deref()).await.map_err(network)?;
    if response.status() == 401 {
        if let (Some(provider), Some(_)) = (&request.auth, &token) {
            if let Some(fresh) = valid_token(&app, &extension_id, provider, true).await? {
                response = send(Some(&fresh.access_token)).await.map_err(network)?;
            }
        }
    }
    let status = response.status().as_u16();
    if let Some(bytes) = request.upload_bytes {
        let _ = read_body(response, false, 0).await;
        return Ok(json!({ "status": status, "bytesSent": bytes, "elapsedMs": started.elapsed().as_millis() as u64 }));
    }
    let mut headers = serde_json::Map::new();
    for (name, value) in response.headers() {
        let value = value.to_str().unwrap_or_default();
        match headers.get_mut(name.as_str()) {
            Some(Value::String(existing)) => {
                existing.push_str(", ");
                existing.push_str(value);
            }
            _ => {
                headers.insert(name.as_str().to_string(), Value::String(value.to_string()));
            }
        }
    }
    let (body, received) = read_body(response, !request.discard_body, MAX_BODY_BYTES).await?;
    let body_text = (!request.discard_body).then(|| String::from_utf8_lossy(&body).into_owned());
    Ok(json!({ "status": status, "headers": headers, "bodyText": body_text, "bytesReceived": received }))
}

struct Provider {
    id: &'static str,
    authorize_url: &'static str,
    token_url: &'static str,
    scopes: &'static str,
    pinned_host: &'static str,
}

const SPOTIFY: Provider = Provider {
    id: "spotify",
    authorize_url: "https://accounts.spotify.com/authorize",
    token_url: "https://accounts.spotify.com/api/token",
    scopes: "playlist-modify-private playlist-modify-public playlist-read-collaborative playlist-read-private user-follow-read user-library-modify user-library-read user-modify-playback-state user-read-currently-playing user-read-playback-state user-read-private user-top-read",
    pinned_host: "api.spotify.com",
};

fn provider_def(id: &str) -> Option<&'static Provider> {
    (id == SPOTIFY.id).then_some(&SPOTIFY)
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct Token {
    access_token: String,
    refresh_token: String,
    expires_at: u64,
    scope: String,
    client_id: String,
}

fn token_path(app: &AppHandle, id: &str, provider: &str) -> Res<PathBuf> {
    Ok(data_dir(app, id)?.join(format!("oauth-{provider}.bin")))
}

fn auth_config(app: &AppHandle) -> (Option<String>, u16) {
    let from_env = std::env::var("FLOWKEY_SPOTIFY_CLIENT_ID").ok().filter(|v| !v.trim().is_empty());
    let dirs = [app.path().app_data_dir().ok(), std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("FlowKey.Shell"))];
    for dir in dirs.into_iter().flatten() {
        let config: Value = read_json(&dir.join("spotify-auth.json"));
        if config.is_object() {
            let id = config["clientId"].as_str().filter(|s| !s.trim().is_empty()).map(String::from);
            let port = config["redirectPort"].as_u64().unwrap_or(0).min(65535) as u16;
            return (from_env.or(id), port);
        }
    }
    (from_env, 0)
}

fn refresh_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

async fn post_token(provider: &Provider, form: &[(&str, &str)]) -> Res<(u16, Value)> {
    let client = reqwest::Client::builder().timeout(Duration::from_secs(15)).build().map_err(network)?;
    let response = client.post(provider.token_url).form(form).send().await.map_err(network)?;
    let status = response.status().as_u16();
    let text = response.text().await.map_err(network)?;
    Ok((status, serde_json::from_str(&text).unwrap_or(Value::Null)))
}

fn token_from(body: &Value, previous: Option<&Token>, client_id: &str) -> Option<Token> {
    let access_token = body["access_token"].as_str()?.to_string();
    let keep = |field: &str, old: Option<&String>| body[field].as_str().map(String::from).or_else(|| old.cloned()).unwrap_or_default();
    Some(Token {
        access_token,
        refresh_token: keep("refresh_token", previous.map(|p| &p.refresh_token)),
        expires_at: now_secs() + body["expires_in"].as_u64().unwrap_or(3600),
        scope: keep("scope", previous.map(|p| &p.scope)),
        client_id: client_id.to_string(),
    })
}

async fn valid_token(app: &AppHandle, id: &str, provider_id: &str, force: bool) -> Res<Option<Token>> {
    let _guard = refresh_lock().lock().await;
    let path = token_path(app, id, provider_id)?;
    let Some(token) = read_sealed::<Option<Token>>(&path) else { return Ok(None) };
    if !force && token.expires_at > now_secs() + 60 {
        return Ok(Some(token));
    }
    let provider = provider_def(provider_id).ok_or_else(|| fail("unknownProvider", format!("oauth provider '{provider_id}' is not implemented")))?;
    if token.refresh_token.is_empty() || token.client_id.is_empty() {
        return Ok((!force && token.expires_at > now_secs()).then_some(token));
    }
    let (status, body) = post_token(provider, &[("grant_type", "refresh_token"), ("refresh_token", &token.refresh_token), ("client_id", &token.client_id)]).await?;
    if status == 400 || status == 401 {
        let _ = std::fs::remove_file(&path);
        return Ok(None);
    }
    let Some(next) = (status < 300).then(|| token_from(&body, Some(&token), &token.client_id)).flatten() else { return Ok(None) };
    write_sealed(&path, &Some(next.clone()))?;
    Ok(Some(next))
}

pub fn rfc3339(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

fn authorized(token: &Token) -> Value {
    json!({ "ok": true, "expiresAt": rfc3339(token.expires_at), "scope": token.scope })
}

fn random_url_safe(bytes: usize) -> Res<String> {
    let mut buffer = vec![0u8; bytes];
    getrandom::fill(&mut buffer).map_err(|e| fail("nativeFailed", e.to_string()))?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buffer))
}

fn reply(stream: &mut std::net::TcpStream, status: u16, message: &str) {
    let html = format!("<!doctype html><html><head><meta charset=\"utf-8\"><title>Openray</title></head><body style=\"font-family:system-ui,sans-serif;background:#1f1f1f;color:#eee;display:grid;place-items:center;height:96vh\"><p>{message}</p></body></html>");
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        _ => "Not Found",
    };
    let _ = write!(stream, "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}", html.len());
}

pub fn wait_for_code(listener: TcpListener, state: &str, provider: &str, timeout: Duration) -> Res<String> {
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + timeout;
    loop {
        if Instant::now() > deadline {
            return Err(fail("timeout", format!("no oauth callback within {} seconds", timeout.as_secs())));
        }
        let mut stream = match listener.accept() {
            Ok((stream, _)) => stream,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
            Err(e) => return Err(e.into()),
        };
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))?;
        let mut buffer = [0u8; 8192];
        let read = stream.read(&mut buffer).unwrap_or(0);
        let request = String::from_utf8_lossy(&buffer[..read]).into_owned();
        let target = request.lines().next().and_then(|line| line.split_whitespace().nth(1)).unwrap_or("/");
        let Some(url) = Url::parse(&format!("http://127.0.0.1{target}")).ok().filter(|u| u.path() == "/callback") else {
            reply(&mut stream, 404, "Not found");
            continue;
        };
        let params: HashMap<String, String> = url.query_pairs().into_owned().collect();
        if params.get("state").map(String::as_str) != Some(state) {
            reply(&mut stream, 400, "Invalid state");
            return Err(fail("invalidState", "oauth callback state mismatch rejected (possible CSRF)"));
        }
        if let Some(error) = params.get("error") {
            reply(&mut stream, 200, "Authorization failed. You can close this tab.");
            return Err(fail("accessDenied", format!("authorization failed: {error}")));
        }
        let Some(code) = params.get("code") else {
            reply(&mut stream, 400, "Missing code");
            return Err(fail("invalidCallback", "oauth callback did not contain an authorization code"));
        };
        reply(&mut stream, 200, &format!("Connected with {provider}. You can close this tab and return to Openray."));
        return Ok(code.clone());
    }
}

#[tauri::command]
pub async fn flowkey_oauth(app: AppHandle, extension_id: String, op: String, provider: String, client_id: Option<String>) -> Res<Value> {
    check_id(&extension_id)?;
    let definition = provider_def(&provider).ok_or_else(|| fail("unknownProvider", format!("oauth provider '{provider}' is not implemented by Openray")))?;
    match op.as_str() {
        "status" => Ok(valid_token(&app, &extension_id, &provider, false).await?.map_or_else(|| json!({ "ok": false }), |t| authorized(&t))),
        "disconnect" => {
            let path = token_path(&app, &extension_id, &provider)?;
            if path.exists() {
                std::fs::remove_file(path)?;
            }
            Ok(json!({ "ok": true }))
        }
        "authorize" => {
            if let Some(token) = valid_token(&app, &extension_id, &provider, false).await? {
                return Ok(authorized(&token));
            }
            let (configured, port) = auth_config(&app);
            let client_id = client_id.filter(|v| !v.trim().is_empty()).or(configured).ok_or_else(|| fail("notConfigured", CLIENT_ID_DOCS))?;
            let verifier = random_url_safe(64)?;
            let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
            let state = random_url_safe(32)?;
            let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|e| fail("listenerFailed", format!("could not bind 127.0.0.1:{port}: {e}")))?;
            let port = listener.local_addr()?.port();
            let redirect = format!("http://127.0.0.1:{port}/callback");
            let mut url = Url::parse(definition.authorize_url).map_err(|e| fail("nativeFailed", e.to_string()))?;
            url.query_pairs_mut()
                .append_pair("response_type", "code")
                .append_pair("client_id", &client_id)
                .append_pair("redirect_uri", &redirect)
                .append_pair("scope", definition.scopes)
                .append_pair("state", &state)
                .append_pair("code_challenge_method", "S256")
                .append_pair("code_challenge", &challenge);
            crate::apps::open_url(url.as_str(), None).map_err(|e| fail("browserFailed", e))?;
            let name = definition.id;
            let code = blocking(move || wait_for_code(listener, &state, name, Duration::from_secs(120))).await?;
            let (status, body) = post_token(definition, &[("grant_type", "authorization_code"), ("code", &code), ("redirect_uri", &redirect), ("client_id", &client_id), ("code_verifier", &verifier)]).await?;
            if status >= 300 {
                let code = body["error"].as_str().unwrap_or("providerError");
                let message = body["error_description"].as_str().map(String::from).unwrap_or_else(|| format!("token endpoint returned {status}"));
                return Err(fail(code, message));
            }
            let token = token_from(&body, None, &client_id).ok_or_else(|| fail("invalidTokenResponse", "token response missing access_token"))?;
            let _guard = refresh_lock().lock().await;
            write_sealed(&token_path(&app, &extension_id, &provider)?, &Some(token.clone()))?;
            Ok(authorized(&token))
        }
        _ => Err(fail("invalidParams", format!("unknown oauth operation {op}"))),
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[tauri::command]
pub async fn flowkey_image(app: AppHandle, url: String, hosts: Vec<String>) -> Res<String> {
    let parsed = Url::parse(&url).map_err(|e| fail("invalidUrl", e.to_string()))?;
    url_allowed(&parsed, &hosts)?;
    let hash = hex(&Sha256::digest(url.as_bytes()));
    let dir = image_dir(&app);
    if let Some(ext) = IMAGE_TYPES.iter().map(|(_, ext)| *ext).find(|ext| dir.join(format!("{hash}.{ext}")).exists()) {
        return Ok(format!("http://extimg.localhost/{hash}.{ext}"));
    }
    let response = client(hosts, Duration::from_secs(30))?.get(parsed).send().await.map_err(network)?;
    if !response.status().is_success() {
        return Err(fail("httpError", format!("image request returned {}", response.status().as_u16())));
    }
    let content_type = response.headers().get(reqwest::header::CONTENT_TYPE).and_then(|v| v.to_str().ok()).unwrap_or_default().split(';').next().unwrap_or_default().trim().to_ascii_lowercase();
    let ext = IMAGE_TYPES.iter().find(|(mime, _)| *mime == content_type).map(|(_, ext)| *ext).ok_or_else(|| fail("notAnImage", format!("'{content_type}' is not a supported image type")))?;
    let (bytes, _) = read_body(response, true, MAX_IMAGE_BYTES).await?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{hash}.{ext}"));
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(tmp, path)?;
    Ok(format!("http://extimg.localhost/{hash}.{ext}"))
}

pub(crate) mod media {
    use super::{fail, now_secs, Res};
    use serde_json::{json, Value};
    use windows::Media::Control::{GlobalSystemMediaTransportControlsSessionManager as Manager, GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status};
    use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_MEDIA_NEXT_TRACK, VK_MEDIA_PLAY_PAUSE, VK_MEDIA_PREV_TRACK};

    const WINDOWS_TO_UNIX_100NS: i64 = 116_444_736_000_000_000;

    fn now_ms() -> i64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(now_secs() as i64 * 1000, |d| d.as_millis() as i64)
    }

    pub fn current() -> Res<Value> {
        let manager = Manager::RequestAsync()?.get()?;
        let Ok(session) = manager.GetCurrentSession() else { return Ok(json!({ "playing": false, "updatedAtMs": now_ms() })) };
        let properties = session.TryGetMediaPropertiesAsync()?.get()?;
        let playing = session.GetPlaybackInfo().and_then(|info| info.PlaybackStatus()).is_ok_and(|status| status == Status::Playing);
        let text = |value: windows::core::Result<windows::core::HSTRING>| value.ok().map(|s| s.to_string()).filter(|s| !s.is_empty());
        let mut position = None;
        let mut duration = None;
        if let Ok(timeline) = session.GetTimelineProperties() {
            let end = timeline.EndTime().map(|d| d.Duration / 10_000).unwrap_or(0);
            let start = timeline.StartTime().map(|d| d.Duration / 10_000).unwrap_or(0);
            if end > start {
                let mut at = timeline.Position().map(|d| d.Duration / 10_000).unwrap_or(0) - start;
                if playing {
                    if let Ok(updated) = timeline.LastUpdatedTime() {
                        at += (now_ms() - (updated.UniversalTime - WINDOWS_TO_UNIX_100NS) / 10_000).max(0);
                    }
                }
                duration = Some(end - start);
                position = Some(at.clamp(0, end - start));
            }
        }
        Ok(json!({
            "playing": playing,
            "title": text(properties.Title()),
            "artist": text(properties.Artist()),
            "album": text(properties.AlbumTitle()),
            "positionMs": position,
            "durationMs": duration,
            "updatedAtMs": now_ms(),
        }))
    }

    fn press(vk: VIRTUAL_KEY) {
        let key = |up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, dwFlags: if up { KEYEVENTF_KEYUP } else { Default::default() }, dwExtraInfo: crate::hotkey::MARKER, ..Default::default() } },
        };
        unsafe { SendInput(&[key(false), key(true)], std::mem::size_of::<INPUT>() as i32) };
    }

    pub fn control(command: &str) -> Res<()> {
        let session = Manager::RequestAsync().and_then(|op| op.get()).ok().and_then(|manager| manager.GetCurrentSession().ok());
        if let Some(session) = session {
            let operation = match command {
                "playPause" => session.TryTogglePlayPauseAsync(),
                "next" => session.TrySkipNextAsync(),
                "previous" => session.TrySkipPreviousAsync(),
                _ => return Err(fail("invalidParams", "command must be playPause, next or previous")),
            };
            if operation.and_then(|op| op.get()).unwrap_or(false) {
                return Ok(());
            }
        }
        press(match command {
            "playPause" => VK_MEDIA_PLAY_PAUSE,
            "next" => VK_MEDIA_NEXT_TRACK,
            "previous" => VK_MEDIA_PREV_TRACK,
            _ => return Err(fail("invalidParams", "command must be playPause, next or previous")),
        });
        Ok(())
    }
}

#[tauri::command]
pub async fn flowkey_media_current() -> Res<Value> {
    blocking(media::current).await
}

#[tauri::command]
pub async fn flowkey_media_control(command: String) -> Res<()> {
    blocking(move || media::control(&command)).await
}

fn fs_entry(path: &Path) -> Res<Value> {
    let meta = std::fs::metadata(path)?;
    Ok(json!({
        "path": path.to_string_lossy(),
        "name": path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
        "sizeBytes": meta.len(),
        "createdAtMs": millis(meta.created()),
        "modifiedAtMs": millis(meta.modified()),
    }))
}

fn walk(root: &Path, limit: usize) -> Vec<Value> {
    let mut entries = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(children) = std::fs::read_dir(&dir) else { continue };
        for child in children.flatten() {
            let Ok(kind) = child.file_type() else { continue };
            if kind.is_dir() {
                stack.push(child.path());
            } else if kind.is_file() {
                if let Ok(entry) = fs_entry(&child.path()) {
                    entries.push(entry);
                }
                if entries.len() >= limit {
                    return entries;
                }
            }
        }
    }
    entries
}

fn trash(path: &Path) -> Res<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::{SHFileOperationW, FOF_ALLOWUNDO, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT, FO_DELETE, SHFILEOPSTRUCTW};
    if !path.exists() {
        return Err(fail("notFound", format!("{} does not exist", path.display())));
    }
    let from: Vec<u16> = path.as_os_str().encode_wide().chain([0, 0]).collect();
    let mut operation = SHFILEOPSTRUCTW {
        wFunc: FO_DELETE,
        pFrom: PCWSTR(from.as_ptr()),
        fFlags: (FOF_ALLOWUNDO | FOF_NOCONFIRMATION | FOF_NOERRORUI | FOF_SILENT).0 as u16,
        ..Default::default()
    };
    let result = unsafe { SHFileOperationW(&mut operation) };
    if result != 0 || operation.fAnyOperationsAborted.as_bool() {
        return Err(fail("trashFailed", format!("could not move {} to the Recycle Bin (code {result})", path.display())));
    }
    Ok(())
}

fn fs_op(op: &str, path: &Path, to: Option<&Path>, content: Option<String>, append: bool, limit: usize) -> Res<Value> {
    let target = || to.ok_or_else(|| fail("invalidParams", "'to' is required"));
    match op {
        "readText" => {
            if std::fs::metadata(path)?.len() > MAX_BODY_BYTES {
                return Err(fail("fileTooLarge", "files larger than 16 MB cannot be read as text"));
            }
            Ok(json!({ "content": std::fs::read_to_string(path)? }))
        }
        "writeText" => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = std::fs::OpenOptions::new().create(true).write(true).append(append).truncate(!append).open(path)?;
            file.write_all(content.unwrap_or_default().as_bytes())?;
            Ok(json!({}))
        }
        "delete" => {
            if path.is_dir() {
                std::fs::remove_dir(path)?;
            } else {
                std::fs::remove_file(path)?;
            }
            Ok(json!({}))
        }
        "stat" => Ok(json!({ "entry": fs_entry(path)? })),
        "mkdir" => {
            std::fs::create_dir_all(path)?;
            Ok(json!({}))
        }
        "exists" => Ok(json!({ "exists": path.exists() })),
        "copy" => {
            std::fs::copy(path, target()?)?;
            Ok(json!({}))
        }
        "move" => {
            let to = target()?;
            if std::fs::rename(path, to).is_err() {
                std::fs::copy(path, to)?;
                std::fs::remove_file(path)?;
            }
            Ok(json!({}))
        }
        "trash" => trash(path).map(|()| json!({})),
        "walk" => Ok(json!({ "entries": walk(path, limit) })),
        _ => Err(fail("invalidParams", format!("unknown fs operation {op}"))),
    }
}

#[tauri::command]
pub async fn flowkey_fs(op: String, path: String, to: Option<String>, content: Option<String>, append: Option<bool>, limit: Option<usize>) -> Res<Value> {
    blocking(move || fs_op(&op, Path::new(&path), to.as_deref().map(Path::new), content, append.unwrap_or(false), limit.unwrap_or(20_000))).await
}

#[tauri::command]
pub fn flowkey_default_app(path: String) -> Option<String> {
    use windows::core::{w, HSTRING, PWSTR};
    use windows::Win32::UI::Shell::{AssocQueryStringW, ASSOCF_NONE, ASSOCSTR_EXECUTABLE};
    let ext = if path.starts_with('.') { path } else { format!(".{}", Path::new(&path).extension()?.to_string_lossy()) };
    let ext = HSTRING::from(ext);
    let mut len = 0u32;
    unsafe {
        let _ = AssocQueryStringW(ASSOCF_NONE, ASSOCSTR_EXECUTABLE, &ext, w!("open"), None, &mut len);
        if len == 0 {
            return None;
        }
        let mut buffer = vec![0u16; len as usize];
        AssocQueryStringW(ASSOCF_NONE, ASSOCSTR_EXECUTABLE, &ext, w!("open"), Some(PWSTR(buffer.as_mut_ptr())), &mut len).ok().ok()?;
        Some(String::from_utf16_lossy(&buffer[..len.saturating_sub(1) as usize]))
    }
}

fn pick(directory: bool, multiple: bool, extensions: Vec<String>) -> Res<Vec<String>> {
    use windows::core::{HSTRING, PCWSTR};
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::Common::COMDLG_FILTERSPEC;
    use windows::Win32::UI::Shell::{FileOpenDialog, IFileOpenDialog, FOS_ALLOWMULTISELECT, FOS_FORCEFILESYSTEM, FOS_PICKFOLDERS, SIGDN_FILESYSPATH};
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let dialog: IFileOpenDialog = CoCreateInstance(&FileOpenDialog, None, CLSCTX_INPROC_SERVER)?;
        let mut options = dialog.GetOptions()? | FOS_FORCEFILESYSTEM;
        if directory {
            options |= FOS_PICKFOLDERS;
        }
        if multiple {
            options |= FOS_ALLOWMULTISELECT;
        }
        dialog.SetOptions(options)?;
        let pattern = HSTRING::from(extensions.iter().map(|e| format!("*.{e}")).collect::<Vec<_>>().join(";"));
        if !directory && !extensions.is_empty() {
            dialog.SetFileTypes(&[COMDLG_FILTERSPEC { pszName: PCWSTR(pattern.as_ptr()), pszSpec: PCWSTR(pattern.as_ptr()) }])?;
        }
        if dialog.Show(None).is_err() {
            return Ok(Vec::new());
        }
        let items = dialog.GetResults()?;
        let mut paths = Vec::new();
        for index in 0..items.GetCount()? {
            let name = items.GetItemAt(index)?.GetDisplayName(SIGDN_FILESYSPATH)?;
            paths.push(name.to_string().unwrap_or_default());
            CoTaskMemFree(Some(name.0 as _));
        }
        Ok(paths)
    }
}

#[tauri::command]
pub async fn flowkey_pick_path(directory: bool, multiple: bool, extensions: Option<Vec<String>>) -> Res<Vec<String>> {
    blocking(move || pick(directory, multiple, extensions.unwrap_or_default())).await
}

#[tauri::command]
pub async fn flowkey_selected_text(app: AppHandle, allow_fallback: bool) -> Res<Option<String>> {
    use windows::Win32::System::DataExchange::GetClipboardSequenceNumber;
    if !allow_fallback {
        return Ok(None);
    }
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || crate::launcher::hide(&handle, true));
    blocking(|| {
        std::thread::sleep(Duration::from_millis(150));
        let previous = crate::clipboard::read_clipboard_text();
        let before = unsafe { GetClipboardSequenceNumber() };
        crate::clipboard::send_copy();
        let deadline = Instant::now() + Duration::from_millis(600);
        while unsafe { GetClipboardSequenceNumber() } == before && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        if unsafe { GetClipboardSequenceNumber() } == before {
            return Ok(None);
        }
        std::thread::sleep(Duration::from_millis(30));
        let text = crate::clipboard::read_clipboard_text();
        if let Some(previous) = previous {
            let _ = crate::clipboard::write_text(&previous);
        }
        Ok(text.filter(|t| !t.is_empty()))
    })
    .await
}

#[tauri::command]
pub fn copy_rich(text: Option<String>, html: Option<String>, paths: Option<Vec<String>>) -> Result<(), String> {
    crate::clipboard::write_rich(text.as_deref(), html.as_deref(), paths.as_deref())
}

#[tauri::command]
pub fn read_clipboard_rich() -> Value {
    let (text, html, paths) = crate::clipboard::read_rich();
    json!({ "text": text, "html": html, "paths": paths })
}

#[tauri::command]
pub fn flowkey_edit_clipboard_entry(id: String, history: tauri::State<crate::clipboard::History>) -> Result<(), String> {
    let entry = history.get(&id).ok_or("the clipboard entry no longer exists")?;
    let target = match &entry.content {
        crate::clipboard::Content::Text { text } => {
            let path = std::env::temp_dir().join(format!("openray-clipboard-{}.txt", hex(&Sha256::digest(id.as_bytes()))[..16].to_owned()));
            std::fs::write(&path, text).map_err(|e| e.to_string())?;
            path
        }
        crate::clipboard::Content::Files { paths } => PathBuf::from(paths.first().ok_or("the entry has no files")?),
        crate::clipboard::Content::Image { .. } => history.image_path(&id).ok_or("the image file is missing")?,
    };
    crate::apps::open_url(&target.to_string_lossy(), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zip_with(entries: &[(&str, &[u8])]) -> tempfile::NamedTempFile {
        let file = tempfile::NamedTempFile::new().unwrap();
        let mut writer = zip::ZipWriter::new(file.reopen().unwrap());
        for (name, bytes) in entries {
            writer.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
        file
    }

    #[test]
    fn installs_packages_and_rejects_zip_slip() {
        let root = tempfile::tempdir().unwrap();
        let good = zip_with(&[("manifest.json", br#"{"id":"demo"}"#), ("main.js", b"export default {}"), ("assets/icon.png", b"png")]);
        let dir = install_package(good.path().to_str().unwrap(), "demo", root.path()).unwrap();
        assert!(dir.join("assets/icon.png").exists());
        let evil = zip_with(&[("manifest.json", br#"{"id":"demo"}"#), ("../escape.js", b"x")]);
        assert_eq!(install_package(evil.path().to_str().unwrap(), "demo", root.path()).unwrap_err().code, "zipSlip");
        assert!(dir.join("main.js").exists());
        let mismatch = zip_with(&[("manifest.json", br#"{"id":"other"}"#)]);
        assert_eq!(install_package(mismatch.path().to_str().unwrap(), "demo", root.path()).unwrap_err().code, "idMismatch");
    }

    #[test]
    fn blocks_private_addresses_and_undeclared_hosts() {
        let hosts = vec!["api.spotify.com".to_string(), ".scdn.co".to_string(), "example.com:8443".to_string()];
        let check = |url: &str| url_allowed(&Url::parse(url).unwrap(), &hosts).map_err(|e| e.code);
        assert_eq!(check("https://api.spotify.com/v1/me"), Ok(()));
        assert_eq!(check("https://i.scdn.co/image/x"), Ok(()));
        assert_eq!(check("https://example.com:8443/"), Ok(()));
        assert_eq!(check("https://example.com/"), Err("hostNotAllowed".into()));
        assert_eq!(check("http://api.spotify.com/"), Err("insecureBlocked".into()));
        assert_eq!(check("https://user@api.spotify.com/"), Err("userInfoBlocked".into()));
        assert!(blocked_address("10.0.0.1".parse().unwrap()));
        assert!(blocked_address("192.168.1.10".parse().unwrap()));
        assert!(blocked_address("::ffff:127.0.0.1".parse().unwrap()));
        assert!(blocked_address("fe80::1".parse().unwrap()));
        assert!(!blocked_address("35.186.224.25".parse().unwrap()));
    }

    #[test]
    fn formats_rfc3339_dates() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(rfc3339(1_767_225_599), "2025-12-31T23:59:59Z");
    }

    #[test]
    fn oauth_callback_requires_matching_state() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let client = std::thread::spawn(move || {
            let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
            write!(stream, "GET /callback?state=wrong&code=abc HTTP/1.1\r\nHost: x\r\n\r\n").unwrap();
            let mut response = String::new();
            let _ = stream.read_to_string(&mut response);
            response
        });
        let result = wait_for_code(listener, "expected", "spotify", Duration::from_secs(5));
        assert_eq!(result.unwrap_err().code, "invalidState");
        assert!(client.join().unwrap().starts_with("HTTP/1.1 400"));
    }

    #[test]
    fn validates_ids_and_entries() {
        assert!(valid_id("spotify-player"));
        assert!(!valid_id("../x"));
        assert!(!valid_id("Spotify"));
        assert!(valid_entry("main.web.js"));
        assert!(!valid_entry("../main.js"));
    }
}
