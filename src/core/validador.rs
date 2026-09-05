use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::{GenericImageView, ImageError, ImageFormat};
use std::fmt;
use std::io::{Cursor, Read};
use std::path::Path;
use std::collections::HashMap;
use crate::imagen_metadata::ImagenMetadata;

// ================= ERRORES =================

/// Error de validación con mensaje y detalles opcionales clave-valor.
#[derive(Debug)]
pub struct ImagenValidationError {
    pub message: String,
    pub details: HashMap<String, String>,
}

impl ImagenValidationError {
    /// Crea un error simple sin detalles.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            details: HashMap::new(),
        }
    }

    /// Crea un error con contexto adicional en pares clave-valor.
    pub fn with_details(
        message: impl Into<String>,
        details: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        Self {
            message: message.into(),
            details: details
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }
}

impl fmt::Display for ImagenValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.details.is_empty() {
            write!(f, "{}", self.message)
        } else {
            let details: Vec<String> = self
                .details
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect();
            write!(f, "{} | {{{}}}", self.message, details.join(", "))
        }
    }
}

impl std::error::Error for ImagenValidationError {}

// ================= CONFIGURACIÓN =================

/// Configuración completa del validador: formatos, tamaños y reglas opcionales.
#[derive(Debug)]
pub struct ImagenConfig {
    pub extensiones_permitidas: Vec<String>,
    pub mimes_permitidos: Vec<String>,

    pub max_bytes: usize,
    pub min_bytes: usize,

    /// Límite de píxeles totales para prevenir ataques DoS con imágenes gigantes.
    pub max_image_pixels: u64,
    pub validar_coherencia_extension_mime: bool,
}

impl ImagenConfig {
    // Límites para validaciones específicas (10MB y 5MB)
    pub const LIMITE_10MB_BYTES: usize = 10 * 1024 * 1024;
    pub const LIMITE_10MB_PIXELS: u64 = 20_000_000;

    pub const LIMITE_5MB_BYTES: usize = 5 * 1024 * 1024;
    pub const LIMITE_5MB_PIXELS: u64 = 15_000_000;

    // Constantes para los valores por defecto de la configuración
    const DEFAULT_MAX_BYTES: usize = 20 * 1024 * 1024; // 20 MB
    const DEFAULT_MIN_BYTES: usize = 1024;             // 1 KB
    const DEFAULT_MAX_IMAGE_PIXELS: u64 = 30_000_000;
    const DEFAULT_VALIDAR_COHERENCIA_EXTENSION_MIME: bool = true;
}

impl Default for ImagenConfig {
    fn default() -> Self {
        Self {
            extensiones_permitidas: vec!["jpg".into(), "jpeg".into(), "png".into(), "webp".into()],
            mimes_permitidos: vec![
                "image/jpeg".into(),
                "image/jpg".into(),
                "image/png".into(),
                "image/webp".into(),
            ],
            max_bytes: Self::DEFAULT_MAX_BYTES,
            min_bytes: Self::DEFAULT_MIN_BYTES,
            max_image_pixels: Self::DEFAULT_MAX_IMAGE_PIXELS,
            validar_coherencia_extension_mime: Self::DEFAULT_VALIDAR_COHERENCIA_EXTENSION_MIME,
        }
    }
}

impl ImagenConfig {
    /// Verifica coherencia interna de la configuración antes de usarla.
    pub fn validate(&self) -> Result<(), ImagenValidationError> {
        if self.max_bytes <= self.min_bytes {
            return Err(ImagenValidationError::new(
                "max_bytes debe ser mayor que min_bytes",
            ));
        }
        Ok(())
    }
}

// ================= UTILIDADES =================

/// Convierte bytes a unidad legible (KB, MB, etc.).
pub fn formatear_bytes(n: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut value = n as f64;
    let mut unit = UNITS[0];
    for u in &UNITS[1..] {
        if value < 1024.0 {
            break;
        }
        value /= 1024.0;
        unit = u;
    }
    format!("{:.1} {}", value, unit)
}

