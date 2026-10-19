use serde::Serialize;
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};

const REFRESH_AFTER: Duration = Duration::from_secs(60);
const LIMIT: usize = 100;
const RECENT_LIMIT: usize = 30;
const IGNORED_DIRS: [&str; 6] = ["node_modules", "tmp", "temp", "cache", "caches", "appdata"];
const HIDDEN: u32 = 0x2;
const SYSTEM: u32 = 0x4;
const REPARSE_POINT: u32 = 0x400;

const KINDS: [(&str, &[&str]); 6] = [
    ("archive", &["zip", "rar", "7z", "tar", "gz", "tgz", "bz2", "xz", "zst", "iso", "cab", "jar"]),
    ("document", &["pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp", "rtf", "epub", "pages", "numbers", "key"]),
    ("image", &["png", "jpg", "jpeg", "gif", "bmp", "webp", "svg", "ico", "tif", "tiff", "heic", "heif", "avif", "psd", "raw"]),
    ("video", &["mp4", "mkv", "mov", "avi", "wmv", "webm", "flv", "m4v", "mpg", "mpeg", "3gp"]),
    ("audio", &["mp3", "wav", "flac", "aac", "ogg", "m4a", "wma", "opus", "aiff", "mid", "midi"]),
    (
        "text",
        &[
            "txt", "md", "markdown", "log", "csv", "tsv", "json", "jsonc", "xml", "yaml", "yml", "toml", "ini", "cfg", "conf", "html", "htm", "css", "scss", "js", "jsx", "ts",
            "tsx", "mjs", "cjs", "py", "rs", "go", "java", "kt", "c", "h", "cpp", "hpp", "cs", "rb", "php", "swift", "sh", "ps1", "bat", "cmd", "sql", "lua", "dart", "vue",
        ],
    ),
];

struct Entry {
    path: Box<str>,
    name: Box<str>,
    dir: bool,
    modified: u64,
}

#[derive(Default)]
struct Index {
    entries: Arc<Vec<Entry>>,
    built: Option<Instant>,
    building: bool,
}

static INDEX: LazyLock<Mutex<Index>> = LazyLock::new(Default::default);

#[derive(Serialize)]
pub struct FileHit {
    path: String,
    name: String,
    dir: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Results {
    home: String,
    first_index: bool,
    building: bool,
    files: Vec<FileHit>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    type_name: String,
    dir: bool,
    size: Option<u64>,
    created: Option<u64>,
    modified: Option<u64>,
    accessed: Option<u64>,
}

fn home() -> PathBuf {
    std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Users\Public"))
}

fn millis(time: std::io::Result<SystemTime>) -> Option<u64> {
    time.ok()?.duration_since(UNIX_EPOCH).ok().map(|d| d.as_millis() as u64)
}

fn ignored(name: &str, dir: bool) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with('.') || lower.ends_with(".tmp") || lower.ends_with(".temp") || (dir && IGNORED_DIRS.contains(&lower.as_str()))
}

fn build(root: &Path) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(children) = std::fs::read_dir(&dir) else { continue };
        for child in children.flatten() {
            let Ok(meta) = child.metadata() else { continue };
            let name = child.file_name().to_string_lossy().into_owned();
            let attributes = meta.file_attributes();
            if attributes & (HIDDEN | SYSTEM) != 0 || ignored(&name, meta.is_dir()) {
                continue;
            }
            let path = child.path();
            if meta.is_dir() && attributes & REPARSE_POINT == 0 {
                stack.push(path.clone());
            }
            entries.push(Entry {
                path: path.to_string_lossy().into(),
                name: name.to_lowercase().into(),
                dir: meta.is_dir(),
                modified: millis(meta.modified()).unwrap_or(0),
            });
        }
    }
    entries
}

fn refresh(app: &AppHandle, force: bool) -> (Arc<Vec<Entry>>, bool, bool) {
    let mut index = INDEX.lock().unwrap();
    let stale = index.built.is_none_or(|built| built.elapsed() > REFRESH_AFTER);
    if (force || stale) && !index.building {
        index.building = true;
        let app = app.clone();
        std::thread::spawn(move || {
            let entries = build(&home());
            let mut index = INDEX.lock().unwrap();
            index.entries = Arc::new(entries);
            index.built = Some(Instant::now());
            index.building = false;
            drop(index);
            let _ = app.emit("file-index-updated", ());
        });
    }
    (index.entries.clone(), index.built.is_none(), index.building)
}

pub fn reindex(app: &AppHandle) {
    refresh(app, true);
}

fn kind_of(name: &str) -> Option<&'static str> {
    let ext = name.rsplit_once('.')?.1.to_lowercase();
    KINDS.iter().find(|(_, exts)| exts.contains(&ext.as_str())).map(|(kind, _)| *kind)
}

fn matches_kind(kind: &str, dir: bool, name: &str) -> bool {
    match kind {
        "directory" => dir,
        "file" => !dir,
        "other" => !dir && kind_of(name).is_none(),
        "allTypes" | "" => true,
        kind => !dir && kind_of(name) == Some(kind),
    }
}

fn score(name: &str, terms: &[String]) -> Option<u8> {
    if !terms.iter().all(|term| name.contains(term.as_str())) {
        return None;
    }
    let first = terms.first()?;
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    if name == first || stem == first {
        return Some(0);
    }
    if name.starts_with(first.as_str()) {
        return Some(1);
    }
    let word_start = name.match_indices(first.as_str()).any(|(at, _)| name[..at].chars().last().is_some_and(|c| !c.is_alphanumeric()));
    Some(if word_start { 2 } else { 3 })
}

fn display_name(path: &str) -> String {
    path.rsplit('\\').next().unwrap_or(path).to_string()
}

