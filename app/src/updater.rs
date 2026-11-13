use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

const LATEST: &str = "https://github.com/GabrielSantos23/OpenRay-Launcher/releases/latest/download/latest.json";
const DOWNLOADS: &str = "https://github.com/GabrielSantos23/OpenRay-Launcher/releases/download/";
const PUBLIC_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEMxM0VDMUM5NzczOEQyNjYKUldSbTBqaDN5Y0Urd2FHMmcxdGpRVUZBVTZtckhJc21VS1RaOGEyRlFkNkVKN1NVbkFwSExMa28K";
const FIRST_CHECK: Duration = Duration::from_secs(15);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const MAX_PACKAGE: u64 = 128 * 1024 * 1024;
const STAGING: &str = ".update";
const OLD_EXE: &str = "openray.old.exe";

#[derive(Clone, PartialEq, Debug)]
pub enum State {
    Idle,
    Checking,
    UpToDate,
    Downloading(String),
    Ready(String),
    Failed(String),
}

pub static STATE: Mutex<State> = Mutex::new(State::Idle);

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn state() -> State {
    STATE.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

fn set(ctx: &egui::Context, next: State) {
    let ready = matches!(next, State::Ready(_));
    *STATE.lock().unwrap_or_else(|e| e.into_inner()) = next;
    if ready {
        crate::extensions::GENERATION.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
    ctx.request_repaint();
}

fn install_dir() -> Result<PathBuf, String> {
    std::env::current_exe().map_err(|e| e.to_string())?.parent().map(Path::to_path_buf).ok_or_else(|| "can't find Openray's folder".into())
}

pub fn start(ctx: &egui::Context) {
    if cfg!(debug_assertions) {
        return;
    }
    if let Ok(dir) = install_dir() {
        let _ = std::fs::remove_file(dir.join(OLD_EXE));
    }
    let ctx = ctx.clone();
    std::thread::spawn(move || {
        std::thread::sleep(FIRST_CHECK);
        loop {
            check(&ctx);
            std::thread::sleep(CHECK_EVERY);
        }
    });
}

pub fn check_now(ctx: &egui::Context) {
    if cfg!(debug_assertions) {
        return set(ctx, State::Failed("Updates are off in development builds.".into()));
    }
    let ctx = ctx.clone();
    std::thread::spawn(move || check(&ctx));
}

fn check(ctx: &egui::Context) {
    if matches!(state(), State::Checking | State::Downloading(_) | State::Ready(_)) {
        return;
    }
    set(ctx, State::Checking);
    let next = match run(ctx) {
        Ok(None) => State::UpToDate,
        Ok(Some(version)) => {
            crate::extension::notify_toast(ctx, crate::extension::Toast::Success(format!("Openray {version} is ready. Restart to update.")));
            State::Ready(version)
        }
        Err(error) => State::Failed(error),
    };
    set(ctx, next);
}

fn run(ctx: &egui::Context) -> Result<Option<String>, String> {
    let latest = match crate::http::get_json(LATEST) {
        Ok(Some(value)) => value,
        Ok(None) => return Ok(None),
        Err(crate::http::Failure::Offline) => return Err("Couldn't reach GitHub.".into()),
        Err(crate::http::Failure::Status(status)) => return Err(format!("GitHub answered {status}.")),
        Err(crate::http::Failure::Body(error)) => return Err(format!("The update information is invalid: {error}")),
    };
    let Some(next) = latest["version"].as_str() else { return Err("The update information has no version.".into()) };
    if !crate::flowkey_host::newer_version(next, version()) {
        return Ok(None);
    }
    let (Some(url), Some(signature)) = (latest["url"].as_str(), latest["signature"].as_str()) else { return Err("The update information is incomplete.".into()) };
    if !url.starts_with(DOWNLOADS) {
        return Err("The update is not hosted on Openray's GitHub releases.".into());
    }
    set(ctx, State::Downloading(next.to_owned()));
    let package = crate::http::download(url, MAX_PACKAGE, Duration::from_secs(300))?;
    verify(&package, signature)?;
    let staging = install_dir()?.join(STAGING);
    let _ = std::fs::remove_dir_all(&staging);
    extract(&package, &staging)?;
    if !staging.join("openray.exe").is_file() {
        return Err("The update package has no openray.exe.".into());
    }
    Ok(Some(next.to_owned()))
}

fn decoded(base64: &str) -> Result<String, String> {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD.decode(base64.trim()).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

fn verify_with(data: &[u8], signature: &str, key: &str) -> Result<(), String> {
    let key = minisign_verify::PublicKey::decode(&decoded(key)?).map_err(|e| format!("bad public key: {e}"))?;
    let signature = minisign_verify::Signature::decode(&decoded(signature)?).map_err(|e| format!("bad signature: {e}"))?;
    key.verify(data, &signature, true).map_err(|_| "The update's signature doesn't match. It was not installed.".to_string())
}

fn verify(data: &[u8], signature: &str) -> Result<(), String> {
    verify_with(data, signature, PUBLIC_KEY)
}

fn extract(package: &[u8], target: &Path) -> Result<(), String> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(package)).map_err(|e| format!("not a zip archive: {e}"))?;
    if zip.len() > 512 {
        return Err("The update package has too many files.".into());
    }
    for index in 0..zip.len() {
        let mut file = zip.by_index(index).map_err(|e| e.to_string())?;
        let Some(relative) = file.enclosed_name() else { return Err(format!("unsafe path in the update: {}", file.name())) };
        let out = target.join(relative);
        if file.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut writer = std::fs::File::create(&out).map_err(|e| e.to_string())?;
        std::io::copy(&mut file, &mut writer).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn files(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files(&path, into);
        } else {
            into.push(path);
        }
    }
}

fn swap(dir: &Path) -> Result<(), String> {
    let staging = dir.join(STAGING);
    let exe = dir.join("openray.exe");
    let old = dir.join(OLD_EXE);
    let _ = std::fs::remove_file(&old);
    // A running exe can be renamed but not overwritten, so the current one steps aside first.
    if exe.exists() {
        std::fs::rename(&exe, &old).map_err(|e| format!("Couldn't replace openray.exe: {e}"))?;
    }
    let mut staged = Vec::new();
    files(&staging, &mut staged);
    for source in staged {
        let target = dir.join(source.strip_prefix(&staging).map_err(|e| e.to_string())?);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let _ = std::fs::remove_file(&target);
        if let Err(error) = std::fs::rename(&source, &target) {
            if !exe.exists() && old.exists() {
                let _ = std::fs::rename(&old, &exe);
            }
            return Err(format!("Couldn't install {}: {error}", target.display()));
        }
    }
    let _ = std::fs::remove_dir_all(&staging);
    Ok(())
}

pub fn apply() -> Result<(), String> {
    if !matches!(state(), State::Ready(_)) {
        return Err("No update is ready.".into());
    }
    let dir = install_dir()?;
    swap(&dir)?;
    std::process::Command::new(dir.join("openray.exe")).args(["--updated", "--show"]).spawn().map_err(|e| format!("Updated, but couldn't restart Openray: {e}"))?;
    Ok(())
}

pub fn describe(state: &State) -> String {
    match state {
        State::Idle => format!("Openray {}", version()),
        State::Checking => "Checking for updates…".into(),
        State::UpToDate => format!("Openray {} is up to date.", version()),
        State::Downloading(next) => format!("Downloading Openray {next}…"),
        State::Ready(next) => format!("Openray {next} is ready. Restart to finish updating."),
        State::Failed(error) => format!("Couldn't check for updates. {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn the_public_key_decodes() {
        assert!(minisign_verify::PublicKey::decode(&decoded(PUBLIC_KEY).unwrap()).is_ok());
    }

    #[test]
    fn rejects_bad_signatures() {
        assert!(verify(b"data", "not base64!").is_err());
        let fake = base64_of("untrusted comment: x\nRUQAAAAAAAAAAA==\ntrusted comment: y\nAAAA\n");
        assert!(verify(b"data", &fake).is_err());
    }

    #[test]
    fn accepts_a_tauri_style_signature_from_the_right_key() {
        let pair = minisign::KeyPair::generate_unencrypted_keypair().unwrap();
        let key = base64_of(&pair.pk.to_box().unwrap().to_string());
        let data = b"openray package";
        let signature = minisign::sign(Some(&pair.pk), &pair.sk, std::io::Cursor::new(data), None, None).unwrap();
        let signature = base64_of(&signature.to_string());
        assert_eq!(verify_with(data, &signature, &key), Ok(()));
        assert!(verify_with(b"tampered", &signature, &key).is_err());
        assert!(verify(data, &signature).is_err());
    }

    fn base64_of(text: &str) -> String {
        use base64::Engine;
        base64::engine::general_purpose::STANDARD.encode(text)
    }

    #[test]
    fn swaps_staged_files_into_place() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("openray.exe"), b"old").unwrap();
        std::fs::create_dir_all(root.join("extensions/demo")).unwrap();
        std::fs::write(root.join("extensions/demo/main.js"), b"old").unwrap();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, body) in [("openray.exe", b"new".as_slice()), ("extensions/demo/main.js", b"new js")] {
            zip.start_file(name, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(body).unwrap();
        }
        let package = zip.finish().unwrap().into_inner();
        extract(&package, &root.join(STAGING)).unwrap();
        swap(root).unwrap();
        assert_eq!(std::fs::read(root.join("openray.exe")).unwrap(), b"new");
        assert_eq!(std::fs::read(root.join(OLD_EXE)).unwrap(), b"old");
        assert_eq!(std::fs::read(root.join("extensions/demo/main.js")).unwrap(), b"new js");
        assert!(!root.join(STAGING).exists());
    }

    #[test]
    fn refuses_zip_slip() {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file("../evil.exe", zip::write::SimpleFileOptions::default()).unwrap();
        zip.write_all(b"x").unwrap();
        let package = zip.finish().unwrap().into_inner();
        let dir = tempfile::tempdir().unwrap();
        assert!(extract(&package, &dir.path().join("stage")).is_err());
    }
}
