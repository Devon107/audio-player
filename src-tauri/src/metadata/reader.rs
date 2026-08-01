use std::borrow::Cow;
use std::path::Path;

use lofty::file::{AudioFile, TaggedFileExt};
use lofty::picture::{MimeType, PictureType};
use lofty::tag::{Accessor, Tag};
use serde::Serialize;

/// Metadatos principales de una pista, listos para enviar al frontend.
#[derive(Debug, Clone, Serialize)]
pub struct TrackMetadata {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub year: Option<u16>,
    pub track_number: Option<u32>,
    pub duration_secs: f64,
    pub has_cover_art: bool,
}

/// Lee los metadatos principales de un archivo de audio. Si el archivo no tiene etiquetas, o
/// están corruptas, se devuelven valores por defecto sensatos (p. ej. el título cae al nombre
/// del archivo) en lugar de fallar.
pub fn read_metadata(path: &Path) -> Result<TrackMetadata, String> {
    let tagged_file = lofty::read_from_path(path).map_err(|e| {
        format!(
            "No se pudieron leer los metadatos de '{}': {e}",
            path.display()
        )
    })?;

    let tag = primary_tag(&tagged_file);

    let title = tag
        .and_then(Accessor::title)
        .map(Cow::into_owned)
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| fallback_title(path));

    Ok(TrackMetadata {
        title,
        artist: non_empty(tag.and_then(Accessor::artist)),
        album: non_empty(tag.and_then(Accessor::album)),
        genre: non_empty(tag.and_then(Accessor::genre)),
        year: tag.and_then(Accessor::date).map(|date| date.year),
        track_number: tag.and_then(Accessor::track),
        duration_secs: tagged_file.properties().duration().as_secs_f64(),
        has_cover_art: tag.is_some_and(|t| !t.pictures().is_empty()),
    })
}

/// Extrae los bytes de la carátula embebida (portada frontal si hay varias) junto con la
/// extensión de archivo apropiada para su tipo MIME. Devuelve `None` si el archivo no tiene
/// ninguna imagen embebida.
pub fn extract_cover_bytes(path: &Path) -> Result<Option<(Vec<u8>, &'static str)>, String> {
    let tagged_file = lofty::read_from_path(path).map_err(|e| {
        format!(
            "No se pudieron leer los metadatos de '{}': {e}",
            path.display()
        )
    })?;

    let Some(tag) = primary_tag(&tagged_file) else {
        return Ok(None);
    };

    let picture = tag
        .pictures()
        .iter()
        .find(|p| p.pic_type() == PictureType::CoverFront)
        .or_else(|| tag.pictures().first());

    Ok(picture.map(|p| (p.data().to_vec(), extension_for(p.mime_type()))))
}

fn primary_tag(tagged_file: &lofty::file::TaggedFile) -> Option<&Tag> {
    tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())
}

fn non_empty(value: Option<Cow<'_, str>>) -> Option<String> {
    value.map(Cow::into_owned).filter(|v| !v.trim().is_empty())
}

fn fallback_title(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn extension_for(mime: Option<&MimeType>) -> &'static str {
    match mime {
        Some(MimeType::Png) => "png",
        Some(MimeType::Jpeg) => "jpg",
        Some(MimeType::Tiff) => "tiff",
        Some(MimeType::Bmp) => "bmp",
        Some(MimeType::Gif) => "gif",
        _ => "bin",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::PathBuf::from(format!(
            "{}/tests/fixtures/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
    }

    #[test]
    fn reads_full_metadata_with_cover() {
        let meta = read_metadata(&fixture("test-tone-with-cover.mp3"))
            .expect("debería leer los metadatos");

        assert_eq!(meta.title, "Con Caratula");
        assert_eq!(meta.artist.as_deref(), Some("Artista Prueba"));
        assert_eq!(meta.album.as_deref(), Some("Album Prueba"));
        assert_eq!(meta.genre.as_deref(), Some("Electronic"));
        assert_eq!(meta.year, Some(2024));
        assert_eq!(meta.track_number, Some(3));
        assert!(meta.has_cover_art);
        assert!((meta.duration_secs - 3.0).abs() < 0.5);
    }

    #[test]
    fn falls_back_to_filename_when_no_tags() {
        let path = fixture("test-tone-no-tags.mp3");
        let meta = read_metadata(&path).expect("debería leer el archivo sin etiquetas");

        assert_eq!(meta.title, "test-tone-no-tags");
        assert_eq!(meta.artist, None);
        assert_eq!(meta.album, None);
        assert_eq!(meta.genre, None);
        assert_eq!(meta.year, None);
        assert!(!meta.has_cover_art);
        assert!((meta.duration_secs - 2.0).abs() < 0.5);
    }

    #[test]
    fn extracts_embedded_cover_bytes() {
        let cover = extract_cover_bytes(&fixture("test-tone-with-cover.mp3"))
            .expect("no debería fallar")
            .expect("debería encontrar una carátula");

        let (bytes, extension) = cover;
        assert!(!bytes.is_empty());
        assert_eq!(extension, "png");
    }

    #[test]
    fn returns_none_when_no_cover_present() {
        let cover =
            extract_cover_bytes(&fixture("test-tone-no-tags.mp3")).expect("no debería fallar");
        assert!(cover.is_none());
    }

    #[test]
    fn rejects_missing_file() {
        assert!(read_metadata(std::path::Path::new("/no/existe/archivo.mp3")).is_err());
    }
}
