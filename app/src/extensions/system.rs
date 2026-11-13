use super::command;
use crate::extension::{Argument, Command, Confirmation, Extension, Host, Icon, Tone};
use std::cell::Cell;
use std::rc::Rc;

pub struct System;

struct Entry {
    id: &'static str,
    title: &'static str,
    glyph: &'static str,
    keywords: &'static [&'static str],
    confirm: Option<(&'static str, &'static str, &'static str)>,
}

const CLOSE_WARNING: &str = "Open apps will close. Unsaved work may be lost.";

const ENTRIES: &[Entry] = &[
    Entry { id: "lock-screen", title: "Lock Screen", glyph: "lock", keywords: &["bloquear", "tela"], confirm: None },
    Entry { id: "log-out", title: "Log Out", glyph: "log-out", keywords: &["sair", "encerrar sessão"], confirm: Some(("Log Out?", CLOSE_WARNING, "Log Out")) },
    Entry { id: "restart", title: "Restart", glyph: "rotate-ccw", keywords: &["reboot", "reiniciar", "reinicializar"], confirm: Some(("Restart Your Computer?", CLOSE_WARNING, "Restart")) },
    Entry { id: "shut-down", title: "Shut Down", glyph: "power", keywords: &["desligar", "desligar computador"], confirm: Some(("Shut Down Your Computer?", CLOSE_WARNING, "Shut Down")) },
    Entry { id: "sleep", title: "Sleep", glyph: "moon", keywords: &["suspender", "dormir"], confirm: None },
    Entry { id: "hibernate", title: "Hibernate", glyph: "moon-star", keywords: &["hibernar"], confirm: None },
    Entry { id: "sleep-displays", title: "Sleep Displays", glyph: "monitor-off", keywords: &["suspender telas", "desligar telas"], confirm: None },
    Entry { id: "show-screen-saver", title: "Show Screen Saver", glyph: "sparkles", keywords: &["protetor de tela"], confirm: None },
    Entry { id: "show-desktop", title: "Show Desktop", glyph: "monitor", keywords: &["área de trabalho"], confirm: None },
    Entry { id: "open-recycle-bin", title: "Open Recycle Bin", glyph: "trash-2", keywords: &["trash", "lixeira", "abrir lixeira"], confirm: None },
    Entry {
        id: "empty-recycle-bin",
        title: "Empty Recycle Bin",
        glyph: "trash",
        keywords: &["trash", "esvaziar lixeira", "lixeira"],
        confirm: Some(("Empty Recycle Bin?", "Everything in the Recycle Bin will be deleted permanently.", "Empty Recycle Bin")),
    },
    Entry { id: "quit-active-app", title: "Quit the Active App", glyph: "x", keywords: &["close", "fechar", "encerrar"], confirm: None },
    Entry { id: "quit-all-apps", title: "Quit All Apps", glyph: "x", keywords: &["close", "fechar todos", "encerrar todos"], confirm: Some(("Quit All Apps?", "Every open app window will be asked to close.", "Quit All Apps")) },
    Entry {
        id: "quit-all-apps-except-frontmost",
        title: "Quit All Apps Except Frontmost",
        glyph: "x",
        keywords: &["close", "fechar outros", "exceto o ativo"],
        confirm: Some(("Quit All Other Apps?", "Every open app window except the active app's will be asked to close.", "Quit Apps")),
    },
    Entry { id: "turn-volume-up", title: "Turn Volume Up", glyph: "volume-2", keywords: &["sound", "audio", "aumentar volume", "som"], confirm: None },
    Entry { id: "turn-volume-down", title: "Turn Volume Down", glyph: "volume-1", keywords: &["sound", "audio", "diminuir volume", "som"], confirm: None },
    Entry { id: "toggle-mute", title: "Toggle Mute", glyph: "volume-x", keywords: &["sound", "audio", "mute", "mudo", "silenciar"], confirm: None },
    Entry { id: "set-volume-0", title: "Set Volume to 0%", glyph: "volume-x", keywords: &["sound", "audio", "volume", "definir volume"], confirm: None },
    Entry { id: "set-volume-25", title: "Set Volume to 25%", glyph: "volume-2", keywords: &["sound", "audio", "volume", "definir volume"], confirm: None },
    Entry { id: "set-volume-50", title: "Set Volume to 50%", glyph: "volume-2", keywords: &["sound", "audio", "volume", "definir volume"], confirm: None },
    Entry { id: "set-volume-75", title: "Set Volume to 75%", glyph: "volume-2", keywords: &["sound", "audio", "volume", "definir volume"], confirm: None },
    Entry { id: "set-volume-100", title: "Set Volume to 100%", glyph: "volume-2", keywords: &["sound", "audio", "volume", "definir volume"], confirm: None },
    Entry {
        id: "toggle-microphone-mute",
        title: "Toggle Microphone Mute",
        glyph: "mic-off",
        keywords: &["mute mic", "mute microphone", "unmute microphone", "mic mute", "silenciar microfone", "microfone"],
        confirm: None,
    },
    Entry { id: "play-pause", title: "Play / Pause", glyph: "play", keywords: &["music", "media", "tocar", "pausar", "mídia"], confirm: None },
    Entry { id: "next-track", title: "Next Track", glyph: "skip-forward", keywords: &["music", "media", "próxima faixa", "avançar"], confirm: None },
    Entry { id: "previous-track", title: "Previous Track", glyph: "skip-back", keywords: &["music", "media", "faixa anterior", "voltar"], confirm: None },
    Entry {
        id: "toggle-dark-mode",
        title: "Toggle Dark Mode",
        glyph: "sun-moon",
        keywords: &["theme", "dark mode", "light mode", "system appearance", "toggle system appearance", "modo escuro", "tema", "aparência"],
        confirm: None,
    },
    Entry { id: "toggle-hidden-files", title: "Toggle Hidden Files", glyph: "eye", keywords: &["arquivos ocultos"], confirm: None },
];

