use crate::extension::{Icon, Tone};
use crate::theme;
use egui::load::{ImageLoadResult, ImageLoader, ImagePoll, LoadError, SizeHint};
use egui::{pos2, vec2, Color32, ColorImage, Mesh, Rect, Sense, Shape, Stroke, Ui};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

const APP_ICON: &str = "appicon://";
const CLIP_IMAGE: &str = "clipimg://";
const EMOJI: &str = "emoji://";
pub const TIME_WIDTH: f32 = 122.0;

include!(concat!(env!("OUT_DIR"), "/icons.rs"));

pub fn has_glyph(name: &str) -> bool {
    svg_bytes(name).is_some()
}

pub fn paint_glyph(ui: &Ui, name: &str, rect: Rect, tint: Color32) {
    if name == "openray" {
        let stroke = Stroke::new(rect.width() * 2.5 / 24.0, tint);
        let scale = rect.width() / 24.0;
        let at = |x: f32, y: f32| rect.min + vec2(x, y) * scale;
        ui.painter().circle_stroke(at(12.0, 12.0), 8.0 * scale, stroke);
        ui.painter().line_segment([at(12.0, 12.0), at(21.0, 3.0)], stroke);
        return;
    }
    if let Some(bytes) = svg_bytes(name) {
        egui::Image::from_bytes(format!("bytes://{name}.svg"), bytes).tint(tint).paint_at(ui, rect);
    }
}

pub fn tile(ui: &mut Ui, glyph_name: &str, tone: Tone, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    let (top, bottom) = tone.gradient();
    let radius = if size >= 22.0 { 6.0 } else { 4.0 };
    let mut outline = Vec::new();
    egui::epaint::tessellator::path::rounded_rectangle(&mut outline, egui::epaint::RoundedRect::new(rect, radius));
    let mut mesh = Mesh::default();
    let shade = |p: egui::Pos2| top.lerp_to_gamma(bottom, ((p.y - rect.top()) / rect.height()).clamp(0.0, 1.0));
    mesh.colored_vertex(rect.center(), shade(rect.center()));
    for p in &outline {
        mesh.colored_vertex(*p, shade(*p));
    }
    let n = outline.len() as u32;
    for i in 0..n {
        mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
    }
    let painter = ui.painter();
    painter.add(Shape::mesh(mesh));
    painter.add(Shape::rect_stroke(rect, radius, Stroke::new(0.5, Color32::from_black_alpha(51)), egui::StrokeKind::Inside));
    painter.line_segment([pos2(rect.left() + radius, rect.top() + 0.25), pos2(rect.right() - radius, rect.top() + 0.25)], Stroke::new(0.5, Color32::from_white_alpha(89)));
    let glyph_size = if size >= 22.0 { 14.0 } else { 10.0 };
    paint_glyph(ui, glyph_name, Rect::from_center_size(rect.center(), vec2(glyph_size, glyph_size)), Color32::WHITE);
}