/// Extrae y normaliza la extensión del nombre de archivo.
pub fn obtener_extension(nombre: &str) -> Option<String> {
    Path::new(nombre)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

// ================= VALIDADOR PRINCIPAL =================

/// Valida imágenes según la configuración provista.
#[derive(Default)]
pub struct ImagenValidator {
    pub config: ImagenConfig,
}

impl ImagenValidator {
    /// Crea un validador verificando que la configuración sea coherente.
    pub fn new(config: ImagenConfig) -> Result<Self, ImagenValidationError> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Verifica que el nombre tenga una extensión permitida. Retorna la extensión en minúsculas.
    fn validar_nombre_extension(&self, nombre: &str) -> Result<String, ImagenValidationError> {
        if nombre.trim().is_empty() {
            return Err(ImagenValidationError::new("Nombre de archivo vacío"));
        }

        let ext = obtener_extension(nombre).unwrap_or_default();

        if ext.is_empty() || !self.config.extensiones_permitidas.contains(&ext) {
            return Err(ImagenValidationError::with_details(
                format!("Extensión no permitida: '{}'", ext),
                [("permitidas", self.config.extensiones_permitidas.join(", "))],
            ));
        }

        Ok(ext)
    }

    /// Detecta el MIME real leyendo solo los primeros 2 KB del archivo (independiente de lo que
    /// el `Read` entregue por llamada: usa `take().read_to_end()` en vez de una sola `read()`).
    fn validar_mime_rapido<R: Read>(&self, reader: &mut R) -> Result<String, ImagenValidationError> {
        let mut header = Vec::with_capacity(2048);
        reader
            .take(2048)
            .read_to_end(&mut header)
            .map_err(|e| ImagenValidationError::new(format!("Error leyendo header: {}", e)))?;

        let kind = infer::get(&header)
            .ok_or_else(|| ImagenValidationError::new("Formato de archivo no identificable"))?;

        let mime = kind.mime_type().to_string();

        if mime == "image/gif" {
            return Err(ImagenValidationError::new("GIF no permitido"));
        }

        if !self.config.mimes_permitidos.contains(&mime) {
            return Err(ImagenValidationError::with_details(
                format!("MIME no soportado: '{}'", mime),
                [("permitidos", self.config.mimes_permitidos.join(", "))],
            ));
        }

        Ok(mime)
    }

    /// Verifica que el MIME detectado por contenido corresponda a la extensión declarada,
    /// para evitar archivos con extensión falsificada (ej: un `.png` que en realidad es otra cosa).
    fn validar_coherencia_extension_mime(
        &self,
        ext: &str,
        mime: &str,
    ) -> Result<(), ImagenValidationError> {
        let coherente = match ext {
            "jpg" | "jpeg" => mime == "image/jpeg" || mime == "image/jpg",
            "png" => mime == "image/png",
            "webp" => mime == "image/webp",
            _ => true, // extensión sin mapeo explícito: no se exige coherencia
        };

        if !coherente {
            return Err(ImagenValidationError::with_details(
                "La extensión del archivo no coincide con su contenido real",
                [
                    ("extension", ext.to_string()),
                    ("mime_detectado", mime.to_string()),
                ],
            ));
        }
        Ok(())
    }

    /// Verifica que el tamaño del archivo esté dentro del rango permitido. 
    fn validar_tamano(&self, size: usize, max_bytes: usize) -> Result<(), ImagenValidationError> {
        if size < self.config.min_bytes {
            return Err(ImagenValidationError::with_details(
                format!("Archivo muy pequeño: {}", formatear_bytes(size)),
                [("min", formatear_bytes(self.config.min_bytes))],
            ));
        }
        if size > max_bytes {
            return Err(ImagenValidationError::with_details(
                format!("Archivo muy grande: {}", formatear_bytes(size)),
                [("max", formatear_bytes(max_bytes))],
            ));
        }
        Ok(())
    }

    /// Identifica el formato real de la imagen a partir de su contenido (una sola vez;
    /// el resultado se reutiliza tanto para la detección de animación como para el decode).
    fn detectar_formato(&self, data: &[u8]) -> Result<ImageFormat, ImagenValidationError> {
        image::guess_format(data)
            .map_err(|_| ImagenValidationError::new("Formato de imagen no reconocido"))
    }

    /// Rechaza APNG/WebP animados. Siempre activo, no es configurable: ningún tipo de imagen
    /// animada está permitido. No decodifica los píxeles: solo inspecciona los metadatos de
    /// animación, así se corta temprano y barato antes del decode completo en
    /// `validar_integridad_profunda`.
    fn validar_animacion(
        &self,
        data: &[u8],
        formato: ImageFormat,
    ) -> Result<(), ImagenValidationError> {
        if self.es_imagen_animada(data, formato) {
            return Err(ImagenValidationError::new("Imágenes animadas no permitidas"));
        }

        Ok(())
    }

    /// Determina si los bytes proporcionados corresponden a una imagen con múltiples fotogramas/animación.
    fn es_imagen_animada(&self, data: &[u8], formato: ImageFormat) -> bool {
        let cursor = Cursor::new(data);

        match formato {
            ImageFormat::Png => PngDecoder::new(cursor)
                .and_then(|d| d.is_apng())
                .unwrap_or(false),

            ImageFormat::WebP => WebPDecoder::new(cursor)
                .map(|d| d.has_animation())
                .unwrap_or(false),

            _ => false,
        }
    }

    /// Decodifica la imagen completa para verificar integridad, píxeles totales y modo de color.
    fn validar_integridad_profunda(
        &self,
        data: &[u8],
        formato: ImageFormat,
        max_image_pixels: u64,
    ) -> Result<(u32, u32, String), ImagenValidationError> {
        let img = image::load_from_memory_with_format(data, formato).map_err(|e| match e {
            ImageError::IoError(_) => ImagenValidationError::new("Imagen corrupta o inválida"),
            ImageError::Unsupported(_) => {
                ImagenValidationError::new("Formato de imagen no soportado")
            }
            _ => ImagenValidationError::new(format!("Error decodificando imagen: {}", e)),
        })?;

        let (ancho, alto) = img.dimensions();
        let total_pixels = ancho as u64 * alto as u64;

        if total_pixels > max_image_pixels {
            return Err(ImagenValidationError::with_details(
                format!("Imagen excede el límite de píxeles: {}", total_pixels),
                [("max", max_image_pixels.to_string())],  // Se usa el parámetro, no self.config
            ));
        }

        let modo = match img.color() {
            image::ColorType::L8 | image::ColorType::L16 => "L",
            image::ColorType::La8 | image::ColorType::La16 => "LA",
            image::ColorType::Rgb8 | image::ColorType::Rgb16 | image::ColorType::Rgb32F => "RGB",
            image::ColorType::Rgba8 | image::ColorType::Rgba16 | image::ColorType::Rgba32F => {
                "RGBA"
            }
            _ => "Other",
        }
        .to_string();

        Ok((ancho, alto, modo))
    }

    /// Lógica común de validación, parametrizada por el límite máximo de tamaño (`max_bytes`).
    ///
    /// Ejecuta en orden: extensión → MIME → coherencia extensión/MIME → tamaño →
    /// animación → integridad/decode (incluye límite de píxeles).
    /// Las comprobaciones más baratas van primero para descartar archivos inválidos
    /// sin pagar el costo del decode completo.
    fn validar_con_limite(
        &self,
        data: &[u8],
        filename: &str,
        max_bytes: usize,
        max_image_pixels: u64,
    ) -> Result<ImagenMetadata, ImagenValidationError> {
        if data.is_empty() {
            return Err(ImagenValidationError::new("Archivo no proporcionado"));
        }

        let ext = self.validar_nombre_extension(filename)?;
        let mime = self.validar_mime_rapido(&mut Cursor::new(data))?;

        if self.config.validar_coherencia_extension_mime {
            self.validar_coherencia_extension_mime(&ext, &mime)?;
        }

        self.validar_tamano(data.len(), max_bytes)?;

        let formato = self.detectar_formato(data)?;
        self.validar_animacion(data, formato)?;

        let (ancho, alto, modo) =
            self.validar_integridad_profunda(data, formato, max_image_pixels)?;

        Ok(ImagenMetadata::new(
            ancho,
            alto,
            mime,
            ext,
            data.len(),
            format!("{:?}", formato),
            modo,
        ))
    }

    /// Valida una imagen usando los límites por defecto de la configuración
    pub fn validar_20mb(
        &self,
        data: &[u8],
        filename: &str,
    ) -> Result<ImagenMetadata, ImagenValidationError> {
        self.validar_con_limite(
            data,
            filename,
            self.config.max_bytes,
            self.config.max_image_pixels,
        )
    }

    /// Valida con límite estricto de 10 MB y 20 MP.
    pub fn validar_10mb(
        &self,
        data: &[u8],
        filename: &str,
    ) -> Result<ImagenMetadata, ImagenValidationError> {
        self.validar_con_limite(
            data,
            filename,
            ImagenConfig::LIMITE_10MB_BYTES,
            ImagenConfig::LIMITE_10MB_PIXELS,
        )
    }

    /// Valida con límite estricto de 5 MB y 15 MP.
    pub fn validar_5mb(
        &self,
        data: &[u8],
        filename: &str,
    ) -> Result<ImagenMetadata, ImagenValidationError> {
        self.validar_con_limite(
            data,
            filename,
            ImagenConfig::LIMITE_5MB_BYTES,
            ImagenConfig::LIMITE_5MB_PIXELS,
        )
    }
}



#[cfg(test)]
mod tests {
    use crate::core::validador::ImagenValidator;

    #[test]
    fn validar_imagen_e_imprimir_resultado() {
        let validador = ImagenValidator::default();
        let ruta_imagen = "static/portada/gur.png";

        match validador.validar_desde_ruta(ruta_imagen) {
            Ok(meta) => {
                println!("✅ Imagen válida: {}x{}", meta.width(), meta.height());
                println!("📋 Metadatos: {:#?}", meta);

                // Validación básica para el test
                assert!(meta.width() > 0);
                assert!(meta.height() > 0);
            }
            Err(e) => {
                panic!("❌ Error de validación: {}", e);
            }
        }
    }
}

use std::fs;

impl ImagenValidator {
    /// Lee el archivo de disco y delega en `validar_bytes`. Maneja errores de I/O y permisos.
    pub fn validar_desde_ruta(&self, ruta: &str) -> Result<ImagenMetadata, ImagenValidationError> {
        let path = Path::new(ruta);

        if !path.exists() {
            return Err(ImagenValidationError::new(format!(
                "Archivo no existe: {}",
                ruta
            )));
        }
        if !path.is_file() {
            return Err(ImagenValidationError::new(format!(
                "No es un archivo regular: {}",
                ruta
            )));
        }

        let data = fs::read(path).map_err(|e| match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                ImagenValidationError::new(format!("Sin permisos de lectura: {}", ruta))
            }
            _ => ImagenValidationError::new(format!("Error leyendo archivo: {}", e)),
        })?;

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();

        self.validar_5mb(&data, filename)
    }
}

pub fn eliminar_exif(
    data: &[u8],
    formato: ImageFormat,
) -> Result<Vec<u8>, image::ImageError> {
    let imagen = image::load_from_memory_with_format(data, formato)?;

    let mut buffer = Cursor::new(Vec::new());
    imagen.write_to(&mut buffer, formato)?;

    Ok(buffer.into_inner())
}

