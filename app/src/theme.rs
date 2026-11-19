use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

static LIGHT: AtomicBool = AtomicBool::new(false);

pub fn is_light() -> bool {
    LIGHT.load(Ordering::Relaxed)
}

fn pick(dark: Color32, light: Color32) -> Color32 {
    if is_light() { light } else { dark }
}

const fn gray(v: u8) -> Color32 {
    Color32::from_rgb(v, v, v)
}

#[allow(non_snake_case)]
pub fn BG() -> Color32 {
    pick(gray(0x1F), gray(0xE0))
}

#[allow(non_snake_case)]
pub fn FG() -> Color32 {
    pick(Color32::WHITE, Color32::BLACK)
}

#[allow(non_snake_case)]
pub fn ACCENT() -> Color32 {
    pick(Color32::from_rgb(0x4F, 0xA3, 0xF8), Color32::from_rgb(0x13, 0x8A, 0xF2))
}

#[allow(non_snake_case)]
pub fn RED() -> Color32 {
    pick(Color32::from_rgb(0xFF, 0x63, 0x63), Color32::from_rgb(0xB1, 0x24, 0x24))
}

#[allow(non_snake_case)]
pub fn GREEN() -> Color32 {
    pick(Color32::from_rgb(0x59, 0xD4, 0x99), Color32::from_rgb(0x00, 0x6B, 0x4F))
}

#[allow(non_snake_case)]
pub fn PANEL() -> Color32 {
    pick(gray(0x36), gray(0xF2))
}

#[allow(non_snake_case)]
pub fn PANEL_BORDER() -> Color32 {
    pick(gray(0x5E), gray(0xC2))
}

#[allow(non_snake_case)]
pub fn PANEL_LINE() -> Color32 {
    pick(gray(0x4A), gray(0xDA))
}

#[allow(non_snake_case)]
pub fn WINDOW_BORDER() -> u32 {
    if is_light() { 0x00C4_C4C4 } else { 0x003B_3B3B }
}

pub fn fg(percent: u8) -> Color32 {
    let a = (255 * percent as u32 / 100) as u8;
    if is_light() { Color32::from_rgba_premultiplied(0, 0, 0, a) } else { Color32::from_rgba_premultiplied(a, a, a, a) }
}

#[allow(non_snake_case)]
pub fn FG_60() -> Color32 {
    fg(60)
}

#[allow(non_snake_case)]
pub fn FG_40() -> Color32 {
    fg(40)
}

#[allow(non_snake_case)]
pub fn SEPARATOR() -> Color32 {
    fg(10)
}

#[allow(non_snake_case)]
pub fn SELECTION() -> Color32 {
    fg(10)
}

#[allow(non_snake_case)]
pub fn KEY_BG() -> Color32 {
    fg(10)
}

#[allow(non_snake_case)]
pub fn KEY_BORDER() -> Color32 {
    fg(18)
}

#[allow(non_snake_case)]
pub fn FOOTER_DIVIDER() -> Color32 {
    fg(20)
}

#[allow(non_snake_case)]
pub fn CONTROL() -> Color32 {
    fg(10)
}

#[allow(non_snake_case)]
pub fn CONTROL_HOVER() -> Color32 {
    fg(15)
}

pub fn system_light() -> bool {
    crate::registry::read_dword(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize", "AppsUseLightTheme") == Some(1)
}

pub fn sync(ctx: &egui::Context, appearance: crate::store::Appearance) {
    thread_local! {
        static SYSTEM: std::cell::Cell<Option<(std::time::Instant, bool)>> = const { std::cell::Cell::new(None) };
    }
    let light = match appearance {
        crate::store::Appearance::Light => true,
        crate::store::Appearance::Dark => false,
        crate::store::Appearance::System => SYSTEM.with(|cell| match cell.get() {
            Some((at, light)) if at.elapsed() < std::time::Duration::from_secs(2) => light,
            _ => {
                let light = system_light();
                cell.set(Some((std::time::Instant::now(), light)));
                light
            }
        }),
    };
    if LIGHT.swap(light, Ordering::Relaxed) != light || !STYLED.swap(true, Ordering::Relaxed) {
        style(ctx);
    }
}

static STYLED: AtomicBool = AtomicBool::new(false);

fn style(ctx: &egui::Context) {
    let light = is_light();
    ctx.set_theme(if light { egui::Theme::Light } else { egui::Theme::Dark });
    ctx.all_styles_mut(|style| {
        style.visuals = if light { egui::Visuals::light() } else { egui::Visuals::dark() };
        style.visuals.panel_fill = BG();
        style.visuals.window_fill = BG();
        style.visuals.override_text_color = Some(FG());
        style.visuals.selection.bg_fill = ACCENT().gamma_multiply(0.45);
        style.visuals.selection.stroke.color = FG();
        style.visuals.text_cursor.stroke = egui::Stroke::new(1.5, FG());
        style.visuals.hyperlink_color = RED();
        style.spacing.item_spacing = egui::vec2(0.0, 0.0);
        style.interaction.selectable_labels = false;
    });
    if let Some(hwnd) = crate::launcher::window() {
        crate::launcher::set_dwm(hwnd, windows::Win32::Graphics::Dwm::DWMWA_BORDER_COLOR, WINDOW_BORDER());
    }
}

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
    STYLED.store(false, Ordering::Relaxed);
    style(ctx);
}
