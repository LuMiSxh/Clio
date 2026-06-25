use super::constants::*;
use super::grayscale::ImageTone;
use crate::error::{ClioError, Result};

pub fn auto_tune_avif(img: &image::DynamicImage, tone: ImageTone) -> (f32, u8) {
    let pixels = img.width() as u64 * img.height() as u64;

    let (mut quality, mut speed) = if pixels < AVIF_TINY_THRESHOLD {
        (AVIF_QUALITY_TINY, AVIF_SPEED_TINY)
    } else if pixels < AVIF_SMALL_THRESHOLD {
        (AVIF_QUALITY_SMALL, AVIF_SPEED_SMALL)
    } else if pixels < AVIF_MEDIUM_THRESHOLD {
        (AVIF_QUALITY_MEDIUM, AVIF_SPEED_MEDIUM)
    } else if pixels < AVIF_LARGE_THRESHOLD {
        (AVIF_QUALITY_LARGE, AVIF_SPEED_LARGE)
    } else {
        (AVIF_QUALITY_HUGE, AVIF_SPEED_HUGE)
    };

    if tone != ImageTone::Color {
        speed = speed.saturating_sub(1);
        let reduction = if tone == ImageTone::LineArt {
            AVIF_GRAYSCALE_QUALITY_REDUCTION + 3.0
        } else {
            AVIF_GRAYSCALE_QUALITY_REDUCTION
        };
        quality = (quality - reduction).max(55.0);
    }

    (quality, speed)
}

pub fn convert_to_avif(img: &image::DynamicImage, tone: ImageTone) -> Result<Vec<u8>> {
    let width = img.width() as usize;
    let height = img.height() as usize;
    let (quality, speed) = auto_tune_avif(img, tone);

    // single thread per encode; rayon handles cross-image parallelism
    let encoder = ravif::Encoder::new()
        .with_quality(quality)
        .with_speed(speed)
        .with_alpha_quality(AVIF_ALPHA_QUALITY)
        .with_num_threads(Some(1));

    if tone != ImageTone::Color {
        encode_gray(&encoder, img, width, height)
    } else {
        encode_color(&encoder, img, width, height)
    }
}

fn encode_gray(
    encoder: &ravif::Encoder,
    img: &image::DynamicImage,
    width: usize,
    height: usize,
) -> Result<Vec<u8>> {
    if let Some(rgb) = img.as_rgb8() {
        let planes = rgb.as_raw().chunks_exact(3).map(|p| [p[0], 128u8, 128u8]);
        return encoder
            .encode_raw_planes_8_bit(
                width,
                height,
                planes,
                None::<[u8; 0]>,
                rav1e::prelude::PixelRange::Full,
                ravif::MatrixCoefficients::BT601,
            )
            .map(|e| e.avif_file)
            .map_err(ClioError::avif_encode);
    }

    if let Some(rgba) = img.as_rgba8() {
        let planes = rgba.as_raw().chunks_exact(4).map(|p| [p[0], 128u8, 128u8]);
        return encoder
            .encode_raw_planes_8_bit(
                width,
                height,
                planes,
                None::<[u8; 0]>,
                rav1e::prelude::PixelRange::Full,
                ravif::MatrixCoefficients::BT601,
            )
            .map(|e| e.avif_file)
            .map_err(ClioError::avif_encode);
    }

    let luma = img.to_luma8();
    let planes = luma.as_raw().iter().map(|&y| [y, 128u8, 128u8]);
    encoder
        .encode_raw_planes_8_bit(
            width,
            height,
            planes,
            None::<[u8; 0]>,
            rav1e::prelude::PixelRange::Full,
            ravif::MatrixCoefficients::BT601,
        )
        .map(|e| e.avif_file)
        .map_err(ClioError::avif_encode)
}

fn encode_color(
    encoder: &ravif::Encoder,
    img: &image::DynamicImage,
    width: usize,
    height: usize,
) -> Result<Vec<u8>> {
    use ravif::{Img, RGB8};

    let rgb_cow = if let Some(r) = img.as_rgb8() {
        std::borrow::Cow::Borrowed(r)
    } else {
        std::borrow::Cow::Owned(img.to_rgb8())
    };

    let rgb_slice: &[RGB8] = unsafe {
        std::slice::from_raw_parts(rgb_cow.as_raw().as_ptr() as *const RGB8, width * height)
    };

    encoder
        .encode_rgb(Img::new(rgb_slice, width, height))
        .map(|e| e.avif_file)
        .map_err(ClioError::avif_encode)
}
