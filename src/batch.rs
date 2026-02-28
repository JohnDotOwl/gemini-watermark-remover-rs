//! Batch processing for multiple images
//! Handles processing of multiple input files with progress tracking

use image::GenericImageView;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::alpha_map;
use crate::blend_modes;
use crate::ncc;
use crate::watermark;

#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub output_dir: Option<PathBuf>,
    pub compare: bool,
    pub verbose: bool,
    pub parallel: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            output_dir: None,
            compare: false,
            verbose: false,
            parallel: true,
        }
    }
}

#[derive(Debug)]
pub struct ProcessResult {
    pub input: PathBuf,
    pub output: PathBuf,
    pub success: bool,
    pub watermark_found: bool,
    pub confidence: f32,
    pub duration_ms: u128,
}

pub fn process_batch(inputs: &[PathBuf], config: &BatchConfig) -> Vec<ProcessResult> {
    use rayon::prelude::*;

    let total = inputs.len();
    let processed = Arc::new(AtomicUsize::new(0));

    if config.verbose {
        println!("Starting batch processing of {} images...", total);
    }

    let results: Vec<ProcessResult> = if config.parallel {
        inputs
            .par_iter()
            .map(|input| {
                let idx = processed.fetch_add(1, Ordering::SeqCst) + 1;
                process_single(input, idx, total, config)
            })
            .collect()
    } else {
        inputs
            .iter()
            .enumerate()
            .map(|(i, input)| {
                let idx = i + 1;
                process_single(input, idx, total, config)
            })
            .collect()
    };

    if config.verbose {
        println!("Batch processing complete: {} images processed", total);
    }

    results
}

fn process_single(input: &Path, index: usize, total: usize, config: &BatchConfig) -> ProcessResult {
    let start = std::time::Instant::now();

    if config.verbose {
        println!("[{}/{}] Processing: {}", index, total, input.display());
    }

    let output = generate_output_path(input, &config.output_dir);

    let mut img = match image::open(input) {
        Ok(img) => img,
        Err(e) => {
            eprintln!("Failed to load {}: {}", input.display(), e);
            return ProcessResult {
                input: input.to_path_buf(),
                output,
                success: false,
                watermark_found: false,
                confidence: 0.0,
                duration_ms: start.elapsed().as_millis(),
            };
        }
    };

    let (width, height) = img.dimensions();

    if config.verbose {
        println!("[{}/{}] Image size: {}x{}", index, total, width, height);
    }

    let watermark_size = watermark::detect_watermark_size(width, height);
    let position = watermark::calculate_watermark_position(width, height, watermark_size);

    if config.verbose {
        println!(
            "[{}/{}] Watermark: {}x{} at ({}, {})",
            index, total, watermark_size, watermark_size, position.x, position.y
        );
    }

    let alpha_map = match alpha_map::load_alpha_map(watermark_size) {
        Ok(map) => map,
        Err(e) => {
            eprintln!("Failed to load alpha map: {}", e);
            return ProcessResult {
                input: input.to_path_buf(),
                output,
                success: false,
                watermark_found: false,
                confidence: 0.0,
                duration_ms: start.elapsed().as_millis(),
            };
        }
    };

    let template = ncc::create_template_from_alpha_map(&alpha_map, watermark_size, watermark_size);
    let gray_img = img.to_luma8();
    let ncc_config = ncc::NccConfig::default();
    let ncc_result = ncc::detect_watermark(&gray_img, &template, &ncc_config);

    if config.verbose {
        println!(
            "[{}/{}] NCC confidence: {:.2}%",
            index,
            total,
            ncc_result.confidence * 100.0
        );
        println!(
            "[{}/{}] Watermark found: {}",
            index, total, ncc_result.found
        );
    }

    blend_modes::remove_watermark(&mut img, &alpha_map, &position);

    if let Err(e) = img.save(&output) {
        eprintln!("Failed to save {}: {}", output.display(), e);
        return ProcessResult {
            input: input.to_path_buf(),
            output,
            success: false,
            watermark_found: ncc_result.found,
            confidence: ncc_result.confidence,
            duration_ms: start.elapsed().as_millis(),
        };
    }

    if config.compare
        && let Err(e) = generate_comparison(input, &output, &output, config.verbose)
    {
        eprintln!("Failed to generate comparison: {}", e);
    }

    let duration = start.elapsed().as_millis();

    if config.verbose {
        println!("[{}/{}] Done in {}ms", index, total, duration);
    }

    ProcessResult {
        input: input.to_path_buf(),
        output,
        success: true,
        watermark_found: ncc_result.found,
        confidence: ncc_result.confidence,
        duration_ms: duration,
    }
}

fn generate_output_path(input: &Path, output_dir: &Option<PathBuf>) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default();
    let ext = input.extension().unwrap_or_default();
    let filename = format!("{}_clean.{}", stem.to_string_lossy(), ext.to_string_lossy());

    match output_dir {
        Some(dir) => dir.join(filename),
        None => input.with_file_name(&filename),
    }
}

fn generate_comparison(
    input: &Path,
    output: &Path,
    base_path: &Path,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let stem = base_path.file_stem().unwrap_or_default().to_string_lossy();
    let comp_path = base_path.with_file_name(format!("{}_compare.png", stem));

    if verbose {
        println!("Generating comparison: {}", comp_path.display());
    }

    let input_img = image::open(input)?;
    let output_img = image::open(output)?;

    let (width, height) = input_img.dimensions();
    let mut comparison = image::RgbaImage::new(width * 2, height);

    let input_rgba = input_img.to_rgba8();
    let output_rgba = output_img.to_rgba8();

    for y in 0..height {
        for x in 0..width {
            let pixel = *input_rgba.get_pixel(x, y);
            comparison.put_pixel(x, y, pixel);
        }
    }

    for y in 0..height {
        for x in 0..width {
            let pixel = *output_rgba.get_pixel(x, y);
            comparison.put_pixel(x + width, y, pixel);
        }
    }

    let divider_start = width.saturating_sub(2);
    let divider_end = (width + 2).min(comparison.width());
    for y in 0..height {
        for x in divider_start..divider_end {
            comparison.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
        }
    }

    comparison.save(&comp_path)?;
    Ok(())
}

pub fn print_summary(results: &[ProcessResult], verbose: bool) {
    let total = results.len();
    let successful = results.iter().filter(|r| r.success).count();
    let watermarks_found = results.iter().filter(|r| r.watermark_found).count();
    let avg_confidence: f32 =
        results.iter().map(|r| r.confidence).sum::<f32>() / total.max(1) as f32;
    let total_time: u128 = results.iter().map(|r| r.duration_ms).sum();

    println!("\n=== Batch Summary ===");
    println!("Total images: {}", total);
    println!("Successfully processed: {}", successful);
    println!("Watermarks detected: {}", watermarks_found);
    if verbose {
        println!("Average confidence: {:.1}%", avg_confidence * 100.0);
    }
    println!(
        "Total time: {}ms ({:.2}s)",
        total_time,
        total_time as f64 / 1000.0
    );

    if verbose {
        println!("\nDetailed Results:");
        for (i, result) in results.iter().enumerate() {
            println!(
                "  [{}] {} -> {} (success: {}, watermark: {:.0}%)",
                i + 1,
                result
                    .input
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
                result
                    .output
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy(),
                result.success,
                result.confidence * 100.0
            );
        }
    }
}
