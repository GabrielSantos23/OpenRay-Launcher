use egui::ColorImage;
use windows::core::{w, Result};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT, D2D_RECT_F};
use windows::Win32::Graphics::Direct2D::{D2D1CreateFactory, ID2D1Factory, D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_RENDER_TARGET_PROPERTIES};
use windows::Win32::Graphics::DirectWrite::{DWriteCreateFactory, IDWriteFactory, DWRITE_FACTORY_TYPE_SHARED, DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_MEASURING_MODE_NATURAL, DWRITE_PARAGRAPH_ALIGNMENT_CENTER, DWRITE_TEXT_ALIGNMENT_CENTER};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Imaging::{CLSID_WICImagingFactory, GUID_WICPixelFormat32bppPBGRA, IWICImagingFactory, WICBitmapCacheOnLoad};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};

struct Factories {
    wic: IWICImagingFactory,
    d2d: ID2D1Factory,
    dwrite: IDWriteFactory,
}

thread_local! {
    static FACTORIES: std::cell::OnceCell<Option<Factories>> = const { std::cell::OnceCell::new() };
}

fn factories() -> Option<Factories> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        Some(Factories { wic: CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?, d2d: D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None).ok()?, dwrite: DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED).ok()? })
    }
}

fn draw(f: &Factories, text: &str, px: u32) -> Result<ColorImage> {
    unsafe {
        let bitmap = f.wic.CreateBitmap(px, px, &GUID_WICPixelFormat32bppPBGRA, WICBitmapCacheOnLoad)?;
        let props = D2D1_RENDER_TARGET_PROPERTIES { pixelFormat: D2D1_PIXEL_FORMAT { format: DXGI_FORMAT_B8G8R8A8_UNORM, alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED }, dpiX: 96.0, dpiY: 96.0, ..Default::default() };
        let target = f.d2d.CreateWicBitmapRenderTarget(&bitmap, &props)?;
        let format = f.dwrite.CreateTextFormat(w!("Segoe UI Emoji"), None, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_STRETCH_NORMAL, px as f32 * 0.8, w!("en-us"))?;
        format.SetTextAlignment(DWRITE_TEXT_ALIGNMENT_CENTER)?;
        format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
        let brush = target.CreateSolidColorBrush(&D2D1_COLOR_F { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, None)?;
        let wide: Vec<u16> = text.encode_utf16().collect();
        target.BeginDraw();
        target.Clear(Some(&D2D1_COLOR_F::default()));
        target.DrawText(&wide, &format, &D2D_RECT_F { left: 0.0, top: 0.0, right: px as f32, bottom: px as f32 }, &brush, D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT, DWRITE_MEASURING_MODE_NATURAL);
        target.EndDraw(None, None)?;
        let mut pixels = vec![0u8; (px * px * 4) as usize];
        bitmap.CopyPixels(std::ptr::null(), px * 4, &mut pixels)?;
        for p in pixels.chunks_exact_mut(4) {
            p.swap(0, 2);
        }
        Ok(ColorImage::from_rgba_premultiplied([px as usize, px as usize], &pixels))
    }
}

fn decode_with(f: &Factories, bytes: &[u8]) -> Result<ColorImage> {
    use windows::Win32::Graphics::Imaging::{GUID_WICPixelFormat32bppRGBA, WICBitmapDitherTypeNone, WICBitmapPaletteTypeCustom, WICDecodeMetadataCacheOnDemand};
    unsafe {
        let stream = f.wic.CreateStream()?;
        stream.InitializeFromMemory(bytes)?;
        let decoder = f.wic.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand)?;
        let converter = f.wic.CreateFormatConverter()?;
        converter.Initialize(&decoder.GetFrame(0)?, &GUID_WICPixelFormat32bppRGBA, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeCustom)?;
        let (mut w, mut h) = (0, 0);
        converter.GetSize(&mut w, &mut h)?;
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        converter.CopyPixels(std::ptr::null(), w * 4, &mut pixels)?;
        Ok(ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &pixels))
    }
}

pub fn decode(bytes: &[u8]) -> std::result::Result<ColorImage, String> {
    FACTORIES.with(|cell| match cell.get_or_init(factories) {
        Some(f) => decode_with(f, bytes).map_err(|e| e.message()),
        None => Err("Windows Imaging is unavailable".into()),
    })
}

pub fn render(text: &str, px: u32) -> std::result::Result<ColorImage, String> {
    FACTORIES.with(|cell| match cell.get_or_init(factories) {
        Some(f) => draw(f, text, px.clamp(8, 256)).map_err(|e| e.to_string()),
        None => Err("Direct2D is unavailable".into()),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_color_emoji() {
        let image = super::render("😀", 32).unwrap();
        assert_eq!(image.size, [32, 32]);
        assert!(image.pixels.iter().any(|p| p.r() > 150 && p.g() > 100 && p.b() < 100 && p.a() > 200), "expected yellow pixels");
    }
}
