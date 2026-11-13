use crate::app_volume::{self, AppAudio};
use crate::extension::{Accessory, Action, Empty, Icon, Item, Load, Section, Shortcut, Tone, View};
use crate::sound_devices::{self, SoundDevice};
use egui::Key;
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

const LEVELS: [u8; 5] = [0, 25, 50, 75, 100];

pub fn sort_apps(apps: &mut [AppAudio]) {
    apps.sort_by(|a, b| b.is_playing.cmp(&a.is_playing).then((a.id == "system").cmp(&(b.id == "system"))).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
}

pub fn sort_devices(devices: &mut [SoundDevice]) {
    let first = |d: &SoundDevice| d.state == "active" && d.is_default;
    devices.sort_by(|a, b| first(b).cmp(&first(a)).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
}

fn device_glyph(input: bool, form_factor: u32) -> &'static str {
    match form_factor {
        1 => "speaker",
        2 => "cable",
        3 => "headphones",
        4 => "mic",
        5 | 6 => "headset",
        9 => "monitor",
        _ if input => "mic",
        _ => "speaker",
    }
}

type Change = Rc<dyn Fn(Option<f32>, Option<bool>) -> Result<(), String>>;

fn volume_actions(input: bool, name: &str, volume: f32, muted: bool, change: Change) -> Vec<Vec<Action>> {
    let (on, off) = if input { ("mic", "mic-off") } else { ("volume-2", "volume-x") };
    let set = |values: (Option<f32>, Option<bool>), fail: String| {
        let change = change.clone();
        move |_: &mut crate::extension::Host| change(values.0, values.1).map_err(|e| format!("{fail}: {e}"))
    };
    let toggle_fail = format!("Unable to {} {name}", if muted { "unmute" } else { "mute" });
    let volume_fail = format!("Unable to set {name} volume");
    vec![
        vec![Action::new("toggle-mute", if muted { "Unmute" } else { "Mute" }, set((None, Some(!muted)), toggle_fail)).icon(if muted { on } else { off }).shortcut(Shortcut::key(Key::M).ctrl()).keep_open()],
        vec![
            Action::new("increase-volume", "Increase Volume", set((Some(app_volume::step(volume, true)), None), volume_fail.clone())).icon("volume-2").shortcut(Shortcut::key(Key::Equals)).keep_open(),
            Action::new("decrease-volume", "Decrease Volume", set((Some(app_volume::step(volume, false)), None), volume_fail.clone())).icon("volume-1").shortcut(Shortcut::key(Key::Minus)).keep_open(),
        ],
        LEVELS
            .iter()
            .map(|&level| {
                let mut action = Action::new("set-volume", format!("Set Volume to {level}%"), set((Some(level as f32), None), volume_fail.clone())).icon(if level > 0 { on } else { off }).keep_open();
                if !muted && volume.round() as u8 == level {
                    action.accessory = Some("Current".into());
                }
                action
            })
            .collect(),
    ]
}

pub struct VolumeMixer {
    load: Load<Arc<Vec<AppAudio>>>,
    dirty: Rc<Cell<bool>>,
    ctx: egui::Context,
}

fn list_apps() -> Result<Arc<Vec<AppAudio>>, String> {
    let mut apps = app_volume::list()?;
    sort_apps(&mut apps);
    Ok(Arc::new(apps))
}

impl VolumeMixer {
    pub fn new(ctx: &egui::Context) -> Self {
        Self { load: Load::start(ctx, list_apps), dirty: Rc::new(Cell::new(false)), ctx: ctx.clone() }
    }
}

impl View for VolumeMixer {
    fn placeholder(&self) -> String {
        "Search apps…".into()
    }

    fn footer(&self) -> (String, Icon) {
        ("Volume Mixer".into(), Icon::Tile("volume-2", Tone::Red))
    }

    fn loading(&self) -> bool {
        self.load.get().is_none()
    }

    fn empty(&self) -> Empty {
        match self.load.get() {
            Some(Err(e)) => Empty { title: "Could not list app volumes".into(), description: Some(e) },
            _ => Empty { title: "No apps are playing audio".into(), description: None },
        }
    }

    fn sections(&mut self) -> Vec<Section> {
        if self.dirty.replace(false) {
            self.load = Load::start(&self.ctx, list_apps);
        }
        let Some(Ok(apps)) = self.load.get() else { return Vec::new() };
        let item = |app: &AppAudio| {
            let (id, dirty) = (app.id.clone(), self.dirty.clone());
            let change: Change = Rc::new(move |volume, muted| {
                dirty.set(true);
                app_volume::set(&id, volume, muted)
            });
            Item {
                id: app.id.clone(),
                title: app.name.clone(),
                subtitle: None,
                icon: app.path.clone().map_or(Icon::Glyph("speaker"), Icon::File),
                accessories: vec![Accessory::Volume { level: app.volume, muted: app.is_muted, input: false }],
                keywords: Vec::new(),
                actions: volume_actions(false, &app.name, app.volume, app.is_muted, change),
            }
        };
        [("Playing", true), ("Idle", false)]
            .into_iter()
            .map(|(title, playing)| Section { title: title.into(), items: apps.iter().filter(|a| a.is_playing == playing).map(item).collect() })
            .filter(|s| !s.items.is_empty())
            .collect()
    }
}

pub struct SoundDevices {
    input: bool,
    load: Load<Arc<Vec<SoundDevice>>>,
    dirty: Rc<Cell<bool>>,
    ctx: egui::Context,
}

fn list_devices(input: bool) -> Result<Arc<Vec<SoundDevice>>, String> {
    let mut devices = sound_devices::list(if input { "input" } else { "output" })?;
    sort_devices(&mut devices);
    Ok(Arc::new(devices))
}

impl SoundDevices {
    pub fn new(ctx: &egui::Context, input: bool) -> Self {
        Self { input, load: Load::start(ctx, move || list_devices(input)), dirty: Rc::new(Cell::new(false)), ctx: ctx.clone() }
    }

    fn label(&self) -> &'static str {
        if self.input {
            "Input Device"
        } else {
            "Output Device"
        }
    }

    fn actions(&self, device: &SoundDevice) -> Vec<Vec<Action>> {
        let mut groups = Vec::new();
        let label = self.label();
        if device.state == "active" {
            if !device.is_default {
                let (id, name, dirty) = (device.id.clone(), device.name.clone(), self.dirty.clone());
                groups.push(vec![Action::new("select", format!("Set as {label}"), move |host| {
                    dirty.set(true);
                    sound_devices::set_default(&id).map_err(|e| format!("Unable to set {label} to {name}: {e}"))?;
                    host.toast = Some(crate::extension::Toast::Success(format!("{label} set to {name}")));
                    Ok(())
                })
                .icon(device_glyph(self.input, device.form_factor))
                .keep_open()]);
            }
            let (id, dirty) = (device.id.clone(), self.dirty.clone());
            let change: Change = Rc::new(move |volume, muted| {
                dirty.set(true);
                if let Some(volume) = volume {
                    sound_devices::set_volume(&id, volume)?;
                }
                if let Some(muted) = muted {
                    sound_devices::set_muted(&id, muted)?;
                }
                Ok(())
            });
            groups.extend(volume_actions(self.input, &device.name, device.volume, device.is_muted, change));
        }
        let name = device.name.clone();
        groups.push(vec![Action::new("copy-name", "Copy Device Name", move |host| {
            host.copy(&name, "device name");
            Ok(())
        })
        .icon("copy")
        .shortcut(Shortcut::key(Key::C).ctrl().shift())
        .keep_open()]);
        let dirty = self.dirty.clone();
        groups.push(vec![
            Action::new("refresh", "Refresh Devices", move |_| {
                dirty.set(true);
                Ok(())
            })
            .icon("refresh-cw")
            .shortcut(Shortcut::key(Key::R).ctrl())
            .keep_open(),
            Action::new("open-settings", "Open Sound Settings", |_| crate::apps::open_url("ms-settings:sound", None)).icon("settings").shortcut(Shortcut::key(Key::O).ctrl().shift()),
        ]);
        groups
    }
}

