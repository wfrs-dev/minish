//! Métricas de una sesión de mecanografía.
//!
//! Este módulo es **puro**: no conoce egui, ni `Instant`, ni ventanas. Recibe
//! números y devuelve números, así que se puede testear sin abrir una ventana.
//!
//! Fórmulas (las de cualquier test de tipeo serio, no inventadas acá):
//!
//! - WPM neto  = (caracteres correctos / 5) / minutos transcurridos
//! - precisión = pulsaciones correctas / pulsaciones totales * 100
//! - ritmo     = milisegundos por pulsación (el "pace" al que vas)

use std::time::Duration;

/// Cuántos caracteres cuenta una "palabra" en el estándar de WPM.
pub const CHARS_PER_WORD: f32 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Stats {
    /// Palabras por minuto, contando solo los caracteres correctos.
    pub wpm: f32,
    /// Porcentaje 0..=100 sobre **todas** las pulsaciones, no sobre el final.
    pub accuracy: f32,
    /// Milisegundos promedio por pulsación.
    pub rhythm_ms: f32,
}

impl Stats {
    /// Calcula las métricas de una sesión.
    ///
    /// - `target`: la frase objetivo (la que había que escribir).
    /// - `typed`: lo que se escribió hasta ahora; puede ser más corto.
    /// - `keystrokes`: pulsaciones aceptadas, incluidos los errores ya borrados.
    /// - `correct_keystrokes`: de esas, cuántas acertaron.
    /// - `elapsed`: tiempo desde la primera pulsación.
    pub fn compute(
        target: &[char],
        typed: &[char],
        keystrokes: u32,
        correct_keystrokes: u32,
        elapsed: Duration,
    ) -> Self {
        // Caracteres correctos: comparación posición a posición, cortando en el
        // más corto de los dos. Nada de `nth()` en un loop (eso es O(n²)).
        let correct_chars = target
            .iter()
            .zip(typed)
            .filter(|(expected, actual)| expected == actual)
            .count() as f32;

        let minutes = elapsed.as_secs_f32() / 60.0;
        let wpm = if minutes > 0.0 && minutes.is_finite() {
            correct_chars / CHARS_PER_WORD / minutes
        } else {
            0.0
        };

        let accuracy = if keystrokes == 0 {
            100.0
        } else {
            correct_keystrokes as f32 / keystrokes as f32 * 100.0
        };

        let rhythm_ms = if keystrokes == 0 {
            0.0
        } else {
            elapsed.as_secs_f32() * 1000.0 / keystrokes as f32
        };

        Self {
            wpm,
            accuracy,
            rhythm_ms,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    /// S1: 5 caracteres = 1 palabra, y el tiempo se mide en minutos.
    #[test]
    fn wpm_usa_cinco_caracteres_por_palabra() {
        // 50 caracteres correctos en 60 s = 10 palabras / 1 minuto = 10 WPM.
        let target = chars("abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwx");
        assert_eq!(target.len(), 50);

        let stats = Stats::compute(&target, &target, 50, 50, Duration::from_secs(60));

        assert!((stats.wpm - 10.0).abs() < 1e-4, "wpm = {}", stats.wpm);
    }

    /// S1: los caracteres mal tipeados no suman WPM.
    #[test]
    fn wpm_cuenta_solo_los_caracteres_correctos() {
        // 4 correctos de 5 = 0.8 palabras en 30 s (0.5 min) = 1.6 WPM.
        let target = chars("abcde");
        let typed = chars("abxde");

        let stats = Stats::compute(&target, &typed, 5, 4, Duration::from_secs(30));

        assert!((stats.wpm - 1.6).abs() < 1e-4, "wpm = {}", stats.wpm);
    }

    /// S2: la precisión cuenta pulsaciones, así que un error corregido penaliza.
    #[test]
    fn precision_se_mide_sobre_pulsaciones_no_sobre_el_resultado() {
        // Resultado perfecto, pero hubo 1 error en el camino: 9 de 10.
        let target = chars("abcde");

        let stats = Stats::compute(&target, &target, 10, 9, Duration::from_secs(10));

        assert!(
            (stats.accuracy - 90.0).abs() < 1e-4,
            "accuracy = {}",
            stats.accuracy
        );
    }

    /// S2: sin pulsaciones no hay errores posibles.
    #[test]
    fn precision_es_cien_por_ciento_sin_pulsaciones() {
        let stats = Stats::compute(&[], &[], 0, 0, Duration::from_secs(3));

        assert_eq!(stats.accuracy, 100.0);
    }

    /// S3: ritmo = milisegundos promedio por pulsación.
    #[test]
    fn ritmo_es_el_promedio_de_milisegundos_por_pulsacion() {
        // 1000 ms / 4 pulsaciones = 250 ms.
        let target = chars("a");

        let stats = Stats::compute(&target, &target, 4, 4, Duration::from_millis(1000));

        assert!(
            (stats.rhythm_ms - 250.0).abs() < 1e-4,
            "ritmo = {}",
            stats.rhythm_ms
        );
    }

    /// S4: un tiempo cero no puede producir NaN ni infinitos.
    #[test]
    fn tiempo_cero_no_produce_nan() {
        let target = chars("a");

        let stats = Stats::compute(&target, &target, 1, 1, Duration::ZERO);

        assert_eq!(stats.wpm, 0.0);
        assert!(stats.wpm.is_finite());
        assert!(stats.rhythm_ms.is_finite());
    }

    /// S1: una sesión a medias es válida (typed más corto que target).
    #[test]
    fn acepta_una_sesion_a_medias() {
        let target = chars("abcdefghij");
        let typed = chars("abc");

        // 3 correctos = 0.6 palabras en 1 minuto.
        let stats = Stats::compute(&target, &typed, 3, 3, Duration::from_secs(60));

        assert!((stats.wpm - 0.6).abs() < 1e-4, "wpm = {}", stats.wpm);
    }
}