const TOGGLES: &[(&str, &str, &str, &[&str])] = &[
    ("toggle-night-light", "Toggle Night Light", "moon", &["night light", "blue light", "night mode", "luz noturna", "luz azul"]),
    ("toggle-hdr", "Toggle HDR", "sparkles", &["hdr", "high dynamic range", "display"]),
    ("toggle-camera-mute", "Toggle Camera Mute", "camera-off", &["mute camera", "webcam", "video mute", "camera off", "silenciar câmera", "câmera"]),
];

const ACTIVE_APP: &[(&str, &str, &str, &str, &[&str])] = &[
    ("toggle-active-app-mute", "Toggle Active App Mute", "volume-x", "mute", &["mute current app", "mute foreground app", "mute active app", "silenciar aplicativo ativo"]),
    ("turn-active-app-volume-up", "Turn Active App Volume Up", "volume-2", "up", &["current app volume up", "foreground app volume", "active app volume", "aumentar volume do aplicativo"]),
    ("turn-active-app-volume-down", "Turn Active App Volume Down", "volume-1", "down", &["current app volume down", "foreground app volume", "active app volume", "diminuir volume do aplicativo"]),
];

pub fn parse_volume(text: &str) -> Option<u8> {
    let value = text.trim().to_lowercase();
    let value = value.strip_suffix('%').unwrap_or(&value).trim();
    let valid = !value.is_empty() && value.chars().all(|c| c.is_ascii_digit() || c == '.') && value.matches('.').count() <= 1 && !value.starts_with('.') && !value.ends_with('.');
    valid.then(|| value.parse::<f64>().ok()).flatten().map(|v| v.round().min(100.0) as u8)
}

pub fn run_system_action(host: &mut Host, id: &str) -> Result<(), String> {
    let hidden = Cell::new(false);
    let result = crate::system_actions::run(&|| hidden.set(true), id);
    if hidden.get() {
        host.hide(false);
    }
    result
}

