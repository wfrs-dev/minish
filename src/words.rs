//! Frase objetivo: lista de palabras y generación pseudoaleatoria.
//!
//! Dos decisiones deliberadas:
//!
//! 1. **Sin acentos ni diéresis.** Un test de tipeo mide velocidad de dedos, no
//!    si acertás una dead key. Con ASCII puro (más la ñ) el tipeo no depende
//!    del layout del teclado.
//! 2. **Sin dependencias.** Un LCG (generador congruencial lineal) alcanza de
//!    sobra para elegir palabras y evita sumar `rand` al proyecto. No es
//!    criptográfico ni falta que lo sea: solo querés que la frase cambie.

/// Palabras disponibles. Todas en minúsculas, sin espacios y sin acentos.
pub const WORDS: &[&str] = &[
    // Las del mockup, para poder reproducir la imagen tal cual.
    "oreo", "dos", "irse", "sano", "sed", "inaudita", "ura", "rata", "distrito", "tendido", "usado",
    "inaudito", "nudo", "aturdido", "tronada", "tenso", "usted", "es", "antro",
    "rodador", // Vocabulario general.
    "casa", "perro", "gato", "mesa", "silla", "ventana", "puerta", "libro", "papel", "mano",
    "dedo", "tecla", "ritmo", "tiempo", "linea", "texto", "mundo", "campo", "cielo", "noche",
    "luz", "mar", "rio", "viento", "piedra", "agua", "fuego", "tierra", "sombra", "verde",
    "blanco", "rojo", "largo", "corto", "nuevo", "viejo", "fuerte", "suave", "rapido", "lento",
    "claro", "oscuro", "simple", "doble", "tres", "cuatro", "cinco", "siete", "nueve", "cero",
    "punto", "coma", "barra", "signo", "frase", "palabra", "letra", "numero", "mañana", "niño",
    "señal", "año",
];

/// Generador congruencial lineal mínimo.
///
/// Estado de 64 bits, multiplicador y salto de los parámetros de Knuth. Se
/// devuelven los bits altos (`>> 33`) porque los bajos de un LCG tienen período
/// corto y se correlacionan.
#[derive(Debug, Clone)]
pub struct Rng(u64);

impl Rng {
    /// Semilla nueva por proceso, tomada del sistema.
    ///
    /// `RandomState` ya está sembrado con entropía del sistema operativo; se le
    /// mezcla el reloj para que dos procesos arrancados en el mismo instante no
    /// generen la misma frase.
    pub fn from_entropy() -> Self {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        use std::time::{SystemTime, UNIX_EPOCH};

        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);

        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u128(nanos);

        Self::from_seed(hasher.finish())
    }

    /// Semilla fija: misma semilla, misma secuencia. Es lo que usan los tests.
    pub fn from_seed(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }

    /// Elige un elemento al azar. `items` no puede estar vacío.
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        debug_assert!(!items.is_empty(), "pick() necesita al menos un elemento");
        &items[self.next_u32() as usize % items.len()]
    }
}

/// Arma una frase de `word_count` palabras separadas por un espacio.
pub fn phrase(rng: &mut Rng, word_count: usize) -> Vec<char> {
    let mut text = String::new();

    for i in 0..word_count {
        if i > 0 {
            text.push(' ');
        }
        text.push_str(rng.pick(WORDS));
    }

    text.chars().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// W1: la lista cumple el invariante en el que se apoya todo lo demás.
    #[test]
    fn todas_las_palabras_son_tipeables() {
        assert!(!WORDS.is_empty());

        for word in WORDS {
            assert!(!word.is_empty(), "hay una palabra vacía en WORDS");
            assert!(
                word.chars().all(|c| c.is_ascii_lowercase() || c == 'ñ'),
                "la palabra {word:?} tiene caracteres que no se tipean directo"
            );
        }
    }

    /// W2: la frase tiene exactamente la cantidad pedida, separada por espacios.
    #[test]
    fn la_frase_tiene_la_cantidad_pedida_de_palabras() {
        let mut rng = Rng::from_seed(7);

        let phrase: String = phrase(&mut rng, 10).iter().collect();

        assert_eq!(phrase.split(' ').count(), 10);
    }

    /// W3: no hay separadores dobles ni espacios al borde.
    #[test]
    fn los_separadores_son_simples() {
        let mut rng = Rng::from_seed(11);

        let phrase: String = phrase(&mut rng, 8).iter().collect();

        assert!(!phrase.starts_with(' '));
        assert!(!phrase.ends_with(' '));
        assert!(!phrase.contains("  "));
    }

    /// W4: misma semilla, misma frase (así los tests son reproducibles).
    #[test]
    fn la_misma_semilla_da_la_misma_frase() {
        let a = phrase(&mut Rng::from_seed(42), 12);
        let b = phrase(&mut Rng::from_seed(42), 12);

        assert_eq!(a, b);
    }

    /// W5: semillas distintas dan frases distintas.
    #[test]
    fn semillas_distintas_dan_frases_distintas() {
        let a = phrase(&mut Rng::from_seed(1), 12);
        let b = phrase(&mut Rng::from_seed(2), 12);

        assert_ne!(a, b);
    }

    /// W6: el generador nunca se sale de la lista.
    #[test]
    fn pick_siempre_devuelve_un_elemento_de_la_lista() {
        let mut rng = Rng::from_seed(3);

        for _ in 0..1_000 {
            let word = rng.pick(WORDS);
            assert!(WORDS.contains(word));
        }
    }
}
