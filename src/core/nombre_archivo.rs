use std::fmt;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
pub struct NombreArchivo(String);

impl NombreArchivo {
    pub fn new(nombre: impl Into<String>) -> Result<Self, NombreArchivoError> {
        let nombre_limpio = nombre.into().trim().to_string();
        ValidadorNombreArchivo::validar(&nombre_limpio)?;
        Ok(Self::generado())
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

impl fmt::Display for NombreArchivo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}


impl NombreArchivo {
    pub fn generado() -> Self {
        Self(Uuid::new_v4().to_string())
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
    fn genera_uuid_v4_desde_diferentes_nombres() {
        let entradas = [
            "imagen",
            "foto_perfil",
            "vacaciones",
            "documento",
            "captura_pantalla",
        ];

        for entrada in entradas {
            let resultado = NombreArchivo::new(entrada).unwrap();

            println!("Entrada: {entrada} -> Resultado: {}", resultado.as_str());
        }
    }
}