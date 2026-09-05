pub mod validador;
pub use validador::{ImagenConfig, ImagenValidationError, ImagenValidator};

pub mod nombre_archivo;
pub use nombre_archivo::{NombreArchivo, NombreArchivoError};

pub mod ruta;
pub use ruta::{ConfigRuta, Ruta, ErrorRuta, ConfigRutaError};

pub mod imagen_metadata;
pub use imagen_metadata::{ImagenMetadata, MetadataError};