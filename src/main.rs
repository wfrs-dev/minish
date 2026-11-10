use eframe::egui;
use std::time::Instant;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 250.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Entrenador de Mecanografía",
        options,
        Box::new(|_cc| Box::new(TypingTrainerApp::new())),
    )
}

struct TypingTrainerApp {
    target_phrase: String,
    input_text: String,
    start_time: Option<Instant>,
    is_finished: bool,
    // Estadísticas finales
    final_wpm: f32,
    final_accuracy: f32,
}

impl TypingTrainerApp {
    fn new() -> Self {
        Self {
            target_phrase: "oreo dos irse sano sed inaudita ura rata".to_string(),
            input_text: String::new(),
            start_time: None,
            is_finished: false,
            final_wpm: 0.0,
            final_accuracy: 100.0,
        }
    }

    fn reset(&mut self) {
        self.input_text.clear();
        self.start_time = None;
        self.is_finished = false;
    }
}

impl eframe::App for TypingTrainerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 1. Capturar eventos de teclado globales en la ventana
        if !self.is_finished {
            ctx.input(|i| {
                for event in &i.events {
                    if let egui::Event::Text(text) = event {
                        // Iniciar cronómetro con la primera tecla
                        if self.start_time.is_none() {
                            self.start_time = Some(Instant::now());
                        }

                        for c in text.chars() {
                            // Evitar rebasar la longitud de la frase objetivo
                            if self.input_text.chars().count() < self.target_phrase.chars().count()
                            {
                                self.input_text.push(c);
                            }
                        }
                    }

                    // Manejar tecla Backspace para borrar
                    if let egui::Event::Key {
                        key: egui::Key::Backspace,
                        pressed: true,
                        ..
                    } = event
                    {
                        self.input_text.pop();
                    }
                }
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(20.0);

            // Estilo visual minimalista (centrado opcional)
            ui.vertical_centered(|ui| {
                ui.heading(
                    egui::RichText::new("Entrenador de Teclado")
                        .size(16.0)
                        .color(egui::Color32::GRAY),
                );
                ui.add_space(20.0);

                // 2. Renderizar el texto carácter por carácter con colores
                ui.horizontal_wrapped(|ui| {
                    let target_chars: Vec<char> = self.target_phrase.chars().collect();
                    let input_chars: Vec<char> = self.input_text.chars().collect();

                    for (i, &target_char) in target_chars.iter().enumerate() {
                        if i < input_chars.len() {
                            let typed_char = input_chars[i];
                            if typed_char == target_char {
                                // Acierto (Verde)
                                ui.label(
                                    egui::RichText::new(target_char.to_string())
                                        .size(22.0)
                                        .color(egui::Color32::from_rgb(46, 160, 67)),
                                );
                            } else {
                                // Fallo (Rojo con fondo sutil como en tu imagen)
                                ui.label(
                                    egui::RichText::new(target_char.to_string())
                                        .size(22.0)
                                        .color(egui::Color32::from_rgb(218, 54, 51))
                                        .background_color(egui::Color32::from_rgba_unmultiplied(
                                            218, 54, 51, 30,
                                        )),
                                );
                            }
                        } else {
                            // Letras pendientes (Gris claro)
                            ui.label(
                                egui::RichText::new(target_char.to_string())
                                    .size(22.0)
                                    .color(egui::Color32::from_rgb(120, 120, 120)),
                            );
                        }
                    }
                });

                ui.add_space(30.0);

                // 3. Evaluar si se completó la frase
                if self.input_text.chars().count() == self.target_phrase.chars().count()
                    && !self.is_finished
                {
                    self.is_finished = true;

                    if let Some(start) = self.start_time {
                        let elapsed_mins = start.elapsed().as_secs_f32() / 60.0;
                        let words = self.target_phrase.split_whitespace().count() as f32;
                        self.final_wpm = if elapsed_mins > 0.0 {
                            words / elapsed_mins
                        } else {
                            0.0
                        };

                        // Calcular precisión
                        let mut errors = 0;
                        for (i, c) in input_chars_iter(&self.input_text) {
                            if let Some(t) = self.target_phrase.chars().nth(i) {
                                if c != t {
                                    errors += 1;
                                }
                            }
                        }
                        let total_chars = self.target_phrase.chars().count() as f32;
                        self.final_accuracy = ((total_chars - errors as f32) / total_chars) * 100.0;
                    }
                }

                // 4. Mostrar estadísticas al finalizar (o en tiempo real)
                if self.is_finished {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "Listo — {:.0} WPM · {:.1}% precisión",
                                self.final_wpm, self.final_accuracy
                            ))
                            .size(16.0)
                            .italics()
                            .color(egui::Color32::WHITE),
                        );

                        if ui.button("Reiniciar").clicked() {
                            self.reset();
                        }
                    });
                } else {
                    ui.label(
                        egui::RichText::new("Escribe la frase para calcular estadísticas...")
                            .size(12.0)
                            .italics()
                            .color(egui::Color32::DARK_GRAY),
                    );
                }
            });
        });

        // Forzar redibujado constante si el usuario está escribiendo para mantener fluidez
        ctx.request_repaint();
    }
}

// Helper para iterar caracteres fácilmente
fn input_chars_iter(s: &str) -> impl Iterator<Item = (usize, char)> + '_ {
    s.chars().enumerate()
}
