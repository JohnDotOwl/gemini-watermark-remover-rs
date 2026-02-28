use image::DynamicImage;

const ALPHA_THRESHOLD: f32 = 0.002;
const MAX_ALPHA: f32 = 0.99;
const LOGO_VALUE: f32 = 255.0;

pub struct Position {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn remove_watermark(img: &mut DynamicImage, alpha_map: &[f32], position: &Position) {
    let rgba = img.to_rgba8();
    let mut output = rgba.clone();

    let (img_width, img_height) = rgba.dimensions();

    for row in 0..position.height {
        for col in 0..position.width {
            let img_x = position.x + col;
            let img_y = position.y + row;

            if img_x >= img_width || img_y >= img_height {
                continue;
            }

            let alpha_idx = (row * position.width + col) as usize;

            let mut alpha = alpha_map[alpha_idx];

            // Skip very small alpha values (noise)
            if alpha < ALPHA_THRESHOLD {
                continue;
            }

            // Limit alpha to avoid division by near-zero
            alpha = alpha.min(MAX_ALPHA);
            let one_minus_alpha = 1.0 - alpha;

            let pixel = rgba.get_pixel(img_x, img_y);
            let mut new_pixel = *pixel;

            // Apply reverse alpha blending to each RGB channel
            for c in 0..3 {
                let watermarked = pixel[c] as f32;
                let original = (watermarked - alpha * LOGO_VALUE) / one_minus_alpha;
                new_pixel[c] = original.clamp(0.0, 255.0) as u8;
            }

            // Alpha channel remains unchanged
            output.put_pixel(img_x, img_y, new_pixel);
        }
    }

    *img = DynamicImage::ImageRgba8(output);
}
