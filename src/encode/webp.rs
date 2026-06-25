use super::constants::*;
use super::grayscale::ImageTone;
use crate::error::{ClioError, Result};

pub fn auto_tune_webp(img: &image::DynamicImage, tone: ImageTone) -> f32 {
    let pixels = img.width() as u64 * img.height() as u64;

    let mut quality = if pixels < WEBP_TINY_THRESHOLD {
        WEBP_QUALITY_TINY
    } else if pixels < WEBP_SMALL_THRESHOLD {
        WEBP_QUALITY_SMALL
    } else if pixels < WEBP_MEDIUM_THRESHOLD {
        WEBP_QUALITY_MEDIUM
    } else if pixels < WEBP_LARGE_THRESHOLD {
        WEBP_QUALITY_LARGE
    } else {
        WEBP_QUALITY_HUGE
    };

    if tone != ImageTone::Color {
        let reduction = if tone == ImageTone::LineArt {
            WEBP_GRAYSCALE_QUALITY_REDUCTION + 2.0
        } else {
            WEBP_GRAYSCALE_QUALITY_REDUCTION
        };
        quality = (quality - reduction).max(60.0);
    }

    quality
}

pub fn convert_to_webp(img: &image::DynamicImage, tone: ImageTone) -> Result<Vec<u8>> {
    use ::webp::{Encoder, WebPConfig};

    let quality = auto_tune_webp(img, tone);
    let (w, h) = (img.width(), img.height());

    let mut config = WebPConfig::new().unwrap();
    config.quality = quality;
    config.method = 6;
    config.thread_level = 0;

    let encoded = if img.color().has_alpha() {
        let rgba = img.to_rgba8();
        Encoder::from_rgba(&rgba, w, h)
            .encode_advanced(&config)
            .map_err(|e| ClioError::webp_encode(format!("{e:?}")))?
    } else {
        let rgb_cow = if let Some(r) = img.as_rgb8() {
            std::borrow::Cow::Borrowed(r)
        } else {
            std::borrow::Cow::Owned(img.to_rgb8())
        };
        Encoder::from_rgb(&rgb_cow, w, h)
            .encode_advanced(&config)
            .map_err(|e| ClioError::webp_encode(format!("{e:?}")))?
    };

    Ok(encoded.to_vec())
}
