//! Estado de una sesión de mecanografía.
//!
//! Dos tipos con responsabilidades distintas:
//!
//! - [`Tracker`]: acumula lo tipeado y las pulsaciones. No sabe si la sesión
//!   terminó, solo si llegó al final de la frase.
//! - [`Session`]: máquina de estados explícita. **O estás tipeando o
//!   terminaste**, y cuando terminaste las métricas ya están congeladas. No
//!   existen estados intermedios raros porque no se pueden construir.
//!
//! El tiempo entra por parámetro (`now`) en vez de leer el reloj adentro: por
//! eso los tests pueden simular una sesión de 10 segundos sin esperar 10
//! segundos.

use std::time::{Duration, Instant};

use crate::stats::Stats;

#[derive(Debug, Clone)]
pub struct Tracker {
    target: Vec<char>,
    typed: Vec<char>,
    keystrokes: u32,
    correct_keystrokes: u32,
    started_at: Option<Instant>,
}

impl Tracker {
    pub fn new(target: Vec<char>) -> Self {
        Self {
            target,
            typed: Vec::with_capacity(64),
            keystrokes: 0,
            correct_keystrokes: 0,
            started_at: None,
        }
    }

    pub fn target(&self) -> &[char] {
        &self.target
    }

    pub fn typed(&self) -> &[char] {
        &self.typed
    }

    pub fn keystrokes(&self) -> u32 {
        self.keystrokes
    }

    pub fn correct_keystrokes(&self) -> u32 {
        self.correct_keystrokes
    }

    pub fn is_complete(&self) -> bool {
        self.typed.len() >= self.target.len()
    }
    /// Tiempo desde la primera pulsación. Cero si todavía no se tipeó nada.
    pub fn elapsed(&self, now: Instant) -> Duration {
        match self.started_at {
            Some(started) => now.saturating_duration_since(started),
            None => Duration::ZERO,
        }
    }

    pub fn stats(&self, elapsed: Duration) -> Stats {
        Stats::compute(
            &self.target,
            &self.typed,
            self.keystrokes,
            self.correct_keystrokes,
            elapsed,
        )
    }

    /// Registra una pulsación. Devuelve `true` si se aceptó.
    ///
    /// Se ignora lo que sobra: si ya se llegó al final de la frase, o si la
    /// pulsación no es un carácter simple (el filtrado de pegado/IME ocurre en
    /// la capa de UI, antes de llegar acá).
    pub fn push(&mut self, c: char, now: Instant) -> bool {
        if self.is_complete() {
            return false;
        }

        // El cronómetro de la sesión arranca con la primera tecla, no al abrir
        // la ventana: no se cuenta el tiempo que tardás en decidirte.
        if self.started_at.is_none() {
            self.started_at = Some(now);
        }

        let expected = self.target[self.typed.len()];
        self.keystrokes += 1;
        if c == expected {
            self.correct_keystrokes += 1;
        }

        self.typed.push(c);
        true
    }

    /// Borra el último carácter. Devuelve `true` si borró algo.
    ///
    /// No descuenta la pulsación: un error corregido sigue penalizando la
    /// precisión, que es como se mide en un test de tipeo real.
    pub fn backspace(&mut self) -> bool {
        self.typed.pop().is_some()
    }
}

#[derive(Debug, Clone)]
pub enum Session {
    Running(Tracker),
    Finished { tracker: Tracker, stats: Stats },
}

impl Session {
    pub fn new(target: Vec<char>) -> Self {
        Self::Running(Tracker::new(target))
    }

    fn tracker(&self) -> &Tracker {
        match self {
            Self::Running(tracker) => tracker,
            Self::Finished { tracker, .. } => tracker,
        }
    }

    pub fn target(&self) -> &[char] {
        self.tracker().target()
    }

