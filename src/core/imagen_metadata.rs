// ================= ERRORES DE VALIDACIÓN =================
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum MetadataError {
    #[error("Aspect ratio no válido para '{tipo}': se esperaba {esperado:.2} (±{tolerancia_pct:.0}%), pero se obtuvo {actual:.2}")]
    AspectRatioInvalido {
        tipo: &'static str,
        actual: f64,
        esperado: f64,
        tolerancia_pct: f64,
    }
}

// ================= VALUE OBJECT: IMAGENMETADATA =================

#[derive(Debug, PartialEq)]
pub struct ImagenMetadata {
    width: u32,
    height: u32,
    mime: String,
    extension: String,
    bytes: usize,
    size_formatted: String,
    format: String,
    mode: String,
    aspect_ratio: f64,
    total_pixels: u64,
    megapixels: f64,
}

impl ImagenMetadata {
    pub const ASPECT_RATIO_FOTO: f64 = 1.0;
    pub const ASPECT_RATIO_BANNER: f64 = 3.0;
    pub const TOLERANCIA_ASPECT_RATIO: f64 = 0.10;
}

impl ImagenMetadata {
    pub fn new(
        width: u32,
        height: u32,
        mime: String,
        extension: String,
        bytes: usize,
        format: String,
        mode: String,
    ) -> Self {
        let total_pixels = (width as u64) * (height as u64);
        let megapixels = (total_pixels as f64 / 1_000_000.0 * 10.0).round() / 10.0;
        
         let aspect_ratio = if height > 0 {
            (width as f64 / height as f64 * 100.0).round() / 100.0
        } else {
            0.0
        };

        let size_formatted = Self::formatear_tamano(bytes);

        Self {
            width,
            height,
            mime,
            extension,
            bytes,
            size_formatted,
            format,
            mode,
            aspect_ratio,
            total_pixels,
            megapixels,
        }
    }

    /// Método privado para convertir bytes en una representación legible (B, KB, MB).
    fn formatear_tamano(bytes: usize) -> String {
        const KB: f64 = 1024.0;
        const MB: f64 = KB * 1024.0;

        let bytes_f = bytes as f64;
        if bytes_f >= MB {
            format!("{:.2} MB", bytes_f / MB)
        } else if bytes_f >= KB {
            format!("{:.2} KB", bytes_f / KB)
        } else {
            format!("{} B", bytes)
        }
    }

    // ---------- Getters ----------

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn mime(&self) -> &str {
        &self.mime
    }

    pub fn extension(&self) -> &str {
        &self.extension
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn size_formatted(&self) -> &str {
        &self.size_formatted
    }

    pub fn format(&self) -> &str {
        &self.format
    }

    pub fn mode(&self) -> &str {
        &self.mode
    }

    pub fn aspect_ratio(&self) -> f64 {
        self.aspect_ratio
    }

    pub fn total_pixels(&self) -> u64 {
        self.total_pixels
    }

    pub fn megapixels(&self) -> f64 {
        self.megapixels
    }

    // ---------- Validaciones Semánticas ----------

    fn aspect_ratio_cumple(&self, objetivo: f64, tolerancia: f64) -> bool {
        let min = objetivo * (1.0 - tolerancia);
        let max = objetivo * (1.0 + tolerancia);
        self.aspect_ratio >= min && self.aspect_ratio <= max
    }

    /// Valida que la imagen cumpla con el aspect ratio de una **foto** (1:1 ±10%).
    pub fn validar_foto(&self) -> Result<(), MetadataError> {
        if self.aspect_ratio_cumple(Self::ASPECT_RATIO_FOTO, Self::TOLERANCIA_ASPECT_RATIO) {
            Ok(())
        } else {
            Err(MetadataError::AspectRatioInvalido {
                tipo: "Foto de perfil",
                actual: self.aspect_ratio,
                esperado: Self::ASPECT_RATIO_FOTO,
                tolerancia_pct: Self::TOLERANCIA_ASPECT_RATIO * 100.0,
            })
        }
    }

    /// Valida que la imagen cumpla con el aspect ratio de un **banner** (3:1 ±10%).
    pub fn validar_banner(&self) -> Result<(), MetadataError> {
        if self.aspect_ratio_cumple(Self::ASPECT_RATIO_BANNER, Self::TOLERANCIA_ASPECT_RATIO) {
            Ok(())
        } else {
            Err(MetadataError::AspectRatioInvalido {
                tipo: "Banner horizontal",
                actual: self.aspect_ratio,
                esperado: Self::ASPECT_RATIO_BANNER,
                tolerancia_pct: Self::TOLERANCIA_ASPECT_RATIO * 100.0,
            })
        }
    }
}