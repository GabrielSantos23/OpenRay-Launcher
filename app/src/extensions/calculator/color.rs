use super::math::to_precision;
use regex::Regex;
use std::sync::LazyLock;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Rgba {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

#[derive(PartialEq, Debug)]
pub struct ParsedColor {
    pub rgba: Rgba,
    pub format: &'static str,
}

static HEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$").unwrap());
static FUNCTION: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(rgba?|hsla?|lab|lch|oklab|oklch)\(\s*([^)]*)\)$").unwrap());
static CHANNEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^-?[\d.]+(%|deg)?$").unwrap());
static WHITESPACE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

fn js_number(text: &str) -> f64 {
    let text = text.trim();
    if text.is_empty() {
        return 0.0;
    }
    text.parse().unwrap_or(f64::NAN)
}

fn scaled(value: &str, scale: f64) -> f64 {
    match value.strip_suffix('%') {
        Some(percent) => js_number(percent) / 100.0 * scale,
        None => js_number(value),
    }
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> (f64, f64, f64) {
    let k = |n: f64| (n + h / 30.0) % 12.0;
    let a = s * l.min(1.0 - l);
    let f = |n: f64| l - a * (-1.0f64).max((k(n) - 3.0).min(9.0 - k(n)).min(1.0));
    (f(0.0) * 255.0, f(8.0) * 255.0, f(4.0) * 255.0)
}

fn valid(c: &Rgba) -> bool {
    [c.r, c.g, c.b].iter().all(|v| v.is_finite() && (0.0..=255.0).contains(v)) && c.a.is_finite() && (0.0..=1.0).contains(&c.a)
}

pub fn parse(input: &str) -> Option<ParsedColor> {
    let text = input.trim().to_lowercase();
    if let Some(caps) = HEX.captures(&text) {
        let digits: String = if caps[1].len() <= 4 { caps[1].chars().flat_map(|c| [c, c]).collect() } else { caps[1].to_owned() };
        let values: Vec<f64> = (0..digits.len()).step_by(2).map(|i| u8::from_str_radix(&digits[i..i + 2], 16).unwrap_or(0) as f64).collect();
        return Some(ParsedColor { rgba: Rgba { r: values[0], g: values[1], b: values[2], a: values.get(3).map_or(1.0, |a| a / 255.0) }, format: "HEX" });
    }
    let caps = FUNCTION.captures(&text)?;
    let kind = &caps[1];
    let mut split = caps[2].split('/').map(str::trim);
    let body = split.next().unwrap_or("");
    let alpha_part = split.next();
    let parts: Vec<&str> = if body.contains(',') { body.split(',').map(str::trim).collect() } else { WHITESPACE.split(body).collect() };
    let alpha_text = alpha_part.or(parts.get(3).copied());
    let polar = |l: f64, c: f64, h: f64| (l, c * (h * std::f64::consts::PI / 180.0).cos(), c * (h * std::f64::consts::PI / 180.0).sin());
    if kind == "lab" || kind == "lch" {
        if parts.len() < 3 {
            return None;
        }
        let l = scaled(parts[0], 100.0);
        let second = scaled(parts[1], if kind == "lab" { 125.0 } else { 150.0 });
        let third = scaled(parts[2], if kind == "lab" { 125.0 } else { 360.0 });
        let lab = if kind == "lab" { (l, second, third) } else { polar(l, second, third) };
        let rgba = rgb_from_lab(lab);
        return valid(&rgba).then_some(ParsedColor { rgba, format: if kind == "lab" { "LAB" } else { "LCH" } });
    }
    if kind == "oklab" || kind == "oklch" {
        if parts.len() < 3 {
            return None;
        }
        let l = scaled(parts[0], 1.0);
        let second = scaled(parts[1], 0.4);
        let third = scaled(parts[2], if kind == "oklab" { 0.4 } else { 360.0 });
        let lab = if kind == "oklab" { (l, second, third) } else { polar(l, second, third) };
        let rgba = rgb_from_oklab(lab);
        return valid(&rgba).then_some(ParsedColor { rgba, format: if kind == "oklab" { "OKLAB" } else { "OKLCH" } });
    }
    if parts.len() < 3 || parts[..3].iter().any(|p| !CHANNEL.is_match(p)) {
        return None;
    }
    let a = alpha_text.map_or(1.0, |t| scaled(t, 1.0));
    if kind.starts_with("rgb") {
        let rgba = Rgba { r: scaled(parts[0], 255.0), g: scaled(parts[1], 255.0), b: scaled(parts[2], 255.0), a };
        return valid(&rgba).then_some(ParsedColor { rgba, format: "RGB" });
    }
    let h = js_number(&parts[0].replace("deg", ""));
    let s = js_number(&parts[1].replace('%', "")) / 100.0;
    let l = js_number(&parts[2].replace('%', "")) / 100.0;
    let (r, g, b) = hsl_to_rgb(((h % 360.0) + 360.0) % 360.0, s, l);
    let rgba = Rgba { r, g, b, a };
    (valid(&rgba) && s <= 1.0 && l <= 1.0).then_some(ParsedColor { rgba, format: "HSL" })
}

fn fmt(n: f64) -> String {
    format!("{}", to_precision(if n.abs() < 1e-9 { 0.0 } else { n }, 3))
}

fn hex2(n: f64) -> String {
    format!("{:02X}", (n + 0.5).floor() as i64)
}

fn rgb_to_hsl(c: &Rgba) -> (f64, f64, f64) {
    let (r, g, b) = (c.r / 255.0, c.g / 255.0, c.b / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    let d = max - min;
    if d == 0.0 {
        return (0.0, 0.0, l * 100.0);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r { ((g - b) / d) % 6.0 } else if max == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 };
    ((h * 60.0 + 360.0) % 360.0, s * 100.0, l * 100.0)
}

fn srgb_to_linear(channel: f64) -> f64 {
    let v = channel / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(v: f64) -> f64 {
    let c = v.clamp(0.0, 1.0);
    (255.0 * if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 } + 0.5).floor()
}

type Triple = (f64, f64, f64);

fn polar_of(l: f64, a: f64, b: f64) -> Triple {
    (l, a.hypot(b), (b.atan2(a).to_degrees() + 360.0) % 360.0)
}

fn lab_from_rgb(c: &Rgba) -> (Triple, Triple) {
    let (lr, lg, lb) = (srgb_to_linear(c.r), srgb_to_linear(c.g), srgb_to_linear(c.b));
    let x = 0.4123907993 * lr + 0.3575843394 * lg + 0.1804807884 * lb;
    let y = 0.2126390059 * lr + 0.7151686788 * lg + 0.0721923154 * lb;
    let z = 0.0193308187 * lr + 0.1191947798 * lg + 0.9505321522 * lb;
    let f = |t: f64| if t > 216.0 / 24389.0 { t.cbrt() } else { (24389.0 / 27.0 * t + 16.0) / 116.0 };
    let (fx, fy, fz) = (f(x / 0.95047), f(y), f(z / 1.08883));
    let (l, a, b) = (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz));
    ((l, a, b), polar_of(l, a, b))
}

fn oklab_from_rgb(c: &Rgba) -> (Triple, Triple) {
    let (lr, lg, lb) = (srgb_to_linear(c.r), srgb_to_linear(c.g), srgb_to_linear(c.b));
    let l = (0.4122214708 * lr + 0.5363325363 * lg + 0.0514459929 * lb).cbrt();
    let m = (0.2119034982 * lr + 0.6806995451 * lg + 0.1073969566 * lb).cbrt();
    let s = (0.0883024619 * lr + 0.2817188376 * lg + 0.6299787005 * lb).cbrt();
    let big_l = 0.2104542553 * l + 0.793617785 * m - 0.0040720468 * s;
    let a = 1.9779984951 * l - 2.428592205 * m + 0.4505937099 * s;
    let b = 0.0259040371 * l + 0.7827717662 * m - 0.808675766 * s;
    ((big_l, a, b), polar_of(big_l, a, b))
}

fn rgb_from_lab((l, a, b): Triple) -> Rgba {
    let fy = (l + 16.0) / 116.0;
    let (fx, fz) = (fy + a / 500.0, fy - b / 200.0);
    let finv = |t: f64| if t > 6.0 / 29.0 { t.powi(3) } else { 3.0 * (6.0f64 / 29.0).powi(2) * (t - 4.0 / 29.0) };
    let (x, y, z) = (0.95047 * finv(fx), finv(fy), 1.08883 * finv(fz));
    let lr = 3.2404542 * x - 1.5371385 * y - 0.4985314 * z;
    let lg = -0.969266 * x + 1.8760108 * y + 0.041556 * z;
    let lb = 0.0556434 * x - 0.2040259 * y + 1.0572252 * z;
    Rgba { r: linear_to_srgb(lr), g: linear_to_srgb(lg), b: linear_to_srgb(lb), a: 1.0 }
}

fn rgb_from_oklab((l, a, b): Triple) -> Rgba {
    let l_ = (l + 0.3963377774 * a + 0.2158037573 * b).powi(3);
    let m = (l - 0.1055613458 * a - 0.0638541728 * b).powi(3);
    let s = (l - 0.0894841775 * a - 1.291485548 * b).powi(3);
    let lr = 4.0767416621 * l_ - 3.3077115913 * m + 0.2309699292 * s;
    let lg = -1.2684380046 * l_ + 2.6097574011 * m - 0.3413193965 * s;
    let lb = -0.0041960863 * l_ - 0.7034186147 * m + 1.707614701 * s;
    Rgba { r: linear_to_srgb(lr), g: linear_to_srgb(lg), b: linear_to_srgb(lb), a: 1.0 }
}

pub struct Format {
    pub id: &'static str,
    pub label: &'static str,
    pub value: String,
}

pub fn formats(c: &Rgba) -> Vec<Format> {
    let Rgba { r, g, b, a } = *c;
    let (h, s, l) = rgb_to_hsl(c);
    let (lab, lch) = lab_from_rgb(c);
    let (oklab, oklch) = oklab_from_rgb(c);
    let white = r.min(g).min(b) / 2.55;
    let black = 100.0 - r.max(g).max(b) / 2.55;
    let alpha = if a < 1.0 { format!(" / {}%", fmt(a * 100.0)) } else { String::new() };
    let hex = format!("#{}{}{}", hex2(r), hex2(g), hex2(b));
    let f = |id, label, value: String| Format { id, label, value };
    vec![
        f("hex", "Hex", hex.clone()),
        f("hexAlpha", "Hex with Alpha", format!("{hex}{}", hex2(a * 255.0))),
        f("rgba", "RGBA", format!("rgba({}, {}, {}, {})", fmt(r), fmt(g), fmt(b), fmt(a))),
        f("rgbaPercentage", "RGBA (Percentage)", format!("rgba({}%, {}%, {}%, {})", fmt(r / 2.55), fmt(g / 2.55), fmt(b / 2.55), fmt(a))),
        f("rgbCss4", "RGB (CSS4)", format!("rgb({}, {}, {}{alpha})", fmt(r), fmt(g), fmt(b))),
        f("hsl", "HSL", format!("hsl({}deg, {}%, {}%)", fmt(h), fmt(s), fmt(l))),
        f("hsla", "HSL with Alpha", format!("hsl({}deg, {}%, {}%, {})", fmt(h), fmt(s), fmt(l), fmt(a))),
        f("hslCss4", "HSL (CSS4)", format!("hsl({}deg {}% {}%{alpha})", fmt(h), fmt(s), fmt(l))),
        f("hwb", "HWB", format!("hwb({}deg {}% {}%{alpha})", fmt(h), fmt(white), fmt(black))),
        f("lab", "LAB", format!("lab({}% {} {})", fmt(lab.0), fmt(lab.1), fmt(lab.2))),
        f("lch", "LCH", format!("lch({}% {} {})", fmt(lch.0), fmt(lch.1), fmt(lch.2))),
        f("oklab", "OKLAB", format!("oklab({} {} {})", fmt(oklab.0), fmt(oklab.1), fmt(oklab.2))),
        f("oklch", "OKLCH", format!("oklch({} {} {})", fmt(oklch.0), fmt(oklch.1), fmt(oklch.2))),
    ]
}

const NAMED: &str = "Alice Blue:f0f8ff,Antique White:faebd7,Aqua:00ffff,Aquamarine:7fffd4,Azure:f0ffff,Beige:f5f5dc,Bisque:ffe4c4,Black:000000,Blanched Almond:ffebcd,Blue:0000ff,Blue Violet:8a2be2,Brown:a52a2a,Burlywood:deb887,Cadet Blue:5f9ea0,Chartreuse:7fff00,Chocolate:d2691e,Coral:ff7f50,Cornflower Blue:6495ed,Cornsilk:fff8dc,Crimson:dc143c,Dark Blue:00008b,Dark Cyan:008b8b,Dark Goldenrod:b8860b,Dark Gray:a9a9a9,Dark Green:006400,Dark Khaki:bdb76b,Dark Magenta:8b008b,Dark Olive Green:556b2f,Dark Orange:ff8c00,Dark Orchid:9932cc,Dark Red:8b0000,Dark Salmon:e9967a,Dark Sea Green:8fbc8f,Dark Slate Blue:483d8b,Dark Slate Gray:2f4f4f,Dark Turquoise:00ced1,Dark Violet:9400d3,Deep Pink:ff1493,Deep Sky Blue:00bfff,Dim Gray:696969,Dodger Blue:1e90ff,Firebrick:b22222,Floral White:fffaf0,Forest Green:228b22,Gainsboro:dcdcdc,Ghost White:f8f8ff,Gold:ffd700,Goldenrod:daa520,Gray:808080,Green:008000,Green Yellow:adff2f,Honeydew:f0fff0,Hot Pink:ff69b4,Indian Red:cd5c5c,Indigo:4b0082,Ivory:fffff0,Khaki:f0e68c,Lavender:e6e6fa,Lavender Blush:fff0f5,Lawn Green:7cfc00,Lemon Chiffon:fffacd,Light Blue:add8e6,Light Coral:f08080,Light Cyan:e0ffff,Light Goldenrod Yellow:fafad2,Light Gray:d3d3d3,Light Green:90ee90,Light Pink:ffb6c1,Light Salmon:ffa07a,Light Sea Green:20b2aa,Light Sky Blue:87cefa,Light Slate Gray:778899,Light Steel Blue:b0c4de,Light Yellow:ffffe0,Lime:00ff00,Lime Green:32cd32,Linen:faf0e6,Maroon:800000,Medium Aquamarine:66cdaa,Medium Blue:0000cd,Medium Orchid:ba55d3,Medium Purple:9370db,Medium Sea Green:3cb371,Medium Slate Blue:7b68ee,Medium Spring Green:00fa9a,Medium Turquoise:48d1cc,Medium Violet Red:c71585,Midnight Blue:191970,Mint Cream:f5fffa,Misty Rose:ffe4e1,Moccasin:ffe4b5,Navajo White:ffdead,Navy:000080,Old Lace:fdf5e6,Olive:808000,Olive Drab:6b8e23,Orange:ffa500,Orange Red:ff4500,Orchid:da70d6,Pale Goldenrod:eee8aa,Pale Green:98fb98,Pale Turquoise:afeeee,Pale Violet Red:db7093,Papaya Whip:ffefd5,Peach Puff:ffdab9,Peru:cd853f,Pink:ffc0cb,Plum:dda0dd,Powder Blue:b0e0e6,Purple:800080,Rebecca Purple:663399,Red:ff0000,Rosy Brown:bc8f8f,Royal Blue:4169e1,Saddle Brown:8b4513,Salmon:fa8072,Sandy Brown:f4a460,Sea Green:2e8b57,Sea Shell:fff5ee,Sienna:a0522d,Silver:c0c0c0,Sky Blue:87ceeb,Slate Blue:6a5acd,Slate Gray:708090,Snow:fffafa,Spring Green:00ff7f,Steel Blue:4682b4,Tan:d2b48c,Teal:008080,Thistle:d8bfd8,Tomato:ff6347,Turquoise:40e0d0,Violet:ee82ee,Wheat:f5deb3,White:ffffff,White Smoke:f5f5f5,Yellow:ffff00,Yellow Green:9acd32";

pub fn name(c: &Rgba) -> &'static str {
    NAMED
        .split(',')
        .filter_map(|entry| entry.split_once(':'))
        .map(|(name, hex)| {
            let channel = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).unwrap_or(0) as f64;
            (name, (channel(0) - c.r).powi(2) + (channel(2) - c.g).powi(2) + (channel(4) - c.b).powi(2))
        })
        .fold(("Alice Blue", f64::INFINITY), |best, next| if next.1 < best.1 { next } else { best })
        .0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_rgb_and_hsl() {
        assert_eq!(parse("#fff"), Some(ParsedColor { rgba: Rgba { r: 255.0, g: 255.0, b: 255.0, a: 1.0 }, format: "HEX" }));
        assert_eq!(parse("rgb(255 0 0 / 50%)"), Some(ParsedColor { rgba: Rgba { r: 255.0, g: 0.0, b: 0.0, a: 0.5 }, format: "RGB" }));
        let green = parse("hsl(120, 100%, 50%)").unwrap().rgba;
        assert_eq!((green.r, green.g, green.b, green.a), (0.0, 255.0, 0.0, 1.0));
        assert_eq!(parse("#ggg"), None);
        assert_eq!(parse("rgb(300, 0, 0)"), None);
    }

    #[test]
    fn names_the_nearest_css_color_and_lists_copy_formats() {
        assert_eq!(name(&Rgba { r: 255.0, g: 255.0, b: 255.0, a: 1.0 }), "White");
        assert_eq!(name(&Rgba { r: 100.0, g: 149.0, b: 236.0, a: 1.0 }), "Cornflower Blue");
        let list = formats(&Rgba { r: 255.0, g: 0.0, b: 0.0, a: 1.0 });
        let get = |id| list.iter().find(|f| f.id == id).map(|f| f.value.as_str());
        assert_eq!(get("hex"), Some("#FF0000"));
        assert_eq!(get("hsl"), Some("hsl(0deg, 100%, 50%)"));
        assert_eq!(get("rgbCss4"), Some("rgb(255, 0, 0)"));
    }
}