    pub fn typed(&self) -> &[char] {
        self.tracker().typed()
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, Self::Finished { .. })
    }

    pub fn keystrokes(&self) -> u32 {
        self.tracker().keystrokes()
    }

    pub fn correct_keystrokes(&self) -> u32 {
        self.tracker().correct_keystrokes()
    }

    /// `None` mientras estás tipeando: no hay métricas que mostrar todavía.
    pub fn stats(&self) -> Option<Stats> {
        match self {
            Self::Running(_) => None,
            Self::Finished { stats, .. } => Some(*stats),
        }
    }

    pub fn push(&mut self, c: char, now: Instant) {
        let Self::Running(tracker) = self else {
            return; // sesión terminada: se ignora
        };

        if !tracker.push(c, now) {
            return;
        }

        if !tracker.is_complete() {
            return;
        }

        // Se clona el tracker para poder reemplazar `*self` sin pelear con el
        // préstamo mutable que devuelve el `let ... else` de arriba.
        let elapsed = tracker.elapsed(now);
        let finished = tracker.clone();
        let stats = finished.stats(elapsed);
        *self = Self::Finished {
            tracker: finished,
            stats,
        };
    }

    pub fn backspace(&mut self) {
        if let Self::Running(tracker) = self {
            tracker.backspace();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    /// S5: el cronómetro arranca en la primera pulsación, no al abrir la app.
    #[test]
    fn el_tiempo_arranca_en_la_primera_pulsacion() {
        let t0 = Instant::now();
        let mut tracker = Tracker::new(chars("ab"));

        assert_eq!(
            tracker.elapsed(t0 + Duration::from_secs(30)),
            Duration::ZERO
        );

        tracker.push('a', t0);
        assert_eq!(
            tracker.elapsed(t0 + Duration::from_secs(30)),
            Duration::from_secs(30)
        );
    }

    /// S6: no se puede tipear más allá del final de la frase.
    #[test]
    fn ignora_el_input_que_sobra() {
        let t0 = Instant::now();
        let mut session = Session::new(chars("ab"));

        session.push('a', t0);
        session.push('b', t0);
        session.push('c', t0);
        session.push('d', t0);

        assert_eq!(session.typed().len(), 2);
        assert!(session.is_finished());
    }

    /// S7: `keystrokes` cuenta toda pulsación aceptada; el acierto solo suma si
    /// la letra coincide con la posición correspondiente.
    #[test]
    fn cuenta_pulsaciones_y_aciertos() {
        let t0 = Instant::now();
        let mut session = Session::new(chars("abc"));

        session.push('a', t0); // acierto
        session.push('x', t0); // error

        let Session::Running(tracker) = &session else {
            panic!("la sesión no debería haber terminado");
        };
        assert_eq!(tracker.keystrokes(), 2);
        assert_eq!(tracker.correct_keystrokes(), 1);
    }

    /// S8: borrar un error no perdona el error en la precisión.
    ///
    /// El error va en el medio a propósito: la sesión termina al llegar al
    /// final de la frase (S10), así que un error en la última letra ya no se
    /// puede corregir.
    #[test]
    fn borrar_no_perdona_el_error() {
        let t0 = Instant::now();
        let mut session = Session::new(chars("abc"));

        session.push('a', t0);
        session.push('x', t0 + Duration::from_secs(1)); // error
        session.backspace();
        session.push('b', t0 + Duration::from_secs(2)); // corrige
        session.push('c', t0 + Duration::from_secs(3)); // termina

        let stats = session.stats().expect("la sesión terminó");
        // 4 pulsaciones, 3 aciertos: el error corregido sigue contando.
        assert!(
            (stats.accuracy - 75.0).abs() < 1e-4,
            "accuracy = {}",
            stats.accuracy
        );
    }

    /// S9: borrar en una frase vacía no rompe nada.
    #[test]
    fn backspace_sobre_vacio_no_hace_nada() {
        let mut session = Session::new(chars("ab"));

        session.backspace();
        session.backspace();

        assert!(session.typed().is_empty());
    }

    /// S10: al completar la frase, la sesión queda terminada y las métricas
    /// quedan congeladas (no siguen cambiando con el reloj).
    #[test]
    fn al_completar_congela_las_metricas() {
        let t0 = Instant::now();
        let mut session = Session::new(chars("ab"));

        session.push('a', t0);
        session.push('b', t0 + Duration::from_secs(60));

        let first = session.stats().expect("la sesión terminó");
        assert!((first.wpm - 0.4).abs() < 1e-4, "wpm = {}", first.wpm);

        // Sesenta segundos más tarde las métricas no pueden haber cambiado.
        session.push('c', t0 + Duration::from_secs(120));
        assert_eq!(session.stats(), Some(first));
    }

    /// S11: una sesión terminada ignora tanto caracteres como borrados.
    #[test]
    fn una_sesion_terminada_ignora_todo() {
        let t0 = Instant::now();
        let mut session = Session::new(chars("a"));
        session.push('a', t0);

        session.backspace();
        session.push('b', t0);

        assert_eq!(session.typed(), chars("a").as_slice());
        assert!(session.is_finished());
    }
}
