use thiserror::Error;

pub type Result<T> = std::result::Result<T, ClioError>;

#[derive(Debug, Error)]
pub enum ClioError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),
    #[error("Image decode failed: {0}")]
    ImageDecode(String),
    #[error("AVIF encode failed: {0}")]
    AvifEncode(String),
    #[error("WebP encode failed: {0}")]
    WebpEncode(String),
    #[error("HTML processing failed: {0}")]
    Html(String),
}

impl ClioError {
    pub fn avif_encode(e: impl std::fmt::Display) -> Self {
        Self::AvifEncode(e.to_string())
    }
    pub fn webp_encode(e: impl std::fmt::Display) -> Self {
        Self::WebpEncode(e.to_string())
    }
    pub fn html(e: impl std::fmt::Display) -> Self {
        Self::Html(e.to_string())
    }
}
