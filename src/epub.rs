use crate::error::Result;
use std::io::{Read, Write};
use std::path::Path;
use zip::{CompressionMethod, ZipArchive, ZipWriter, write::FileOptions};

pub struct EpubEntry {
    pub name: String,
    pub data: Vec<u8>,
}

pub struct EpubAssets {
    pub images: Vec<EpubEntry>,
    pub html: Vec<EpubEntry>,
    pub css: Vec<EpubEntry>,
    pub opf: Vec<EpubEntry>,
    pub fonts: Vec<EpubEntry>,
    pub other: Vec<EpubEntry>,
}

fn ext(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or("")
}

pub fn open_epub(path: &Path) -> Result<EpubAssets> {
    let file = std::fs::File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut assets = EpubAssets {
        images: vec![],
        html: vec![],
        css: vec![],
        opf: vec![],
        fonts: vec![],
        other: vec![],
    };

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_owned();
        let mut data = Vec::with_capacity(entry.size() as usize);
        entry.read_to_end(&mut data)?;

        let lower = name.to_lowercase();
        let e = ext(&lower);
        let epub_entry = EpubEntry { name, data };

        match e {
            "jpg" | "jpeg" | "png" | "gif" | "webp" | "avif" => assets.images.push(epub_entry),
            "html" | "xhtml" => assets.html.push(epub_entry),
            "css" => assets.css.push(epub_entry),
            "opf" => assets.opf.push(epub_entry),
            "ttf" | "otf" => assets.fonts.push(epub_entry),
            _ => assets.other.push(epub_entry),
        }
    }

    Ok(assets)
}

pub fn repack_epub(assets: &EpubAssets, output: &Path) -> Result<()> {
    let file = std::fs::File::create(output)?;
    let mut zip = ZipWriter::new(file);

    let stored: FileOptions<()> =
        FileOptions::default().compression_method(CompressionMethod::Stored);
    let deflated: FileOptions<()> = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(9));

    // mimetype must be first and stored — EPUB spec requirement
    zip.start_file("mimetype", stored)?;
    zip.write_all(b"application/epub+zip")?;

    for entry in assets.images.iter().chain(assets.fonts.iter()) {
        zip.start_file(&entry.name, stored)?;
        zip.write_all(&entry.data)?;
    }

    for entry in assets
        .html
        .iter()
        .chain(assets.css.iter())
        .chain(assets.opf.iter())
        .chain(assets.other.iter())
    {
        if entry.name == "mimetype" {
            continue;
        }
        // `other` may hold already-compressed assets (woff2, audio, video) that don't
        // benefit from DEFLATE; storing them saves CPU at no size cost.
        let opts = if is_precompressed(&entry.name) {
            stored
        } else {
            deflated
        };
        zip.start_file(&entry.name, opts)?;
        zip.write_all(&entry.data)?;
    }

    zip.finish()?;
    Ok(())
}

fn is_precompressed(name: &str) -> bool {
    matches!(
        ext(&name.to_lowercase()),
        "woff" | "woff2" | "mp3" | "mp4" | "m4a" | "aac" | "ogg" | "opus" | "webm" | "mov"
    )
}
