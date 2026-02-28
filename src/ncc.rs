//! Normalized Cross-Correlation (NCC) template matching for watermark detection
//! Used to verify watermark exists before attempting removal

use image::{GrayImage, Luma};

#[derive(Debug, Clone)]
pub struct NccConfig {
    pub threshold: f32,
    pub search_scale: f32,
}

impl Default for NccConfig {
    fn default() -> Self {
        Self {
            threshold: 0.7,
            search_scale: 1.0,
        }
    }
}

#[derive(Debug)]
pub struct NccResult {
    pub found: bool,
    pub confidence: f32,
    pub position: Option<(u32, u32)>,
}

pub fn detect_watermark(image: &GrayImage, template: &GrayImage, config: &NccConfig) -> NccResult {
    let (img_width, img_height) = image.dimensions();
    let (tpl_width, tpl_height) = template.dimensions();

    if tpl_width > img_width || tpl_height > img_height {
        return NccResult {
            found: false,
            confidence: 0.0,
            position: None,
        };
    }

    let max_x = img_width - tpl_width + 1;
    let max_y = img_height - tpl_height + 1;

    let mut best_score: f32 = 0.0;
    let mut best_position: Option<(u32, u32)> = None;

    for y in 0..max_y {
        for x in 0..max_x {
            let score = calculate_ncc(image, template, x, y, tpl_width, tpl_height);

            if score > best_score {
                best_score = score;
                best_position = Some((x, y));
            }
        }
    }

    let found = best_score >= config.threshold;
    let confidence = best_score;

    NccResult {
        found,
        confidence,
        position: best_position,
    }
}

fn calculate_ncc(
    image: &GrayImage,
    template: &GrayImage,
    offset_x: u32,
    offset_y: u32,
    tpl_width: u32,
    tpl_height: u32,
) -> f32 {
    let mut mean_img: f64 = 0.0;
    let mut mean_tpl: f64 = 0.0;
    let n = (tpl_width * tpl_height) as f64;

    for dy in 0..tpl_height {
        for dx in 0..tpl_width {
            let img_pixel = image.get_pixel(offset_x + dx, offset_y + dy)[0] as f64;
            let tpl_pixel = template.get_pixel(dx, dy)[0] as f64;
            mean_img += img_pixel;
            mean_tpl += tpl_pixel;
        }
    }

    mean_img /= n;
    mean_tpl /= n;

    let mut numerator: f64 = 0.0;
    let mut denom_img: f64 = 0.0;
    let mut denom_tpl: f64 = 0.0;

    for dy in 0..tpl_height {
        for dx in 0..tpl_width {
            let img_pixel = image.get_pixel(offset_x + dx, offset_y + dy)[0] as f64;
            let tpl_pixel = template.get_pixel(dx, dy)[0] as f64;

            let diff_img = img_pixel - mean_img;
            let diff_tpl = tpl_pixel - mean_tpl;

            numerator += diff_img * diff_tpl;
            denom_img += diff_img * diff_img;
            denom_tpl += diff_tpl * diff_tpl;
        }
    }

    let denom = (denom_img * denom_tpl).sqrt();

    if denom < 1e-10 {
        0.0
    } else {
        (numerator / denom) as f32
    }
}

pub fn create_template_from_alpha_map(alpha_map: &[f32], width: u32, height: u32) -> GrayImage {
    let mut template = GrayImage::new(width, height);

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            let alpha = alpha_map[idx];
            let gray = (alpha * 255.0).clamp(0.0, 255.0) as u8;
            template.put_pixel(x, y, Luma([gray]));
        }
    }

    template
}
