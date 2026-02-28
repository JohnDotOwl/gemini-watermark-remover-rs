use image::{GenericImageView, ImageError};

pub fn load_alpha_map(size: u32) -> Result<Vec<f32>, ImageError> {
    let path = if size == 48 {
        "bg_48.png"
    } else if size == 96 {
        "bg_96.png"
    } else {
        panic!("Unsupported watermark size: {}", size);
    };

    let img = image::open(path)?;
    let (width, height) = img.dimensions();

    if width != size || height != size {
        panic!(
            "Alpha map size mismatch: expected {}x{}, got {}x{}",
            size, size, width, height
        );
    }

    let mut alpha_map = Vec::with_capacity((width * height) as usize);

    for pixel in img.pixels() {
        let rgb = pixel.2;
        let r = rgb[0] as f32;
        let g = rgb[1] as f32;
        let b = rgb[2] as f32;

        // Take maximum RGB channel and normalize to [0, 1]
        let max_channel = r.max(g).max(b);
        let alpha = max_channel / 255.0;

        alpha_map.push(alpha);
    }

    Ok(alpha_map)
}