fn system_command(id: &str, title: &str, glyph: &'static str, keywords: &[&str], run: impl Fn(&mut Host) -> Result<(), String> + 'static) -> Command {
    let mut cmd = command("system", id, title, Icon::Tile(glyph, Tone::Red), "Command", "Run Command", run);
    cmd.subtitle = Some("System".into());
    cmd.keywords = ["system", "sistema"].iter().chain(keywords).map(|k| k.to_string()).collect();
    cmd
}

impl Extension for System {
    fn id(&self) -> &'static str {
        "system"
    }

    fn commands(&self) -> Vec<Command> {
        let mut commands: Vec<Command> = ENTRIES
            .iter()
            .map(|entry| {
                let id = entry.id;
                let mut cmd = system_command(id, entry.title, entry.glyph, entry.keywords, move |host| run_system_action(host, id));
                cmd.confirm = entry.confirm.map(|(title, message, confirm_title)| Confirmation { title: title.into(), message: message.into(), confirm_title: confirm_title.into() });
                cmd
            })
            .collect();
        commands.extend(TOGGLES.iter().map(|&(id, title, glyph, keywords)| {
            let cmd = system_command(id, title, glyph, keywords, move |host| {
                host.hud(crate::toggles::run(id));
                Ok(())
            });
            cmd
        }));
        let mut set_volume = system_command("set-volume", "Set Volume", "volume-2", &["sound", "audio", "volume", "som", "áudio"], |_| Ok(()));
        set_volume.argument = Some(Argument {
            placeholder: "Volume (0-100)".into(),
            optional: false,
            run: Rc::new(|text, host| {
                let Some(volume) = parse_volume(text) else {
                    return Err("Enter a number between 0 and 100.".into());
                };
                let result = crate::system_actions::run(&|| {}, &format!("set-volume-{volume}"));
                host.hud(result.map(|()| format!("Volume set to {volume}%")).map_err(|_| format!("Unable to set volume to {volume}%")));
                Ok(())
            }),
        });
        commands.push(set_volume);
        let mut mixer = system_command("volume-mixer", "Volume Mixer", "volume-2", &["app volume", "set app volume", "mute app", "audio mixer", "volume", "mixer de volume", "áudio"], |_| Ok(()));
        mixer.primary_action_title = "Open Command";
        mixer.view = Some(Rc::new(|ctx| Box::new(super::sound::VolumeMixer::new(ctx))));
        commands.push(mixer);
        commands.extend(ACTIVE_APP.iter().map(|&(id, title, glyph, action, keywords)| {
            system_command(id, title, glyph, keywords, move |host| {
                host.hud(crate::app_volume::change_active(action));
                Ok(())
            })
        }));
        for (id, title, glyph, input, keywords) in [
            ("set-sound-output-device", "Set Sound Output Device", "speaker", false, &["output device", "audio output", "playback device", "speakers", "headphones", "sound device", "dispositivo de saída", "alto-falantes", "áudio"][..]),
            ("set-sound-input-device", "Set Sound Input Device", "mic", true, &["input device", "audio input", "recording device", "microphone", "mic", "sound device", "dispositivo de entrada", "microfone", "áudio"][..]),
        ] {
            let mut cmd = system_command(id, title, glyph, keywords, |_| Ok(()));
            cmd.primary_action_title = "Open Command";
            cmd.view = Some(Rc::new(move |ctx| Box::new(super::sound::SoundDevices::new(ctx, input))));
            commands.push(cmd);
        }
        commands
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_volume_text() {
        assert_eq!(parse_volume("50"), Some(50));
        assert_eq!(parse_volume(" 75% "), Some(75));
        assert_eq!(parse_volume("33.6"), Some(34));
        assert_eq!(parse_volume("250"), Some(100));
        assert_eq!(parse_volume("-5"), None);
        assert_eq!(parse_volume("loud"), None);
        assert_eq!(parse_volume(""), None);
    }
}