pub fn command_icon(ui: &mut Ui, icon: &Icon, title: &str, size: f32) {
    match icon {
        Icon::Tile(name, tone) => tile(ui, name, *tone, size),
        Icon::Glyph(name) => {
            let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
            paint_glyph(ui, name, Rect::from_center_size(rect.center(), vec2(16.0, 16.0)), theme::FG_60);
        }
        Icon::Boxed(name) => {
            let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
            ui.painter().rect_filled(rect, 5.0, theme::KEY_BG);
            paint_glyph(ui, name, Rect::from_center_size(rect.center(), vec2(14.0, 14.0)), theme::FG_60);
        }
        Icon::Dot(color) => {
            let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
            ui.painter().circle_filled(rect.center(), 4.0, *color);
        }
        Icon::Time(color, text, dim) => {
            let (rect, _) = ui.allocate_exact_size(vec2(TIME_WIDTH, size), Sense::hover());
            let alpha = if *dim { 0.5 } else { 1.0 };
            ui.painter().circle_filled(pos2(rect.left() + 4.0, rect.center().y), 4.0, color.unwrap_or(theme::FG_40).gamma_multiply(alpha));
            let galley = ui.painter().layout(text.clone(), theme::font(13.0), theme::FG_60.gamma_multiply(alpha), 104.0);
            ui.painter().with_clip_rect(Rect::from_min_size(pos2(rect.left() + 18.0, rect.top()), vec2(104.0, size))).galley(pos2(rect.left() + 18.0, rect.center().y - galley.size().y / 2.0), galley, theme::FG_60);
        }
        Icon::Initials(text) => {
            let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
            ui.painter().circle_filled(rect.center(), size / 2.0, theme::CONTROL);
            if text.is_empty() {
                paint_glyph(ui, "user", Rect::from_center_size(rect.center(), vec2(size / 2.0, size / 2.0)), theme::FG_60);
            } else {
                let font = theme::weighted(if size >= 48.0 { size / 3.0 } else { size / 2.4 }, theme::SEMIBOLD);
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, text, font, theme::FG_60);
            }
        }
        Icon::Swatch(color) => {
            let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
            ui.painter().rect_filled(rect, 5.0, *color);
            ui.painter().rect_stroke(rect, 5.0, egui::Stroke::new(1.0, theme::KEY_BORDER), egui::StrokeKind::Inside);
        }
        Icon::File(_) | Icon::Thumbnail(_) | Icon::Url(_) | Icon::Image(_) | Icon::Emoji(_) => {
            let uri = match icon {
                Icon::File(path) => format!("{APP_ICON}{}/{path}", (size * ui.ctx().pixels_per_point()).round() as u32),
                Icon::Thumbnail(path) => format!("{APP_ICON}{}t/{path}", (size * ui.ctx().pixels_per_point() * 2.0).round() as u32),
                Icon::Url(url) | Icon::Image(url) => url.clone(),
                Icon::Emoji(text) => format!("{EMOJI}{}/{text}", (size * ui.ctx().pixels_per_point()).round() as u32),
                _ => unreachable!(),
            };
            let image = egui::Image::new(uri).fit_to_exact_size(vec2(size, size)).corner_radius(4.0);
            let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
            match image.load_for_size(ui.ctx(), vec2(size, size)) {
                Ok(poll) if poll.size().is_some() || matches!(poll, egui::load::TexturePoll::Ready { .. }) => match poll.size().filter(|_| matches!(icon, Icon::Thumbnail(_))) {
                    Some(source) => image.uv(cover_uv(source)).paint_at(ui, rect),
                    None => image.paint_at(ui, rect),
                },
                Ok(_) => {}
                Err(_) => letter(ui, rect, title),
            }
        }
    }
}

pub fn cover_uv(size: egui::Vec2) -> Rect {
    let side = size.min_elem();
    let visible = vec2(side / size.x.max(1.0), side / size.y.max(1.0));
    Rect::from_center_size(pos2(0.5, 0.5), visible)
}

fn letter(ui: &Ui, rect: Rect, title: &str) {
    ui.painter().rect_filled(rect, 5.0, theme::KEY_BG);
    let initial: String = title.trim().chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, initial, theme::weighted(11.0, theme::SEMIBOLD), theme::FG_60);
}

#[derive(Default)]
pub struct AppIconLoader {
    cache: Arc<Mutex<HashMap<String, Option<Result<Arc<ColorImage>, String>>>>>,
}

fn decode(png_bytes: &[u8]) -> Result<ColorImage, String> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer).map_err(|e| e.to_string())?;
    let pixels = &buffer[..info.buffer_size()];
    let size = [info.width as usize, info.height as usize];
    Ok(match info.color_type {
        png::ColorType::Rgba => ColorImage::from_rgba_unmultiplied(size, pixels),
        png::ColorType::Rgb => ColorImage::from_rgb(size, pixels),
        png::ColorType::GrayscaleAlpha => ColorImage::from_rgba_unmultiplied(size, &pixels.chunks_exact(2).flat_map(|p| [p[0], p[0], p[0], p[1]]).collect::<Vec<_>>()),
        png::ColorType::Grayscale => ColorImage::from_rgb(size, &pixels.iter().flat_map(|&g| [g, g, g]).collect::<Vec<_>>()),
        other => return Err(format!("unsupported png color type {other:?}")),
    })
}

