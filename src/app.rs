//! Capa de UI.
//!
//! Acá no hay lógica de negocio: se traducen eventos a pulsaciones de la
//! [`Session`] y se dibuja el estado. Las métricas viven en `stats`, el estado
//! en `session`, la paleta en `theme`. Ese es el motivo por el que `stats` se
//! puede testear sin abrir una ventana.

use std::time::Instant;

use eframe::egui::{
    self, Align, Label, Layout, RichText,
    text::{LayoutJob, TextFormat},
};

use crate::{
    session::Session,
    theme,
    words::{self, Rng},
};

/// Tamaño fijo de la ventana.
///
/// Es una constante porque la ventana no se redimensiona: en niri (y en otros
/// tiling) una ventana con tamaño fijo se abre **flotante**, y así el texto
/// siempre entra en tres líneas sin que el gestor lo estire.
pub const WINDOW_SIZE: [f32; 2] = [720.0, 300.0];

/// Distancia al borde de la pantalla cuando se pide la esquina superior derecha.
const EDGE_MARGIN: f32 = 24.0;

/// Márgenes internos del panel (izquierda/derecha, arriba/abajo).
const PANEL_MARGIN_X: i8 = 28;
const PANEL_MARGIN_Y: i8 = 22;

/// Palabras por intento (el mockup muestra tres líneas de texto).
const WORD_COUNT: usize = 18;
/// Ancho máximo del bloque de texto: en pantallas anchas no se estira.
const PHRASE_MAX_WIDTH: f32 = 640.0;

/// Alto aproximado de todo lo que no es la frase: título (~18), espacios
/// (22 + 26), métricas (~18) y pista del atajo (10 + 16).
///
/// Sirve para repartir el aire arriba y abajo por igual, así que no necesita
/// precisión exacta; el test de layout es el que garantiza que la frase entre.
const CHROME_HEIGHT: f32 = 110.0;

/// Alto útil dentro de los márgenes del panel.
const fn content_height() -> f32 {
    WINDOW_SIZE[1] - 2.0 * PANEL_MARGIN_Y as f32
}

/// Una acción de teclado, en orden.
enum Action {
    Type(char),
    Backspace,
}

pub struct TypingApp {
    rng: Rng,
    session: Session,
    font: egui::FontId,
    /// La posición se pide una sola vez, cuando ya se conoce el monitor.
    placement_requested: bool,
}

impl TypingApp {
    pub fn new(ctx: &egui::Context) -> Self {
        theme::apply(ctx);

        let mut rng = Rng::from_entropy();
        let session = Session::new(words::phrase(&mut rng, WORD_COUNT));

        Self {
            rng,
            session,
            font: theme::phrase_font(),
            placement_requested: false,
        }
    }

    fn restart(&mut self) {
        self.session = Session::new(words::phrase(&mut self.rng, WORD_COUNT));
    }