fn recent() -> Vec<FileHit> {
    let Some(appdata) = std::env::var_os("APPDATA") else { return Vec::new() };
    let Ok(links) = std::fs::read_dir(PathBuf::from(appdata).join(r"Microsoft\Windows\Recent")) else { return Vec::new() };
    let mut links: Vec<(u64, PathBuf)> = links
        .flatten()
        .filter(|link| link.path().extension().is_some_and(|ext| ext.eq_ignore_ascii_case("lnk")))
        .map(|link| (link.metadata().and_then(|m| m.modified()).ok().and_then(|t| millis(Ok(t))).unwrap_or(0), link.path()))
        .collect();
    links.sort_by(|a, b| b.0.cmp(&a.0));
    let mut hits: Vec<FileHit> = Vec::new();
    for (_, link) in links.into_iter().take(RECENT_LIMIT * 2) {
        let Some(target) = crate::focus::shortcut_target(&link.to_string_lossy()) else { continue };
        let Ok(meta) = std::fs::metadata(&target) else { continue };
        if hits.iter().any(|hit| hit.path.eq_ignore_ascii_case(&target)) {
            continue;
        }
        hits.push(FileHit { name: display_name(&target), path: target, dir: meta.is_dir() });
    }
    hits
}

pub fn search(app: &AppHandle, query: &str, kind: &str) -> Results {
    let (entries, first_index, building) = refresh(app, false);
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        let mut files: Vec<FileHit> = recent().into_iter().filter(|hit| matches_kind(kind, hit.dir, &hit.name)).take(RECENT_LIMIT).collect();
        let mut modified: Vec<&Entry> = entries.iter().filter(|entry| !entry.dir && matches_kind(kind, false, &entry.name)).collect();
        modified.sort_by(|a, b| b.modified.cmp(&a.modified));
        for entry in modified {
            if files.len() >= RECENT_LIMIT {
                break;
            }
            if !files.iter().any(|hit| hit.path.eq_ignore_ascii_case(&entry.path)) {
                files.push(FileHit { path: entry.path.to_string(), name: display_name(&entry.path), dir: false });
            }
        }
        return Results { home: home().to_string_lossy().into(), first_index, building, files };
    }
    let terms: Vec<String> = query.split_whitespace().map(String::from).collect();
    let mut hits: Vec<(u8, &Entry)> = entries
        .iter()
        .filter(|entry| matches_kind(kind, entry.dir, &entry.name))
        .filter_map(|entry| score(&entry.name, &terms).map(|s| (s, entry)))
        .collect();
    hits.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.modified.cmp(&a.1.modified)));
    let files = hits
        .into_iter()
        .take(LIMIT)
        .map(|(_, entry)| FileHit { path: entry.path.to_string(), name: display_name(&entry.path), dir: entry.dir })
        .collect();
    Results { home: home().to_string_lossy().into(), first_index, building, files }
}

fn type_name(path: &str) -> Option<String> {
    use windows::core::HSTRING;
    use windows::Win32::Storage::FileSystem::FILE_FLAGS_AND_ATTRIBUTES;
    use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_TYPENAME};
    let mut info = SHFILEINFOW::default();
    let ok = unsafe { SHGetFileInfoW(&HSTRING::from(path), FILE_FLAGS_AND_ATTRIBUTES(0), Some(&mut info), size_of::<SHFILEINFOW>() as u32, SHGFI_TYPENAME) };
    let len = info.szTypeName.iter().position(|&c| c == 0).unwrap_or(0);
    (ok != 0 && len > 0).then(|| String::from_utf16_lossy(&info.szTypeName[..len]))
}

pub fn info(path: &str) -> Result<FileInfo, String> {
    let meta = std::fs::metadata(path).map_err(|e| format!("{path} is not available: {e}"))?;
    Ok(FileInfo {
        type_name: type_name(path).unwrap_or_else(|| if meta.is_dir() { "Folder".into() } else { "File".into() }),
        dir: meta.is_dir(),
        size: (!meta.is_dir()).then(|| meta.len()),
        created: millis(meta.created()),
        modified: millis(meta.modified()),
        accessed: millis(meta.accessed()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn terms(query: &str) -> Vec<String> {
        query.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn ranks_exact_prefix_word_and_substring() {
        assert_eq!(score("report.pdf", &terms("report")), Some(0));
        assert_eq!(score("reports 2024.xlsx", &terms("report")), Some(1));
        assert_eq!(score("q3-report.docx", &terms("report")), Some(2));
        assert_eq!(score("myreport.txt", &terms("report")), Some(3));
        assert_eq!(score("budget 2024.xlsx", &terms("2024 bud")), Some(2));
        assert_eq!(score("budget.xlsx", &terms("budget 2024")), None);
    }

    #[test]
    fn filters_by_kind() {
        assert!(matches_kind("image", false, "photo.JPG"));
        assert!(!matches_kind("image", true, "photos.png"));
        assert!(matches_kind("directory", true, "photos"));
        assert!(matches_kind("other", false, "setup.exe"));
        assert!(!matches_kind("other", false, "notes.md"));
        assert!(matches_kind("allTypes", true, "x"));
    }

    #[test]
    fn indexes_and_skips_ignored_entries() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("Projects/node_modules/pkg")).unwrap();
        std::fs::create_dir_all(root.path().join(".git")).unwrap();
        std::fs::write(root.path().join("Projects/Plan.md"), "x").unwrap();
        std::fs::write(root.path().join("Projects/draft.tmp"), "x").unwrap();
        let mut names: Vec<String> = build(root.path()).into_iter().map(|e| e.name.to_string()).collect();
        names.sort();
        assert_eq!(names, ["plan.md", "projects"]);
    }
}
