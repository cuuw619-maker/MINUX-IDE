use eframe::egui::Color32;
use serde::Deserialize;
use std::sync::OnceLock;

const THEME_DATA: &str = include_str!("../resources/themes.json");

#[derive(Clone, Debug, Deserialize)]
pub struct ThemePalette {
    pub id: String,
    pub name: String,
    pub accent: String,
    pub accent_bg: String,
}
#[derive(Deserialize)]
struct ThemeRegistry {
    default: String,
    themes: Vec<ThemePalette>,
}
fn registry() -> &'static ThemeRegistry {
    static THEMES: OnceLock<ThemeRegistry> = OnceLock::new();
    THEMES.get_or_init(|| serde_json::from_str(THEME_DATA).expect("Bundled theme catalog must be valid JSON"))
}
pub fn all() -> &'static [ThemePalette] { &registry().themes }
pub fn default_id() -> &'static str { &registry().default }
pub fn palette(id: &str) -> &'static ThemePalette {
    registry().themes.iter().find(|theme| theme.id == id)
        .or_else(|| registry().themes.iter().find(|theme| theme.id == registry().default))
        .expect("At least one bundled theme must exist")
}
pub fn color(hex: &str) -> Color32 {
    let value = hex.trim().trim_start_matches('#');
    if value.len() != 6 { return Color32::from_rgb(110, 155, 255); }
    match u32::from_str_radix(value, 16) {
        Ok(rgb) => Color32::from_rgb(((rgb >> 16) & 0xff) as u8, ((rgb >> 8) & 0xff) as u8, (rgb & 0xff) as u8),
        Err(_) => Color32::from_rgb(110, 155, 255),
    }
}
pub fn accent(id: &str) -> Color32 { color(&palette(id).accent) }
pub fn accent_bg(id: &str) -> Color32 { color(&palette(id).accent_bg) }
pub fn valid_id(id: &str) -> bool { registry().themes.iter().any(|theme| theme.id == id) }
