use std::fmt;
use std::path::{Component, Path, PathBuf};
use crate::vo::NombreArchivo;

/// Directorios predefinidos para almacenar imágenes.
#[derive(Debug, PartialEq, Eq)]
pub enum DirectorioImagenes {
    /// Directorio por defecto para uploads generales.
    Uploads,
    Temporales,
    FotoPerfil,
    Banner,
}

impl DirectorioImagenes {

    /// Devuelve la ruta asociada a cada variante en formato de cadena relativa.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uploads => "static/uploads/imagenes",
            Self::Temporales => "static/tmp/imagenes",
            Self::FotoPerfil => "static/uploads/perfiles",
            Self::Banner => "static/uploads/banners",
        }
    }
}

impl Default for DirectorioImagenes {
    fn default() -> Self {
        Self::Uploads
    }
}

impl fmt::Display for DirectorioImagenes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Errores posibles al construir `ConfigRuta` a partir del entorno.
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq)]
pub enum ConfigRutaError {
    #[error("La variable de entorno '{clave}' no está definida")]
    VariableNoDefinida { clave: String },
    #[error("La variable de entorno '{clave}' contiene caracteres UTF-8 no válidos")]
    VariableNoUnicode { clave: String },
}

#[derive(Debug, PartialEq, Eq)]
pub struct ConfigRuta {
    directorio_base: PathBuf,
}

impl ConfigRuta {

    fn new(directorio_base: impl Into<PathBuf>) -> Self {
        Self { directorio_base: directorio_base.into() }
    }

    /// Crea la configuración leyendo únicamente la variable de entorno especificada.
    /// Retorna un `ConfigRutaError` tipado si la variable no existe o no es UTF-8 válido.
    pub fn solo_env(env: impl AsRef<str>) -> Result<Self, ConfigRutaError> {
        let clave = env.as_ref();
        
        std::env::var(clave)
            .map(Self::new)
            .map_err(|err| match err {
                std::env::VarError::NotPresent => ConfigRutaError::VariableNoDefinida {
                    clave: clave.to_string(),
                },
                std::env::VarError::NotUnicode(_) => ConfigRutaError::VariableNoUnicode {
                    clave: clave.to_string(),
                },
            })
    }

    /// Crea la configuración a partir de un directorio predefinido.
    pub fn desde_directorio(dir: DirectorioImagenes) -> Self {
        Self::new(dir.as_str())
    }

    pub fn directorio_base(&self) -> &Path {
        &self.directorio_base
    }

    /// Componentes normales (nombres) del directorio base, usados por el
    /// validador para comprobar que una ruta comienza con este prefijo.
    fn componentes(&self) -> Vec<&str> {
        self.directorio_base
            .components()
            .filter_map(|c| match c {
                Component::Normal(name) => name.to_str(),
                _ => None,
            })
            .collect()
    }
}

impl Default for ConfigRuta {
    fn default() -> Self {
        Self::desde_directorio(DirectorioImagenes::default())
    }
}

// VALUE OBJECT: Ruta

#[derive(Debug, PartialEq, Eq)]
pub struct Ruta(PathBuf);

impl Ruta {
    /// Constructor principal. Delega la verificación al `ValidadorRuta`,
    /// usando el directorio base indicado por `config`.
    pub fn try_new(path: impl Into<PathBuf>, config: &ConfigRuta) -> Result<Self, ErrorRuta> {
        let path_buf = path.into();
        ValidadorRuta::validar(&path_buf, config)?;
        Ok(Self(path_buf))
    }

    /// Construye la ruta completa de un archivo usando el directorio base
    /// definido en `config`, en vez de una constante interna.
    pub fn para_archivo(
        filename: &NombreArchivo,
        config: &ConfigRuta,
    ) -> Result<Self, ErrorRuta> {
        let path = config.directorio_base().join(filename.as_str());
        Self::try_new(path, config)
    }

    /// Obtiene una referencia a la ruta interna como `&Path`.
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Obtiene una referencia a la ruta interna como `&PathBuf`.
    pub fn as_path_buf(&self) -> &PathBuf {
        &self.0
    }

    /// Devuelve la ruta en formato apto para URLs Web (siempre con '/').
    pub fn to_web_string(&self) -> String {
        self.0.to_string_lossy().replace('\\', "/")
    }

    /// Consume el Value Object y devuelve el `PathBuf` subyacente.
    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

impl Ruta {
    /// Reconstruye una `Ruta` directamente desde la base de datos sin aplicar restricciones de validación de directorio.
    pub fn desde_db(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }
}

impl fmt::Display for Ruta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_web_string())
    }
}

// =============================================================================
// VALIDADOR
// =============================================================================
pub struct ValidadorRuta;

