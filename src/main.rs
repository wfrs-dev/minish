//! minish — entrenador de mecanografía minimalista.
//!
//! Estructura:
//!
//! - [`stats`]: fórmulas de WPM, precisión y ritmo. Puro, con tests.
//! - [`session`]: qué se tipeó, cuántas pulsaciones, en qué estado está.
//! - [`words`]: vocabulario y generación de frases (sin dependencias).
//! - [`theme`]: paleta y tipografía.
//! - [`app`]: la capa de egui, que solo dibuja y traduce eventos.

mod app;
mod session;
mod stats;
mod theme;
mod words;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("minish — entrenador de mecanografía")
            // El app_id es lo que usan los compositores (y sus reglas) para
            // reconocer la ventana. Ver doc/niri.md.
            .with_app_id("minish")
            // Tamaño fijo: min == max == inner. En niri una ventana de tamaño
            // fijo se abre flotante en vez de entrar en el mosaico, y en el
            // resto de los gestores `resizable(false)` evita que la estiren.
            .with_inner_size(app::WINDOW_SIZE)
            .with_min_inner_size(app::WINDOW_SIZE)
            .with_max_inner_size(app::WINDOW_SIZE)
            .with_resizable(false)
            // "que se muestre sobre las demás ventanas"
            .with_window_level(eframe::egui::WindowLevel::AlwaysOnTop),
        ..Default::default()
    };

    eframe::run_native(
        "minish — entrenador de mecanografía",
        options,
        // El creador devuelve `Result<Box<dyn App>, _>` desde eframe 0.37.
        Box::new(|cc| Ok(Box::new(app::TypingApp::new(&cc.egui_ctx)))),
    )
}
