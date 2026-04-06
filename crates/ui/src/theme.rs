use gpui::Rgba;

/// Deep navy background
pub const BG_PRIMARY: Rgba = rgb_const(0x1a, 0x1a, 0x2e);
/// Slightly lighter secondary background
pub const BG_SECONDARY: Rgba = rgb_const(0x16, 0x21, 0x3e);
/// Light gray text
pub const TEXT_PRIMARY: Rgba = rgb_const(0xe0, 0xe0, 0xe0);
/// Medium gray secondary text
pub const TEXT_SECONDARY: Rgba = rgb_const(0x88, 0x88, 0x88);
/// Blue-purple accent
pub const ACCENT: Rgba = rgb_const(0x7c, 0x8c, 0xf7);
/// Green for positive P/L
pub const GREEN: Rgba = rgb_const(0x4a, 0xde, 0x80);
/// Red for negative P/L
pub const RED: Rgba = rgb_const(0xef, 0x44, 0x44);
/// Yellow for Tesouro
pub const YELLOW: Rgba = rgb_const(0xf5, 0x9e, 0x0b);
/// Purple for gold
pub const PURPLE: Rgba = rgb_const(0x8b, 0x5c, 0xf6);
/// Subtle border color
pub const BORDER: Rgba = rgb_const(0x2a, 0x2a, 0x4a);

/// Build an Rgba at const time from individual r, g, b bytes.
const fn rgb_const(r: u8, g: u8, b: u8) -> Rgba {
    Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}
