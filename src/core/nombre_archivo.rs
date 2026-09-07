use std::fmt;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
pub struct NombreArchivo(String);

impl NombreArchivo {

    pub fn new(nombre: impl Into<String>) -> Result<Self, NombreArchivoError> {
        let nombre_limpio = nombre.into().trim().to_string();
        ValidadorNombreArchivo::validar(&nombre_limpio)?;
        Ok(Self::generado(&nombre_limpio))
    }

    /// Devuelve la representación en cadena como `&str`.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume el Value Object y devuelve la cadena interna `String`.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl NombreArchivo {
    /// Reconstruye un `NombreArchivo` directamente desde la base de datos sin aplicar validaciones de formato o limpieza de espacios.
    pub fn desde_db(nombre: impl Into<String>) -> Self {
        Self(nombre.into())
    }
}

impl fmt::Display for NombreArchivo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl NombreArchivo {
    pub fn fijo(nombre: impl Into<String>) -> Result<Self, NombreArchivoError> {
        let nombre_limpio = nombre.into().trim().to_string();
        ValidadorNombreArchivo::validar(&nombre_limpio)?;
        Ok(Self(nombre_limpio))
    }
}

impl NombreArchivo {
    
    /// Genera un nombre de archivo único usando un UUID v4 y extrayendo 
    /// la extensión del nombre o ruta proporcionada.
    pub fn generado(nombre_o_extension: &str) -> Self {
        let path = Path::new(nombre_o_extension.trim());
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .trim_start_matches('.');

        if extension.is_empty() {
            Self(Uuid::new_v4().to_string())
        } else {
            Self(format!("{}.{}", Uuid::new_v4(), extension))
        }
    }
}

// =============================================================================
// 2. ERRORES DE DOMINIO
// =============================================================================

use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum NombreArchivoError {
    #[error("El nombre del archivo no puede estar vacío")]
    Vacio,
    #[error("La longitud del nombre del archivo debe estar entre 1 y 255 caracteres")]
    LongitudInvalida,
    #[error("El nombre del archivo no puede contener separadores de ruta ('/' o '\\')")]
    ContieneSeparador,
    #[error("El nombre del archivo no es un nombre válido para el sistema de archivos")]
    Invalido,
    #[error("El nombre del archivo está reservado por el sistema (ej. '.' o '..')")]
    Reservado,
}

// =============================================================================
// VALIDADOR
// =============================================================================

pub struct ValidadorNombreArchivo;

impl ValidadorNombreArchivo {
    pub fn validar(nombre: &str) -> Result<(), NombreArchivoError> {
        Self::validar_no_vacio(nombre)?;
        Self::validar_longitud(nombre)?;
        Self::validar_sin_separadores(nombre)?;
        Self::validar_no_reservado(nombre)?;
        Self::validar_estructura_path(nombre)?;
        Ok(())
    }

    /// Regla 1: El nombre de archivo no debe estar vacío.
    fn validar_no_vacio(nombre: &str) -> Result<(), NombreArchivoError> {
        if nombre.is_empty() {
            Err(NombreArchivoError::Vacio)
        } else {
            Ok(())
        }
    }

    /// Regla 2: La longitud del nombre debe estar dentro del rango permitido.
    fn validar_longitud(nombre: &str) -> Result<(), NombreArchivoError> {
        const MIN_LONGITUD: usize = 1;
        const MAX_LONGITUD: usize = 255;

        let longitud = nombre.len();

        if !(MIN_LONGITUD..=MAX_LONGITUD).contains(&longitud) {
            Err(NombreArchivoError::LongitudInvalida)
        } else {
            Ok(())
        }
    }

    /// Regla 3: Un nombre de archivo no debe incluir delimitadores de ruta (`/` o `\`).
    fn validar_sin_separadores(nombre: &str) -> Result<(), NombreArchivoError> {
        if nombre.contains('/') || nombre.contains('\\') {
            Err(NombreArchivoError::ContieneSeparador)
        } else {
            Ok(())
        }
    }

    /// Regla 4: Evita alias reservados del sistema operativo (`.` o `..`).
    fn validar_no_reservado(nombre: &str) -> Result<(), NombreArchivoError> {
        if nombre == "." || nombre == ".." {
            Err(NombreArchivoError::Reservado)
        } else {
            Ok(())
        }
    }

