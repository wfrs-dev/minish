//! Paleta y tipografía en un solo lugar.
//!
//! La UI no menciona ningún color literal: pide nombres. Si mañana querés otro
//! tema, tocás este archivo y nada más.

use eframe::egui::{self, Color32, FontFamily, FontId, Stroke};

// --- Paleta (tomada del mockup: verde sobre crema) -------------------------

pub const BG: Color32 = Color32::from_rgb(250, 248, 243);
pub const TEXT: Color32 = Color32::from_rgb(38, 38, 38);
pub const MUTED: Color32 = Color32::from_rgb(138, 138, 138);

/// Carácter que todavía no tipeaste.
pub const PENDING: Color32 = Color32::from_rgb(170, 170, 170);
/// Carácter tipeado bien.
pub const CORRECT: Color32 = Color32::from_rgb(26, 122, 75);
/// Carácter tipeado mal.
pub const INCORRECT: Color32 = Color32::from_rgb(200, 60, 65);
/// Fondo rosado detrás del carácter mal tipeado.
pub const INCORRECT_BG: Color32 = Color32::from_rgb(250, 226, 227);
/// Marca del próximo carácter a tipear.
pub const CARET: Color32 = CORRECT;

// --- Tamaños --------------------------------------------------------------

pub const TITLE_SIZE: f32 = 13.0;
pub const PHRASE_SIZE: f32 = 28.0;
pub const STATS_SIZE: f32 = 15.0;
pub const HINT_SIZE: f32 = 12.0;

pub fn phrase_font() -> FontId {
    FontId::new(PHRASE_SIZE, FontFamily::Proportional)
}

pub fn caret_stroke() -> Stroke {
    Stroke::new(2.0, CARET)
}

/// Fuerza el tema claro con esta paleta, sin importar el tema del sistema.
///
/// Se aplica sobre los dos temas a propósito: si el SO está en oscuro y solo
/// configurás el claro, egui usa el oscuro por defecto y la app se ve negra.
pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = BG;
    visuals.window_fill = BG;
    visuals.override_text_color = Some(TEXT);

    ctx.set_theme(egui::Theme::Light);
    ctx.set_visuals_of(egui::Theme::Light, visuals.clone());
    ctx.set_visuals_of(egui::Theme::Dark, visuals);
}