impl ValidadorRuta {
    /// Límite típico de longitud total de una ruta (PATH_MAX en Linux).
    const MAX_LONGITUD_RUTA: usize = 4096;
    /// Límite típico de longitud de un solo componente (nombre de archivo/dir).
    const MAX_LONGITUD_COMPONENTE: usize = 255;

    /// Ejecuta el pipeline completo de validación sobre la ruta proporcionada,
    /// comprobando el prefijo contra el directorio base de `config`.
    pub fn validar(path: &Path, config: &ConfigRuta) -> Result<(), ErrorRuta> {
        Self::validar_no_vacio(path)?;
        Self::validar_no_absoluta(path)?;
        Self::validar_utf8_y_sin_bytes_nulos(path)?;
        Self::validar_longitudes(path)?;
        Self::validar_seguridad_path_traversal(path)?;
        Self::validar_prefijo_base(path, config)?;
        Ok(())
    }

    /// Regla 1: La ruta no puede ser una cadena de texto o Path vacío.
    fn validar_no_vacio(path: &Path) -> Result<(), ErrorRuta> {
        if path.as_os_str().is_empty() {
            Err(ErrorRuta::RutaVacia)
        } else {
            Ok(())
        }
    }

    /// Regla 2: La ruta debe ser relativa. Una ruta absoluta (p. ej.
    /// "/etc/passwd" o "C:\Windows\...") no debe poder combinarse ni
    /// confundirse con el directorio base, y `Path::join` con una ruta
    /// absoluta ignora silenciosamente el prefijo, lo cual es peligroso.
    fn validar_no_absoluta(path: &Path) -> Result<(), ErrorRuta> {
        if path.is_absolute() {
            Err(ErrorRuta::RutaAbsoluta)
        } else {
            Ok(())
        }
    }

    /// Regla 3: La ruta debe ser UTF-8 válido y no contener bytes nulos.
    /// Un byte nulo incrustado puede usarse para truncar la ruta en APIs
    /// de bajo nivel (C, algunos drivers de FS) y provocar que se acceda
    /// a un archivo distinto del validado.
    fn validar_utf8_y_sin_bytes_nulos(path: &Path) -> Result<(), ErrorRuta> {
        match path.to_str() {
            Some(s) if s.contains('\0') => Err(ErrorRuta::CaracterInvalido),
            Some(_) => Ok(()),
            None => Err(ErrorRuta::CodificacionInvalida),
        }
    }

    /// Regla 4: Límite de longitud total y por componente, para evitar
    /// errores de sistema de archivos (ENAMETOOLONG) o abuso con nombres
    /// desproporcionadamente largos.
    fn validar_longitudes(path: &Path) -> Result<(), ErrorRuta> {
        if path.as_os_str().len() > Self::MAX_LONGITUD_RUTA {
            return Err(ErrorRuta::RutaDemasiadoLarga);
        }
        for componente in path.components() {
            if let Component::Normal(nombre) = componente {
                if nombre.len() > Self::MAX_LONGITUD_COMPONENTE {
                    return Err(ErrorRuta::ComponenteDemasiadoLargo);
                }
            }
        }
        Ok(())
    }

    /// Regla 5: Prevención de ataques de tipo Path Traversal (`..`).
    fn validar_seguridad_path_traversal(path: &Path) -> Result<(), ErrorRuta> {
        if path.components().any(|c| matches!(c, Component::ParentDir)) {
            Err(ErrorRuta::NavegacionProhibida)
        } else {
            Ok(())
        }
    }

    /// Regla 6: Verifica que los primeros componentes normales de `path`
    /// coincidan con los del directorio base definido en `config`, y que
    /// haya al menos un componente adicional (el archivo/subdirectorio).
    fn validar_prefijo_base(path: &Path, config: &ConfigRuta) -> Result<(), ErrorRuta> {
        let prefijo_esperado = config.componentes();

        let componentes_normales: Vec<&str> = path
            .components()
            .filter_map(|c| match c {
                Component::Normal(name) => name.to_str(),
                _ => None,
            })
            .collect();

        let tiene_prefijo = componentes_normales.len() > prefijo_esperado.len()
            && componentes_normales
                .iter()
                .zip(prefijo_esperado.iter())
                .all(|(componente, esperado)| componente == esperado);

        if tiene_prefijo {
            Ok(())
        } else {
            Err(ErrorRuta::BaseInvalida)
        }
    }