impl View for SoundDevices {
    fn placeholder(&self) -> String {
        "Search devices…".into()
    }

    fn footer(&self) -> (String, Icon) {
        if self.input {
            ("Set Sound Input Device".into(), Icon::Tile("mic", Tone::Red))
        } else {
            ("Set Sound Output Device".into(), Icon::Tile("speaker", Tone::Red))
        }
    }

    fn loading(&self) -> bool {
        self.load.get().is_none()
    }

    fn empty(&self) -> Empty {
        match self.load.get() {
            Some(Err(e)) => Empty { title: "Could not list sound devices".into(), description: Some(e) },
            _ => Empty { title: format!("No {}s found", self.label().to_lowercase()), description: None },
        }
    }

    fn sections(&mut self) -> Vec<Section> {
        if self.dirty.replace(false) {
            let input = self.input;
            self.load = Load::start(&self.ctx, move || list_devices(input));
        }
        let Some(Ok(devices)) = self.load.get() else { return Vec::new() };
        [("active", "Available"), ("disabled", "Disabled"), ("disconnected", "Disconnected")]
            .into_iter()
            .map(|(state, title)| Section {
                title: title.into(),
                items: devices
                    .iter()
                    .filter(|d| d.state == state)
                    .map(|device| Item {
                        id: device.id.clone(),
                        title: device.name.clone(),
                        subtitle: None,
                        icon: Icon::Glyph(device_glyph(self.input, device.form_factor)),
                        accessories: if device.state == "active" {
                            let volume = Accessory::Volume { level: device.volume, muted: device.is_muted, input: self.input };
                            if device.is_default {
                                vec![Accessory::Tag("Default".into()), volume]
                            } else {
                                vec![volume]
                            }
                        } else {
                            Vec::new()
                        },
                        keywords: Vec::new(),
                        actions: self.actions(device),
                    })
                    .collect(),
            })
            .filter(|s| !s.items.is_empty())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_and_sorts_like_the_web_version() {
        assert_eq!([app_volume::step(80.0, true), app_volume::step(82.0, true), app_volume::step(82.0, false), app_volume::step(80.0, false), app_volume::step(98.0, true), app_volume::step(0.0, false)], [85.0, 85.0, 80.0, 75.0, 100.0, 0.0]);
        let device = |name: &str, is_default| SoundDevice { id: name.into(), name: name.into(), state: "active", form_factor: 1, is_default, volume: 50.0, is_muted: false };
        let mut devices = vec![device("B", false), device("C", true), device("A", false)];
        sort_devices(&mut devices);
        assert_eq!(devices.iter().map(|d| d.name.as_str()).collect::<Vec<_>>(), ["C", "A", "B"]);
        let app = |id: &str, name: &str, is_playing| AppAudio { id: id.into(), name: name.into(), path: None, volume: 50.0, is_muted: false, is_playing };
        let mut apps = vec![app("system", "System Sounds", false), app("b", "Zed", false), app("a", "Spotify", true)];
        sort_apps(&mut apps);
        assert_eq!(apps.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), ["Spotify", "Zed", "System Sounds"]);
    }
}