pub fn fetch(url: &str) -> Result<Vec<u8>, String> {
    use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .tls_config(TlsConfig::builder().provider(TlsProvider::NativeTls).root_certs(RootCerts::PlatformVerifier).build())
        .timeout_global(Some(std::time::Duration::from_secs(10)))
        .build()
        .into();
    agent.get(url).call().map_err(|e| e.to_string())?.body_mut().with_config().limit(2 * 1024 * 1024).read_to_vec().map_err(|e| e.to_string())
}

impl ImageLoader for AppIconLoader {
    fn id(&self) -> &str {
        concat!(module_path!(), "::AppIconLoader")
    }

    fn load(&self, ctx: &egui::Context, uri: &str, _: SizeHint) -> ImageLoadResult {
        let remote = uri.starts_with("https://") || uri.starts_with(CLIP_IMAGE) || uri.starts_with(crate::flowkey_host::IMAGE_ORIGIN) || uri.starts_with(crate::flowkey_host::FILE_ORIGIN);
        if !remote && !uri.starts_with(APP_ICON) && !uri.starts_with(EMOJI) {
            return Err(LoadError::NotSupported);
        }
        let mut cache = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        match cache.get(uri) {
            Some(Some(Ok(image))) => return Ok(ImagePoll::Ready { image: image.clone() }),
            Some(Some(Err(e))) => return Err(LoadError::Loading(e.clone())),
            Some(None) => return Ok(ImagePoll::Pending { size: None }),
            None => {}
        }
        if let Some((size, text)) = uri.strip_prefix(EMOJI).and_then(|rest| rest.split_once('/')) {
            let result = crate::emoji_render::render(text, size.parse().unwrap_or(32)).map(Arc::new);
            cache.insert(uri.to_owned(), Some(result.clone()));
            return result.map(|image| ImagePoll::Ready { image }).map_err(LoadError::Loading);
        }
        cache.insert(uri.to_owned(), None);
        let (cache, uri, ctx) = (self.cache.clone(), uri.to_owned(), ctx.clone());
        std::thread::spawn(move || {
            let bytes = match uri.strip_prefix(APP_ICON).and_then(|rest| rest.split_once('/')) {
                Some((size, path)) => crate::apps::icon_png(path.to_owned(), size.trim_end_matches('t').parse::<i32>().unwrap_or(32).clamp(16, 256), size.ends_with('t')),
                None => match uri.strip_prefix(CLIP_IMAGE) {
                    Some(id) => crate::extensions::clipboard_history::HISTORY.get().and_then(|h| h.image_path(id)).ok_or_else(|| "image is gone".to_string()).and_then(|p| std::fs::read(p).map_err(|e| e.to_string())),
                    None => match uri.strip_prefix(crate::flowkey_host::FILE_ORIGIN).map(std::path::PathBuf::from).or_else(|| crate::flowkey_host::image_file(&uri)) {
                        Some(path) => std::fs::read(path).map_err(|e| e.to_string()),
                        None if uri.starts_with(crate::flowkey_host::IMAGE_ORIGIN) => Err("invalid image name".into()),
                        None => fetch(&uri),
                    },
                },
            };
            let result = bytes.and_then(|data| decode(&data).or_else(|_| crate::emoji_render::decode(&data))).map(Arc::new);
            cache.lock().unwrap_or_else(|e| e.into_inner()).insert(uri, Some(result));
            ctx.request_repaint();
        });
        Ok(ImagePoll::Pending { size: None })
    }

    fn forget(&self, uri: &str) {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).remove(uri);
    }

    fn forget_all(&self) {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    fn byte_size(&self) -> usize {
        self.cache.lock().unwrap_or_else(|e| e.into_inner()).values().flatten().flatten().map(|i| i.pixels.len() * 4).sum()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "network"]
    fn fetches_and_decodes_a_favicon() {
        let bytes = super::fetch("https://www.google.com/s2/favicons?domain=bing.com&sz=64").unwrap();
        let image = super::decode(&bytes).unwrap();
        assert!(image.size[0] >= 16);
    }
}
