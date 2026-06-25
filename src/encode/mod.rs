pub mod avif;
mod constants;
pub mod font;
pub mod grayscale;
pub mod webp;

use crate::error::{ClioError, Result};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Avif,
    Webp,
}

impl Format {
    pub fn ext(self) -> &'static str {
        match self {
            Self::Avif => "avif",
            Self::Webp => "webp",
        }
    }
}

pub fn process_image(name: &str, data: &[u8], format: Format) -> Result<(String, Vec<u8>)> {
    let img = image::load_from_memory(data).map_err(|e| ClioError::ImageDecode(e.to_string()))?;
    let tone = grayscale::classify_image_tone(&img);
    let encoded = match format {
        Format::Avif => avif::convert_to_avif(&img, tone)?,
        Format::Webp => webp::convert_to_webp(&img, tone)?,
    };
    Ok((swap_ext(name, format.ext()), encoded))
}

pub(crate) fn swap_ext(name: &str, ext: &str) -> String {
    match name.rfind('.') {
        Some(i) => format!("{}.{ext}", &name[..i]),
        None => format!("{name}.{ext}"),
    }
}