    /// Regla 5: Garantiza que sea interpretable como `file_name`
    /// mediante `std::path::Path`.
    fn validar_estructura_path(nombre: &str) -> Result<(), NombreArchivoError> {
        let path = Path::new(nombre);

        if path.file_name().is_none() {
            Err(NombreArchivoError::Invalido)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Función auxiliar para registrar e imprimir la entrada, salida y errores en consola.
    fn probar_y_notificar(id_test: &str, entrada: &str) -> Result<(), NombreArchivoError> {
        println!("\n--- [Prueba: {}] ---", id_test);
        println!("  📥 Entrada : {:?}", entrada);

        let resultado = ValidadorNombreArchivo::validar(entrada);

        match &resultado {
            Ok(()) => {
                println!("  📤 Salida  : Ok(())");
                println!("  ✅ Estado  : Nombre de archivo válido.");
            }
            Err(err) => {
                println!("  📤 Salida  : Err(NombreArchivoError::{:?})", err);
                println!("  🚨 Mensaje : \"{}\"", err);
            }
        }

        resultado
    }

    #[test]
    fn test_01_nombre_valido_estandar() {
        let entrada = "documento.txt";
        let res = probar_y_notificar("01_NombreValidoEstandar", entrada);
        assert!(res.is_ok());
    }

    #[test]
    fn test_02_nombre_vacio() {
        let entrada = "";
        let res = probar_y_notificar("02_NombreVacio", entrada);
        assert_eq!(res, Err(NombreArchivoError::Vacio));
    }

    #[test]
    fn test_03_longitud_excedida() {
        let entrada = "a".repeat(256);
        let res = probar_y_notificar("03_LongitudExcedida", &entrada);
        assert_eq!(res, Err(NombreArchivoError::LongitudInvalida));
    }

    #[test]
    fn test_04_separador_posix_slash() {
        let entrada = "carpeta/archivo.txt";
        let res = probar_y_notificar("04_SeparadorPosixSlash", entrada);
        assert_eq!(res, Err(NombreArchivoError::ContieneSeparador));
    }

    #[test]
    fn test_05_separador_windows_backslash() {
        let entrada = "carpeta\\archivo.txt";
        let res = probar_y_notificar("05_SeparadorWindowsBackslash", entrada);
        assert_eq!(res, Err(NombreArchivoError::ContieneSeparador));
    }

    #[test]
    fn test_06_reservado_punto_simple() {
        let entrada = ".";
        let res = probar_y_notificar("06_ReservadoPuntoSimple", entrada);
        assert_eq!(res, Err(NombreArchivoError::Reservado));
    }

    #[test]
    fn test_07_reservado_doble_punto() {
        let entrada = "..";
        let res = probar_y_notificar("07_ReservadoDoblePunto", entrada);
        assert_eq!(res, Err(NombreArchivoError::Reservado));
    }

    #[test]
    fn test_08_caracteres_unicode_y_emojis() {
        let entrada = "reporte_año_2026_📊.pdf";
        let res = probar_y_notificar("08_CaracteresUnicodeYEmojis", entrada);
        assert!(res.is_ok());
    }

    #[test]
    fn test_09_multiples_extensiones() {
        let entrada = "backup.tar.gz";
        let res = probar_y_notificar("09_MultiplesExtensiones", entrada);
        assert!(res.is_ok());
    }

    #[test]
    fn test_10_espacios_y_simbolos_permitidos() {
        let entrada = "mi documento - borrador (v2).docx";
        let res = probar_y_notificar("10_EspaciosYSimbolosPermitidos", entrada);
        assert!(res.is_ok());
    }
}

#[cfg(test)]
mod tests2 {
    use super::*;

    #[test]
    fn test_diez_entradas_new() {
        let entradas = vec![
            "documento.txt",
            "imagen.png",
            "archivo_sin_extension",
            "ruta/relativa/archivo.pdf",
            "  archivo_con_espacios.rs  ",
            "comprimido.tar.gz",
            ".archivo_oculto.md",
            "documento.FINAL.docx",
            "audio.mp3",
            "script.py",
        ];

        for entrada in entradas {
            match NombreArchivo::new(entrada) {
                Ok(nombre) => println!("Original: {:<30} | UUID Generado: {}", entrada, nombre.0),
                Err(e) => println!("Original: {:<30} | Error de validación: {:?}", entrada, e),
            }
        }
    }
}

#[cfg(test)]
mod tests3 {
    use super::*;

    #[test]
    fn test_entradas_que_rompen_new() {
        let entradas_invalidas = vec![
            "",                                  // Vacío
            "   ",                               // Solo espacios
            "archivo/con/slashes/invalidos.txt", // Caracteres prohibidos en rutas o nombres planos
            "archivo\\con\\backslash.txt",       // Backslashes
            "archivo?invalido.png",              // Signos de interrogación
            "archivo*invalido.txt",              // Asteriscos
            "archivo<invalido>.doc",             // Signos de mayor/menor
            "archivo:invalido.pdf",              // Dos puntos
            "archivo|invalido.mp3",              // Pipe
            "\0archivo_nulo.txt",                // Carácter nulo
        ];

        for entrada in entradas_invalidas {
            match NombreArchivo::new(entrada) {
                Ok(nombre) => println!("Inesperado OK para '{}': {}", entrada, nombre.0),
                Err(e) => println!("Error atrapado correctamente para '{}': {:?}", entrada, e),
            }
        }
    }
}