    /// Chequeo OPCIONAL y con I/O: resuelve symlinks tanto de la ruta como
    /// del directorio base y verifica que la ruta canónica siga estando
    /// dentro del directorio base canónico.
    ///
    /// Esta es la defensa más robusta contra path traversal (más que
    /// cualquier chequeo basado solo en el texto de la ruta), porque cubre
    /// el caso de un symlink ya existente dentro del directorio de subida
    /// que apunta fuera de él. No forma parte de `validar()` porque:
    /// - Requiere que la ruta exista en disco (falla si no existe todavía,
    ///   p. ej. antes de escribir un archivo nuevo).
    /// - Hace I/O, lo cual no encaja con la construcción "pura" del VO.
    ///
    /// Úsalo justo antes de leer o sobrescribir un archivo ya existente en
    /// disco, no al construir el `Ruta` en memoria.
    pub fn validar_resolucion_simbolica(
        path: &Path,
        config: &ConfigRuta,
    ) -> Result<(), ErrorRuta> {
        let base_canonica = config
            .directorio_base()
            .canonicalize()
            .map_err(|_| ErrorRuta::DirectorioBaseInaccesible)?;

        let ruta_canonica = path
            .canonicalize()
            .map_err(|_| ErrorRuta::ArchivoInaccesible)?;

        if ruta_canonica.starts_with(&base_canonica) {
            Ok(())
        } else {
            Err(ErrorRuta::EscapeSimbolicoDetectado)
        }
    }
}

// =============================================================================
// TESTS
// =============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acepta_ruta_dentro_del_directorio_configurado() {
        let config = ConfigRuta::default(); // "uploads/imagenes"
        let ruta = Ruta::try_new("uploads/imagenes/foto.png", &config);
        assert!(ruta.is_ok());
    }

    #[test]
    fn rechaza_ruta_fuera_del_directorio_configurado() {
        let config = ConfigRuta::default();
        let ruta = Ruta::try_new("otros/foto.png", &config);
        assert_eq!(ruta, Err(ErrorRuta::BaseInvalida));
    }

    #[test]
    fn respeta_directorio_base_personalizado() {
        // Por ejemplo, un directorio distinto para tests o para otro entorno.
        let config = ConfigRuta::new("test/imagenes");
        let ruta = Ruta::try_new("test/imagenes/foto.png", &config);
        assert!(ruta.is_ok());

        // La misma ruta falla contra la config por defecto.
        let config_default = ConfigRuta::default();
        let ruta_invalida = Ruta::try_new("test/imagenes/foto.png", &config_default);
        assert_eq!(ruta_invalida, Err(ErrorRuta::BaseInvalida));
    }

    #[test]
    fn rechaza_path_traversal() {
        let config = ConfigRuta::default();
        let ruta = Ruta::try_new("uploads/imagenes/../../etc/passwd", &config);
        assert_eq!(ruta, Err(ErrorRuta::NavegacionProhibida));
    }

    #[test]
    fn rechaza_ruta_absoluta() {
        let config = ConfigRuta::default();
        let ruta = Ruta::try_new("/uploads/imagenes/foto.png", &config);
        assert_eq!(ruta, Err(ErrorRuta::RutaAbsoluta));
    }

    #[test]
    fn rechaza_byte_nulo() {
        let config = ConfigRuta::default();
        let ruta = Ruta::try_new("uploads/imagenes/foto\0.png", &config);
        assert_eq!(ruta, Err(ErrorRuta::CaracterInvalido));
    }

    #[test]
    fn rechaza_componente_demasiado_largo() {
        let config = ConfigRuta::default();
        let nombre_largo = "a".repeat(300) + ".png";
        let ruta = Ruta::try_new(format!("uploads/imagenes/{nombre_largo}"), &config);
        assert_eq!(ruta, Err(ErrorRuta::ComponenteDemasiadoLargo));
    }
}

// =============================================================================
// ERRORES DE DOMINIO
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErrorRuta {
    #[error("La ruta debe comenzar con la jerarquía 'uploads/imagenes/'")]
    BaseInvalida,
    #[error("La ruta contiene secuencias de navegación inseguras ('..')")]
    NavegacionProhibida,
    #[error("La ruta no puede estar vacía")]
    RutaVacia,
    #[error("La ruta no puede ser absoluta; debe ser relativa al directorio base")]
    RutaAbsoluta,
    #[error("La ruta contiene caracteres nulos no permitidos")]
    CaracterInvalido,
    #[error("La ruta no es UTF-8 válida")]
    CodificacionInvalida,
    #[error("La longitud total de la ruta excede el límite permitido")]
    RutaDemasiadoLarga,
    #[error("Un componente de la ruta excede la longitud máxima permitida")]
    ComponenteDemasiadoLargo,
    #[error("El directorio base no es accesible o no existe")]
    DirectorioBaseInaccesible,
    #[error("La ruta indicada no es accesible o no existe")]
    ArchivoInaccesible,
    #[error("La ruta resuelve simbólicamente fuera del directorio base")]
    EscapeSimbolicoDetectado,
}