    /// Ctrl+Enter (Cmd+Enter en macOS) reinicia: sirve a mitad de la prueba y
    /// también al terminar.
    ///
    /// `consume_key` **se lleva** el evento. Si en cambio se leyera
    /// `i.events` a mano, el Enter quedaría disponible para cualquier otro
    /// widget de la ventana.
    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let restart =
            ctx.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Enter));

        if restart {
            self.restart();
        }
    }

    /// Pide quedar pegado a la esquina superior derecha de la pantalla.
    ///
    /// Wayland no deja que un cliente se posicione solo: niri ignora este
    /// pedido y la posición real se define con una regla del compositor (ver
    /// `doc/niri.md`). En X11 sí se respeta. Se pide una sola vez porque
    /// `monitor_size` llega recién cuando la ventana ya existe.
    fn request_top_right(&mut self, ctx: &egui::Context) {
        let Some(monitor) = ctx.input(|input| input.viewport().monitor_size) else {
            return; // todavía no sabemos el tamaño del monitor: reintenta
        };

        let position = egui::pos2(monitor.x - WINDOW_SIZE[0] - EDGE_MARGIN, EDGE_MARGIN);
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(position));
        self.placement_requested = true;
    }

    /// Traduce los eventos de egui a pulsaciones, en el orden en que llegaron.
    ///
    /// Se descarta todo evento de texto que no sea **exactamente un carácter**:
    /// por ahí entran el pegado (Ctrl+V) y los commits de IME, que en un test de
    /// tipeo son trampa. `Event::Text` no es un "keydown": es texto ya
    /// compuesto, así que filtrar acá es más robusto que mirar el modificador.
    fn read_input(&mut self, ui: &egui::Ui) {
        if self.session.is_finished() {
            return;
        }

        let mut actions = Vec::new();

        ui.ctx().input(|input| {
            for event in &input.events {
                match event {
                    egui::Event::Text(text) => {
                        let mut chars = text.chars();
                        if let (Some(c), None) = (chars.next(), chars.next()) {
                            // Los caracteres de control (Enter, Tab, Escape)
                            // también llegan como `Text` en algunos backends:
                            // no son tipeo y no deben ensuciar la frase.
                            if !c.is_control() {
                                actions.push(Action::Type(c));
                            }
                        }
                    }
                    egui::Event::Key {
                        key: egui::Key::Backspace,
                        pressed: true,
                        ..
                    } => actions.push(Action::Backspace),
                    _ => {}
                }
            }
        });

        // Un solo `now` para el frame: si se procesara evento por evento con su
        // propio reloj, dos teclas del mismo frame quedarían con tiempos
        // distintos sin motivo.
        let now = Instant::now();
        for action in actions {
            match action {
                Action::Type(c) => self.session.push(c, now),
                Action::Backspace => self.session.backspace(),
            }
        }
    }

    /// Arma el `LayoutJob` de la frase tal como se va a dibujar.
    ///
    /// Con un `Label` por carácter, `horizontal_wrapped` corta las palabras al
    /// medio y el `item_spacing` separa las letras. Un `LayoutJob` con un
    /// `TextFormat` por carácter da el mismo color por letra pero con kerning y
    /// corte por palabra correctos.
    fn phrase_layout(&self, available_width: f32) -> LayoutJob {
        phrase_job(
            self.session.target(),
            self.session.typed(),
            &self.font,
            available_width.min(PHRASE_MAX_WIDTH),
            self.session.is_finished(),
        )
    }

    fn draw_footer(&self, ui: &mut egui::Ui) {
        match self.session.stats() {
            Some(stats) => {
                ui.label(
                    RichText::new(format!(
                        "Listo — {:.0} WPM · {:.1}% precisión · {:.0} ms ritmo",
                        stats.wpm, stats.accuracy, stats.rhythm_ms
                    ))
                    .size(theme::STATS_SIZE)
                    .italics()
                    .color(theme::TEXT),
                );
            }
            None => {
                ui.label(
                    RichText::new(format!(
                        "{}/{} caracteres · {}/{} pulsaciones acertadas",
                        self.session.typed().len(),
                        self.session.target().len(),
                        self.session.correct_keystrokes(),
                        self.session.keystrokes(),
                    ))
                    .size(theme::HINT_SIZE)
                    .italics()
                    .color(theme::MUTED),
                );
            }
        }

        ui.add_space(10.0);
        ui.label(
            RichText::new("Presioná Ctrl+Enter para reiniciar la prueba")
                .size(theme::HINT_SIZE)
                .italics()
                .color(theme::MUTED),
        );
    }
}

impl eframe::App for TypingApp {
    // En eframe 0.37 el punto de entrada es `ui` (recibe un `Ui` ya creado), no
    // `update(ctx, frame)`. `CentralPanel::show` también pide un `&mut Ui`.
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // `Context` es un `Arc` por dentro: clonarlo es barato y evita pelearse
        // con el préstamo de `ui` al mutar la sesión.
        let ctx = ui.ctx().clone();

        self.handle_shortcuts(&ctx);
        if !self.placement_requested {
            self.request_top_right(&ctx);
        }
        self.read_input(ui);

        let panel = egui::Frame::new()
            .inner_margin(egui::Margin::symmetric(PANEL_MARGIN_X, PANEL_MARGIN_Y))
            .fill(theme::BG);

        egui::CentralPanel::default().frame(panel).show(ui, |ui| {
            // `top_down(Align::Center)` centra cada hijo horizontalmente, que es
            // como se ve el mockup.
            ui.with_layout(Layout::top_down(Align::Center), |ui| {
                // Se mide antes de dibujar: la frase puede ocupar 2, 3 o 4
                // líneas, así que el aire sobrante se reparte arriba y abajo y
                // queda centrada en los tres casos.
                let job = self.phrase_layout(ui.available_width());
                let galley = ui.ctx().fonts_mut(|fonts| fonts.layout_job(job));
                // Misma fuente de verdad que el panel: la ventana es de tamaño fijo.
                let leftover = content_height() - CHROME_HEIGHT - galley.size().y;
                ui.add_space((leftover / 2.0).max(0.0));

                ui.label(
                    RichText::new("minish")
                        .size(theme::TITLE_SIZE)
                        .italics()
                        .color(theme::MUTED),
                );

                ui.add_space(22.0);
                ui.add(Label::new(egui::WidgetText::Galley(galley)));

                ui.add_space(26.0);
                self.draw_footer(ui);
            });
        });
    }
}

