pub mod validador;
pub use validador::{ImagenConfig, ImagenMetadata, ImagenValidationError, ImagenValidator};

pub mod nombre_archivo;
pub use nombre_archivo::{NombreArchivo, NombreArchivoError};

pub mod ruta;
pub use ruta::{ConfigRuta, Ruta, ErrorRuta, ConfigRutaError};