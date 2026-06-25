use crate::error::{ClioError, Result};
use ttf2woff2::{BrotliQuality, encode};

/// Converts a TTF or OTF font to WOFF2.
/// Returns the WOFF2 bytes and the new filename (.woff2 extension).
pub fn convert_to_woff2(name: &str, data: &[u8]) -> Result<(String, Vec<u8>)> {
    let woff2 = encode(data, BrotliQuality::default())
        .map_err(|e| ClioError::html(format!("woff2 encode: {e}")))?;
    let new_name = swap_font_ext(name);
    Ok((new_name, woff2))
}

fn swap_font_ext(name: &str) -> String {
    match name.rfind('.') {
        Some(i) => format!("{}.woff2", &name[..i]),
        None => format!("{name}.woff2"),
    }
}