/// Describe la frase como un `LayoutJob`: **qué** se pinta, sin pintarlo.
///
/// Está fuera del `impl` a propósito: el test de layout mide exactamente el
/// mismo job que la app dibuja, sin abrir una ventana.
fn phrase_job(
    target: &[char],
    typed: &[char],
    font: &egui::FontId,
    max_width: f32,
    finished: bool,
) -> LayoutJob {
    let mut job = LayoutJob::default();

    for (index, &expected) in target.iter().enumerate() {
        let mut format = TextFormat {
            font_id: font.clone(),
            color: theme::PENDING,
            ..Default::default()
        };

        match typed.get(index) {
            Some(&actual) if actual == expected => format.color = theme::CORRECT,
            Some(_) => {
                format.color = theme::INCORRECT;
                format.background = theme::INCORRECT_BG;
                format.expand_bg = 1.5;
            }
            None => {
                // Marca dónde seguís, sin agregar un cursor aparte.
                if index == typed.len() && !finished {
                    format.underline = theme::caret_stroke();
                }
            }
        }

        let mut buffer = [0u8; 4];
        job.append(expected.encode_utf8(&mut buffer), 0.0, format);
    }

    job.wrap.max_width = max_width;
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    /// L1: con la ventana de tamaño fijo, ninguna frase generada puede desbordar
    /// el alto reservado. Se mide con el mismo motor de texto que usa la app
    /// (`ctx.run_pass` corre un frame sin ventana ni GPU).
    #[test]
    fn ninguna_frase_desborda_la_ventana() {
        let ctx = egui::Context::default();
        let font = theme::phrase_font();

        let mut peor_alto: f32 = 0.0;
        let mut peor_lineas = 0usize;

        let mut output = ctx.run_pass(Default::default(), |ctx| {
            let mut rng = Rng::from_seed(2024);

            for _ in 0..200 {
                let target = words::phrase(&mut rng, WORD_COUNT);
                let job = phrase_job(&target, &[], &font, PHRASE_MAX_WIDTH, false);
                let galley = ctx.fonts_mut(|fonts| fonts.layout_job(job));

                peor_alto = peor_alto.max(galley.size().y);
                peor_lineas = peor_lineas.max(galley.rows.len());
            }
        });

        // Sin backend gráfico nadie aplica el delta de texturas: egui exige
        // limpiarlo explícitamente antes de soltar la salida del frame.
        output.textures_delta.clear();

        assert!(
            CHROME_HEIGHT + peor_alto <= content_height(),
            "la frase más alta mide {peor_alto} px ({peor_lineas} líneas): con {CHROME_HEIGHT} px de chrome no entra en {} px",
            content_height()
        );
    }

    /// L2: el ancho del texto entra en la ventana con sus márgenes.
    #[test]
    fn ninguna_frase_desborda_el_ancho() {
        let ctx = egui::Context::default();
        let font = theme::phrase_font();

        let mut peor_ancho: f32 = 0.0;

        let mut output = ctx.run_pass(Default::default(), |ctx| {
            let mut rng = Rng::from_seed(99);

            for _ in 0..200 {
                let target = words::phrase(&mut rng, WORD_COUNT);
                let job = phrase_job(&target, &[], &font, PHRASE_MAX_WIDTH, false);
                let galley = ctx.fonts_mut(|fonts| fonts.layout_job(job));

                peor_ancho = peor_ancho.max(galley.size().x);
            }
        });

        output.textures_delta.clear();

        assert!(
            peor_ancho <= PHRASE_MAX_WIDTH,
            "la frase más ancha mide {peor_ancho} px"
        );
    }

    fn tecla(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        }
    }

    /// L3: Ctrl+Enter reinicia la prueba.
    #[test]
    fn ctrl_enter_reinicia_la_prueba() {
        let ctx = egui::Context::default();
        let mut app = TypingApp::new(&ctx);
        let anterior = app.session.target().to_vec();

        let mut input = egui::RawInput::default();
        input
            .events
            .push(tecla(egui::Key::Enter, egui::Modifiers::COMMAND));

        let mut output = ctx.run_pass(input, |ctx| app.handle_shortcuts(ctx));
        output.textures_delta.clear();

        assert_ne!(
            app.session.target(),
            anterior.as_slice(),
            "Ctrl+Enter tendría que haber generado una frase nueva"
        );
    }

    /// L4: sin Ctrl, Enter no escribe nada en la frase.
    #[test]
    fn enter_no_se_escribe_en_la_frase() {
        let ctx = egui::Context::default();
        let mut app = TypingApp::new(&ctx);

        let mut input = egui::RawInput::default();
        // Algunos backends mandan Enter como texto: no debe ensuciar la frase.
        input.events.push(egui::Event::Text("\n".to_owned()));
        input
            .events
            .push(tecla(egui::Key::Enter, egui::Modifiers::NONE));

        let mut output = ctx.run_pass(input, |ctx| {
            ctx.root_ui(|ui| app.read_input(ui));
        });
        output.textures_delta.clear();

        assert!(
            app.session.typed().is_empty(),
            "se escribió {:?}",
            app.session.typed()
        );
    }
}
