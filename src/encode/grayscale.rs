use image::{DynamicImage, GenericImageView};

const SAMPLE_COUNT: u64 = 1024;
const RGB_THRESHOLD: u8 = 3;
const GRAY_THRESHOLD: f32 = 0.95;
const LINE_ART_GRAY_THRESHOLD: f32 = 0.98;
const LINE_ART_BW_THRESHOLD: f32 = 0.88;
const DARK_LUMA_THRESHOLD: u8 = 24;
const LIGHT_LUMA_THRESHOLD: u8 = 232;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageTone {
    Color,
    Grayscale,
    LineArt,
}

#[inline(always)]
fn is_gray_pixel(r: u8, g: u8, b: u8) -> bool {
    r.abs_diff(g) <= RGB_THRESHOLD
        && g.abs_diff(b) <= RGB_THRESHOLD
        && b.abs_diff(r) <= RGB_THRESHOLD
}

#[inline(always)]
fn luma_approx(r: u8, g: u8, b: u8) -> u8 {
    ((r as u16 * 77 + g as u16 * 150 + b as u16 * 29) >> 8) as u8
}

#[inline(always)]
fn is_bw_line_pixel(r: u8, g: u8, b: u8) -> bool {
    let y = luma_approx(r, g, b);
    y <= DARK_LUMA_THRESHOLD || y >= LIGHT_LUMA_THRESHOLD
}

#[derive(Default)]
struct ToneSample {
    gray_count: u32,
    bw_count: u32,
    total: u32,
}

impl ToneSample {
    fn push(&mut self, r: u8, g: u8, b: u8) {
        if is_gray_pixel(r, g, b) {
            self.gray_count += 1;
            if is_bw_line_pixel(r, g, b) {
                self.bw_count += 1;
            }
        }
        self.total += 1;
    }

    fn classify(&self) -> ImageTone {
        if self.total == 0 {
            return ImageTone::Color;
        }
        let gray_ratio = self.gray_count as f32 / self.total as f32;
        let bw_ratio = self.bw_count as f32 / self.total as f32;
        if gray_ratio > LINE_ART_GRAY_THRESHOLD && bw_ratio > LINE_ART_BW_THRESHOLD {
            ImageTone::LineArt
        } else if gray_ratio > GRAY_THRESHOLD {
            ImageTone::Grayscale
        } else {
            ImageTone::Color
        }
    }
}

pub fn classify_image_tone(img: &DynamicImage) -> ImageTone {
    if matches!(
        img,
        DynamicImage::ImageLuma8(_) | DynamicImage::ImageLuma16(_)
    ) {
        return classify_luma_image(img);
    }

    let width = img.width() as u64;
    let height = img.height() as u64;
    let pixel_count = width * height;
    if pixel_count == 0 {
        return ImageTone::Color;
    }
    let step = (pixel_count / SAMPLE_COUNT).max(1);

    if let Some(rgb) = img.as_rgb8() {
        let raw = rgb.as_raw();
        let mut sample = ToneSample::default();
        let mut i = 0u64;
        while i < pixel_count {
            let base = (i as usize) * 3;
            sample.push(raw[base], raw[base + 1], raw[base + 2]);
            i += step;
        }
        return sample.classify();
    }

    if let Some(rgba) = img.as_rgba8() {
        let raw = rgba.as_raw();
        let mut sample = ToneSample::default();
        let mut i = 0u64;
        while i < pixel_count {
            let base = (i as usize) * 4;
            sample.push(raw[base], raw[base + 1], raw[base + 2]);
            i += step;
        }
        return sample.classify();
    }

    let mut sample = ToneSample::default();
    let mut i = 0u64;
    while i < pixel_count {
        let x = (i % width) as u32;
        let y = (i / width) as u32;
        let channels = img.get_pixel(x, y).0;
        if channels.len() >= 3 {
            sample.push(channels[0], channels[1], channels[2]);
        }
        i += step;
    }
    sample.classify()
}

fn classify_luma_image(img: &DynamicImage) -> ImageTone {
    let width = img.width() as u64;
    let height = img.height() as u64;
    let pixel_count = width * height;
    if pixel_count == 0 {
        return ImageTone::Color;
    }
    let step = (pixel_count / SAMPLE_COUNT).max(1);

    if let Some(luma) = img.as_luma8() {
        let raw = luma.as_raw();
        let mut bw_count = 0u32;
        let mut total = 0u32;
        let mut i = 0u64;
        while i < pixel_count {
            let y = raw[i as usize];
            if y <= DARK_LUMA_THRESHOLD || y >= LIGHT_LUMA_THRESHOLD {
                bw_count += 1;
            }
            total += 1;
            i += step;
        }
        return if total > 0 && (bw_count as f32 / total as f32) > LINE_ART_BW_THRESHOLD {
            ImageTone::LineArt
        } else {
            ImageTone::Grayscale
        };
    }

    ImageTone::Grayscale
}
