use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId};
use std::sync::Arc;

pub const BG: Color32 = Color32::from_rgb(0x1F, 0x1F, 0x1F);
pub const FG: Color32 = Color32::WHITE;
pub const ACCENT: Color32 = Color32::from_rgb(0x4F, 0xA3, 0xF8);
pub const RED: Color32 = Color32::from_rgb(0xFF, 0x63, 0x63);
pub const GREEN: Color32 = Color32::from_rgb(0x59, 0xD4, 0x99);
pub const PANEL: Color32 = Color32::from_rgb(0x36, 0x36, 0x36);
pub const PANEL_BORDER: Color32 = Color32::from_rgb(0x5E, 0x5E, 0x5E);
pub const PANEL_LINE: Color32 = Color32::from_rgb(0x4A, 0x4A, 0x4A);
pub const WINDOW_BORDER: u32 = 0x003B_3B3B;

pub const fn fg(percent: u8) -> Color32 {
    let a = (255 * percent as u32 / 100) as u8;
    Color32::from_rgba_premultiplied(a, a, a, a)
}

pub const FG_60: Color32 = fg(60);
pub const FG_40: Color32 = fg(40);
pub const SEPARATOR: Color32 = fg(10);
pub const SELECTION: Color32 = fg(10);
pub const KEY_BG: Color32 = fg(10);
pub const KEY_BORDER: Color32 = fg(18);
pub const FOOTER_DIVIDER: Color32 = fg(20);
pub const CONTROL: Color32 = fg(10);
pub const CONTROL_HOVER: Color32 = fg(15);

pub const MEDIUM: &str = "inter-medium";
pub const SEMIBOLD: &str = "inter-semibold";
pub const BOLD: &str = "inter-bold";

pub fn font(size: f32) -> FontId {
    FontId::proportional(size)
}

pub fn weighted(size: f32, family: &str) -> FontId {
    FontId::new(size, FontFamily::Name(family.into()))
}

pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

fn system_font(name: &str) -> Option<FontData> {
    let dir = std::env::var_os("WINDIR").map(std::path::PathBuf::from).unwrap_or_else(|| r"C:\Windows".into());
    std::fs::read(dir.join("Fonts").join(name)).ok().map(FontData::from_owned)
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let defaults = fonts.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    let mut add = |name: &str, data: FontData| {
        fonts.font_data.insert(name.into(), Arc::new(data));
    };
    add("inter", FontData::from_static(include_bytes!("../assets/fonts/Inter-Regular.ttf")));
    add(MEDIUM, FontData::from_static(include_bytes!("../assets/fonts/Inter-Medium.ttf")));
    add(SEMIBOLD, FontData::from_static(include_bytes!("../assets/fonts/Inter-SemiBold.ttf")));
    add(BOLD, FontData::from_static(include_bytes!("../assets/fonts/Inter-Bold.ttf")));
    let fallbacks: Vec<String> = [("segoe-ui", "SegUIVar.ttf"), ("segoe-symbol", "seguisym.ttf")]
        .into_iter()
        .filter_map(|(name, file)| system_font(file).map(|data| (name, data)))
        .map(|(name, data)| {
            fonts.font_data.insert(name.into(), Arc::new(data));
            name.to_string()
        })
        .collect();
    let chain = |first: &str| [first.to_string()].into_iter().chain(fallbacks.iter().cloned()).chain(defaults.iter().cloned()).collect::<Vec<_>>();
    fonts.families.insert(FontFamily::Proportional, chain("inter"));
    for name in [MEDIUM, SEMIBOLD, BOLD] {
        fonts.families.insert(FontFamily::Name(name.into()), chain(name));
    }
    if let Some(data) = system_font("CascadiaMono.ttf").or_else(|| system_font("consola.ttf")) {
        fonts.font_data.insert("mono".into(), Arc::new(data));
        fonts.families.entry(FontFamily::Monospace).or_default().insert(0, "mono".into());
    }
    ctx.set_fonts(fonts);
    egui_extras::install_image_loaders(ctx);
    ctx.set_theme(egui::Theme::Dark);
    ctx.all_styles_mut(|style| {
        style.visuals = egui::Visuals::dark();
        style.visuals.panel_fill = BG;
        style.visuals.window_fill = BG;
        style.visuals.override_text_color = Some(FG);
        style.visuals.selection.bg_fill = ACCENT.gamma_multiply(0.45);
        style.visuals.selection.stroke.color = FG;
        style.visuals.text_cursor.stroke = egui::Stroke::new(1.5, FG);
        style.visuals.hyperlink_color = RED;
        style.spacing.item_spacing = egui::vec2(0.0, 0.0);
        style.interaction.selectable_labels = false;
    });
}
