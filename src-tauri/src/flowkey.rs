use crate::flowkey_core::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tauri::http::{Request, Response};
use tauri::{AppHandle, Manager};

type Res<T> = Result<T, FkError>;

fn data_base(app: &AppHandle) -> Res<PathBuf> {
    app.path().app_data_dir().map_err(|e| fail("ioError", e.to_string()))
}

fn cache_dir(app: &AppHandle) -> PathBuf {
    app.path().app_cache_dir().unwrap_or_else(|_| std::env::temp_dir())
}

#[tauri::command]
pub fn flowkey_list(app: AppHandle) -> Vec<Listed> {
    list(app.path().resource_dir().ok().as_deref())
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
    let resources = app.path().resource_dir().ok();
    let Some(base) = root(source, resources.as_deref()).filter(|_| valid_id(id)).map(|dir| dir.join(id)) else { return not_found() };
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
    let path = image_dir(&cache_dir(app)).join(name);
    match std::fs::read(&path) {
        Ok(bytes) => respond(200, content_type(&path), bytes, None),
        Err(_) => not_found(),
    }
}

#[tauri::command]
pub fn flowkey_package_manifest(path: String) -> Res<Value> {
    package_manifest(&path)
}

#[tauri::command]
pub async fn flowkey_install(path: String, id: String) -> Res<String> {
    blocking(move || install_package(&path, &id, &installed_root()).map(|dir| dir.to_string_lossy().into_owned())).await
}

#[tauri::command]
pub fn flowkey_uninstall(app: AppHandle, id: String) -> Res<()> {
    uninstall(&data_base(&app)?, &id)
}

#[tauri::command]
pub fn flowkey_data_get(app: AppHandle, extension_id: String, name: String) -> Res<Value> {
    data_get(&data_base(&app)?, &extension_id, &name)
}

#[tauri::command]
pub fn flowkey_data_set(app: AppHandle, extension_id: String, name: String, value: Value) -> Res<()> {
    data_set(&data_base(&app)?, &extension_id, &name, &value)
}

#[tauri::command]
pub fn flowkey_secret(app: AppHandle, extension_id: String, op: String, key: String, value: Option<String>, max_keys: Option<usize>) -> Res<Option<String>> {
    secret(&data_base(&app)?, &extension_id, &op, key, value, max_keys)
}

#[tauri::command]
pub async fn flowkey_http(app: AppHandle, extension_id: String, request: HttpRequest, hosts: Vec<String>) -> Res<Value> {
    http(data_base(&app)?, extension_id, request, hosts).await
}

#[tauri::command]
pub async fn flowkey_oauth(app: AppHandle, extension_id: String, op: String, provider: String, client_id: Option<String>) -> Res<Value> {
    oauth(data_base(&app)?, extension_id, op, provider, client_id).await
}

#[tauri::command]
pub async fn flowkey_image(app: AppHandle, url: String, hosts: Vec<String>) -> Res<String> {
    image(cache_dir(&app), url, hosts).await
}

#[tauri::command]
pub async fn flowkey_media_current() -> Res<Value> {
    blocking(media::current).await
}

#[tauri::command]
pub async fn flowkey_media_control(command: String) -> Res<()> {
    blocking(move || media::control(&command)).await
}

#[tauri::command]
pub async fn flowkey_fs(op: String, path: String, to: Option<String>, content: Option<String>, append: Option<bool>, limit: Option<usize>) -> Res<Value> {
    blocking(move || fs_op(&op, Path::new(&path), to.as_deref().map(Path::new), content, append.unwrap_or(false), limit.unwrap_or(20_000))).await
}

#[tauri::command]
pub fn flowkey_default_app(path: String) -> Option<String> {
    default_app(path)
}

#[tauri::command]
pub async fn flowkey_pick_path(directory: bool, multiple: bool, extensions: Option<Vec<String>>) -> Res<Vec<String>> {
    blocking(move || pick(directory, multiple, extensions.unwrap_or_default())).await
}

#[tauri::command]
pub async fn flowkey_selected_text(app: AppHandle, allow_fallback: bool) -> Res<Option<String>> {
    if !allow_fallback {
        return Ok(None);
    }
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || crate::launcher::hide(&handle, true));
    blocking(|| Ok(copy_selection())).await
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
