//! Frases reales: el texto a tipear y quién lo dijo.
//!
//! El archivo `resources/frases.txt` se embebe en el binario con `include_str!`.
//! Eso tiene dos consecuencias prácticas: la app no depende del directorio
//! desde donde se la ejecute, y el archivo no puede faltar en tiempo de
//! ejecución (si falta, no compila).
//!
//! Formato: una frase por línea, con el autor separado por un **tabulador**.
//!
//! ```text
//! Frase que se tipea<TAB>Autor que la dijo
//! ```
//!
//! El autor es informativo: se muestra debajo de la frase y **no** se tipea.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

/// El archivo de frases, embebido al compilar.
const SOURCE: &str = include_str!("../resources/frases.txt");

/// Largo máximo de una frase, en caracteres.
///
/// La ventana tiene tamaño fijo: una frase más larga no entra en el alto
/// reservado y empuja las métricas fuera de la ventana. El filtro se aplica al
/// cargar, así que una frase descartada simplemente nunca se elige: no hay
/// frases "rotas", solo frases que no entran.
///
/// El número sale de medir, no de elegir: con la fuente de 28 px entran tres
/// líneas (96 px de los 116 disponibles). La frase más larga que todavía entra
/// en tres líneas mide 133 caracteres, así que 130 deja margen para que un
/// cambio de métricas de la fuente no rompa el layout. El test de `app` recorre
/// **todas** las frases del archivo y falla si alguna no entra; si agregás
/// frases nuevas, ese test es el que te avisa.
pub const MAX_CHARS: usize = 130;

/// Una frase y, si se sabe, quién la dijo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phrase {
    text: &'static str,
    author: Option<&'static str>,
}

impl Phrase {
    /// El texto que se tipea.
    pub fn text(&self) -> &'static str {
        self.text
    }

    /// El autor, solo para mostrar. `None` cuando la línea no lo traía.
    pub fn author(&self) -> Option<&'static str> {
        self.author
    }

    /// El texto troceado en caracteres, que es como lo consume [`crate::session`].
    ///
    ///
    /// Se trocea por `char` y no por byte: con acentos, un carácter ocupa dos
    /// bytes y cortar por byte partiría la `ó` al medio.
    pub fn chars(&self) -> Vec<char> {
        self.text().chars().collect()
    }

    /// Lee una línea del archivo. Devuelve `None` si no es una frase usable.
    ///
    /// El autor es opcional porque el archivo tiene líneas sin tabulador. Se
    /// descartan las líneas vacías y las frases que no entran en la ventana.
    fn parse(line: &'static str) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        let (text, author) = match line.split_once('\t') {
            Some((text, author)) => (text.trim(), Some(author.trim())),
            None => (line, None),
        };

        if text.is_empty() || text.chars().count() > MAX_CHARS {
            return None;
        }

        Some(Self {
            text,
            // `""` no es un autor: una línea con el tabulador pero sin nombre
            // se muestra igual que una sin tabulador.
            author: author.filter(|author| !author.is_empty()),
        })
    }
}

/// Todas las frases usables del archivo, en el orden en que aparecen.
///
/// Se recorre una sola vez al arrancar; el azar elige después sobre el arreglo
/// ya filtrado, así una frase descartada nunca puede salir sorteada.
pub fn all() -> Vec<Phrase> {
    SOURCE.lines().filter_map(Phrase::parse).collect()
}

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
    /// elijan la misma frase.
    pub fn from_entropy() -> Self {
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

    /// Un índice al azar en `0..len`.
    ///
    /// Devuelve el índice en vez de la referencia para no tener que prestar el
    /// arreglo y el generador a la vez: `self.rng.pick(&self.phrases)` no
    /// compila, `self.phrases[self.rng.index(len)]` sí.
    pub fn index(&mut self, len: usize) -> usize {
        debug_assert!(len > 0, "index() necesita al menos un elemento");
        self.next_u32() as usize % len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// P1: el archivo se lee y trae frases de verdad.
    #[test]
    fn el_archivo_trae_frases() {
        assert!(
            all().len() > 100,
            "solo se cargaron {} frases: ¿se rompió el parseo?",
            all().len()
        );
    }

    /// P2: una frase es texto tipeable: sin tabuladores, sin controles, sin
    /// espacios en los bordes.
    #[test]
    fn el_texto_esta_limpio() {
        for phrase in all() {
            let text = phrase.text();
            assert!(!text.is_empty(), "hay una frase vacía");
            assert!(
                !text.contains('\t'),
                "el tabulador quedó dentro del texto: {text:?}"
            );
            assert!(
                !text.chars().any(char::is_control),
                "hay un carácter de control en {text:?}"
            );
            assert_eq!(
                text.trim(),
                text,
                "la frase tiene espacios en los bordes: {text:?}"
            );
        }
    }

    /// P3: el autor es opcional. El archivo tiene líneas sin tabulador, y esas
    /// no pueden romper el parseo ni inventar un autor vacío.
    #[test]
    fn el_autor_es_opcional() {
        let phrases = all();

        assert!(
            phrases.iter().any(|p| p.author().is_some()),
            "ninguna frase tiene autor"
        );
        assert!(
            phrases.iter().any(|p| p.author().is_none()),
            "ninguna frase quedó sin autor: el archivo sí tiene líneas así"
        );

        for phrase in &phrases {
            if let Some(author) = phrase.author() {
                assert!(!author.is_empty(), "hay un autor vacío");
                assert_eq!(author.trim(), author, "el autor tiene espacios: {author:?}");
            }
        }
    }

    /// P4: las frases que no entran en la ventana se descartan al cargar.
    #[test]
    fn las_frases_largas_se_descartan() {
        let demasiado_larga = |line: &str| line.split('\t').next().unwrap_or("").chars().count();

        assert!(
            SOURCE.lines().any(|line| demasiado_larga(line) > MAX_CHARS),
            "el archivo no tiene ninguna frase de más de {MAX_CHARS} caracteres: el filtro no se está probando"
        );

        for phrase in all() {
            let largo = phrase.text().chars().count();
            assert!(
                largo <= MAX_CHARS,
                "{largo} caracteres (máximo {MAX_CHARS}): {:?}",
                phrase.text()
            );
        }
    }

    /// P5: los acentos sobreviven al parseo. Es la razón de existir del cambio:
    /// antes el vocabulario era ASCII puro y las frases reales no entraban.
    #[test]
    fn los_acentos_sobreviven() {
        let phrases = all();

        let con_acento = phrases.iter().filter(|p| !p.text().is_ascii()).count();
        assert!(
            con_acento > 100,
            "solo {con_acento} frases tienen acentos: se esperaban muchas más"
        );

        // Trocear y volver a armar no puede cambiar el texto.
        for phrase in &phrases {
            let round_trip: String = phrase.chars().into_iter().collect();
            assert_eq!(round_trip, phrase.text(), "el troceo perdió caracteres");
        }
    }

    /// P6: misma semilla, misma elección (así los tests son reproducibles).
    #[test]
    fn la_misma_semilla_elige_lo_mismo() {
        let phrases = all();
        let mut a = Rng::from_seed(42);
        let mut b = Rng::from_seed(42);

        for _ in 0..50 {
            assert_eq!(a.index(phrases.len()), b.index(phrases.len()));
        }
    }

    /// P7: el índice nunca se sale del arreglo.
    #[test]
    fn el_indice_nunca_se_sale() {
        let phrases = all();
        let mut rng = Rng::from_seed(3);

        for _ in 0..1_000 {
            assert!(rng.index(phrases.len()) < phrases.len());
        }
    }
}
