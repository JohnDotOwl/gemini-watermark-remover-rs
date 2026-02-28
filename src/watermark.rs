pub use crate::blend_modes::Position;

pub fn detect_watermark_size(width: u32, height: u32) -> u32 {
    // Gemini's watermark rules:
    // If both image width and height are greater than 1024, use 96×96 watermark
    // Otherwise, use 48×48 watermark
    if width > 1024 && height > 1024 {
        println!("Auto-detected 96x96 watermark (image > 1024x1024)");
        96
    } else {
        println!("Auto-detected 48x48 watermark (image <= 1024x1024)");
        48
    }
}

pub fn calculate_watermark_position(
    image_width: u32,
    image_height: u32,
    logo_size: u32,
) -> Position {
    let (margin_right, margin_bottom) = if logo_size == 96 { (64, 64) } else { (32, 32) };

    Position {
        x: image_width.saturating_sub(margin_right + logo_size),
        y: image_height.saturating_sub(margin_bottom + logo_size),
        width: logo_size,
        height: logo_size,
    }
}
