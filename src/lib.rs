pub mod core;

pub mod validador {
    pub use super::core::{ImagenConfig, ImagenMetadata, ImagenValidationError, ImagenValidator};
}

pub mod vo {
    pub use super::core::{NombreArchivo, NombreArchivoError, ConfigRuta, Ruta, ErrorRuta, ConfigRutaError};
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::BufReader;
    use std::path::Path;
    use image::GenericImageView;

    #[test]
    fn extraer_y_mostrar_informacion_imagen() {
        // Cambia esta ruta por la de tu imagen de prueba
        let ruta_imagen = Path::new("static/uploads/imagenes/20260904_101816.jpg");

        if !ruta_imagen.exists() {
            println!("❌ El archivo en la ruta {:?} no existe.", ruta_imagen);
            return;
        }

        println!("\n==================================================");
        println!("  1. METADATOS DEL SISTEMA DE ARCHIVOS");
        println!("==================================================");
        if let Ok(meta) = std::fs::metadata(ruta_imagen) {
            println!("Ruta: {:?}", ruta_imagen);
            println!("Tamaño: {} bytes ({:.2} MB)", meta.len(), meta.len() as f64 / (1024.0 * 1024.0));
            println!("Es archivo estándar: {}", meta.is_file());
            if let Ok(creado) = meta.created() {
                println!("Fecha creación (SO): {:?}", creado);
            }
            if let Ok(modificado) = meta.modified() {
                println!("Última modificación (SO): {:?}", modificado);
            }
        }

        println!("\n==================================================");
        println!("  2. INFORMACIÓN ESTRUCTURAL Y DIMENSIONES");
        println!("==================================================");
        if let Ok(formato) = image::ImageFormat::from_path(ruta_imagen) {
            println!("Formato detectado: {:?}", formato);
        }

        match image::open(ruta_imagen) {
            Ok(img) => {
                let (ancho, alto) = img.dimensions();
                println!("Ancho: {} px", ancho);
                println!("Alto: {} px", alto);
                println!("Relación de aspecto: {:.2}", ancho as f32 / alto as f32);
                println!("Espacio de color: {:?}", img.color());
                println!("Bytes por píxel aprox: {}", img.color().bytes_per_pixel());
            }
            Err(err) => println!("Error al decodificar la imagen: {}", err),
        }

        println!("\n==================================================");
        println!("  3. METADATOS EXIF (CÁMARA, CONFIGURACIÓN, GPS)");
        println!("==================================================");
        match File::open(ruta_imagen) {
            Ok(file) => {
                let mut buf_reader = BufReader::new(file);
                let exif_reader = exif::Reader::new();

                match exif_reader.read_from_container(&mut buf_reader) {
                    Ok(exif_data) => {
                        for field in exif_data.fields() {
                            println!(
                                "  • {:<28} : {}",
                                field.tag.to_string(),
                                field.display_value().with_unit(&exif_data)
                            );
                        }
                    }
                    Err(err) => println!("Sin datos EXIF o formato no soportado: {}", err),
                }
            }
            Err(err) => println!("Error al abrir archivo para lectura EXIF: {}", err),
        }
        println!("==================================================\n");
    }
}