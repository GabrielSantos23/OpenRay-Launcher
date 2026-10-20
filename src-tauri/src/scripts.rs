use serde::Serialize;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const HEADER_LINES: usize = 60;

#[derive(Serialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct Script {
    pub path: String,
    pub title: String,
    pub mode: String,
    pub package_name: String,
    pub description: String,
    pub icon: String,
    pub needs_confirmation: bool,
    pub current_directory_path: String,
    pub arguments: Vec<serde_json::Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub code: Option<i32>,
}

pub fn parse(path: &str, text: &str) -> Option<Script> {
    let mut script = Script { path: path.to_string(), ..Default::default() };
    let mut schema = false;
    for line in text.lines().take(HEADER_LINES) {
        let Some(at) = line.find("@raycast.").or_else(|| line.find("@openray.")) else { continue };
        let rest = &line[at + 9..];
        let (key, value) = rest.split_once(char::is_whitespace).map_or((rest, ""), |(k, v)| (k, v.trim()));
        match key {
            "schemaVersion" => schema = value == "1",
            "title" => script.title = value.to_string(),
            "mode" => script.mode = value.to_string(),
            "packageName" => script.package_name = value.to_string(),
            "description" => script.description = value.to_string(),
            "icon" => script.icon = value.to_string(),
            "needsConfirmation" => script.needs_confirmation = value == "true",
            "currentDirectoryPath" => script.current_directory_path = value.to_string(),
            k if k.starts_with("argument") => {
                if let Ok(arg) = serde_json::from_str(value) {
                    script.arguments.push(arg);
                }
            }
            _ => {}
        }
    }
    (schema && !script.title.is_empty() && !script.mode.is_empty()).then_some(script)
}

pub fn list(directories: &[String]) -> Vec<Script> {
    let mut scripts = Vec::new();
    for dir in directories {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if !entry.file_type().is_ok_and(|t| t.is_file()) || interpreter(&path).is_none() {
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let text = String::from_utf8_lossy(&bytes[..bytes.len().min(16_384)]);
            if let Some(script) = parse(&path.to_string_lossy(), &text) {
                scripts.push(script);
            }
        }
    }
    scripts.sort_by_key(|s| s.title.to_lowercase());
    scripts
}

pub fn interpreter(path: &Path) -> Option<(&'static str, &'static [&'static str])> {
    let ext = path.extension()?.to_string_lossy().to_lowercase();
    Some(match ext.as_str() {
        "bat" | "cmd" => ("cmd.exe", &["/d", "/c"]),
        "ps1" => ("powershell.exe", &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"]),
        "py" => ("python", &[]),
        "js" | "mjs" => ("node", &[]),
        "ts" => ("npx.cmd", &["--yes", "tsx"]),
        "rb" => ("ruby", &[]),
        "sh" => ("bash", &[]),
        "go" => ("go", &["run"]),
        "cs" => ("dotnet", &["run"]),
        _ => return None,
    })
}

pub fn run(path: &str, arguments: &[String]) -> Result<Output, String> {
    let file = Path::new(path);
    if !file.exists() {
        return Err(format!("{path} no longer exists"));
    }
    let (program, prefix) = interpreter(file).ok_or("This script type isn't supported")?;
    let text = std::fs::read_to_string(file).unwrap_or_default();
    let cwd = parse(path, &text)
        .map(|s| s.current_directory_path)
        .filter(|d| !d.is_empty())
        .map(|d| crate::run::expand(&d))
        .filter(|d| Path::new(d).is_dir())
        .unwrap_or_else(|| file.parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default());
    let output = Command::new(program)
        .args(prefix)
        .arg(path)
        .args(arguments)
        .current_dir(cwd)
        .env("PYTHONIOENCODING", "utf-8")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Couldn't start {program}: {e}"))?;
    Ok(Output { stdout: String::from_utf8_lossy(&output.stdout).into_owned(), stderr: String::from_utf8_lossy(&output.stderr).into_owned(), code: output.status.code() })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCRIPT: &str = "# Required parameters:\n# @raycast.schemaVersion 1\n# @raycast.title Hello World\n# @raycast.mode compact\n# @raycast.packageName Dev Utils\n# @raycast.argument1 { \"type\": \"text\", \"placeholder\": \"Name\" }\n\nWrite-Output \"Hi $args\"\n";

    #[test]
    fn parses_metadata_comments() {
        let script = parse("C:\\s\\hello.ps1", SCRIPT).unwrap();
        assert_eq!(script.title, "Hello World");
        assert_eq!(script.mode, "compact");
        assert_eq!(script.package_name, "Dev Utils");
        assert_eq!(script.arguments.len(), 1);
        assert_eq!(script.arguments[0]["placeholder"], "Name");
        assert!(parse("x", &SCRIPT.replace("@raycast.title Hello World", "")).is_none());
        assert!(parse("x", &SCRIPT.replace("schemaVersion 1", "schemaVersion 2")).is_none());
        assert!(parse("x", "REM @raycast.schemaVersion 1\nREM @raycast.title T\nREM @raycast.mode silent").is_some());
    }

    #[test]
    fn picks_interpreters_by_extension() {
        assert_eq!(interpreter(Path::new("a.CMD")).unwrap().0, "cmd.exe");
        assert_eq!(interpreter(Path::new("a.ps1")).unwrap().0, "powershell.exe");
        assert!(interpreter(Path::new("a.txt")).is_none());
    }

    #[test]
    fn runs_a_batch_script_with_arguments() {
        let dir = std::env::temp_dir().join(format!("openray-script-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("hello.cmd");
        std::fs::write(&path, "@echo off\nREM @raycast.schemaVersion 1\nREM @raycast.title Hello\nREM @raycast.mode compact\necho Hello %1\n").unwrap();
        assert_eq!(list(&[dir.to_string_lossy().into_owned()]).len(), 1);
        let output = run(&path.to_string_lossy(), &["World".into()]).unwrap();
        assert_eq!(output.stdout.trim(), "Hello World");
        assert_eq!(output.code, Some(0));
        let _ = std::fs::remove_dir_all(dir);
    }
}
