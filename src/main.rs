use clap::Parser;
use gemini_watermark_remover_rs::{alpha_map, batch, blend_modes, ncc, watermark};
use image::GenericImageView;
use std::path::{Path, PathBuf};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, value_name = "FILE", num_args(1..))]
    input: Vec<PathBuf>,

    #[arg(short, long, value_name = "PATH")]
    output: Option<PathBuf>,

    #[arg(long)]
    force_small: bool,

    #[arg(long)]
    force_large: bool,

    #[arg(long)]
    batch: bool,

    #[arg(long)]
    compare: bool,

    #[arg(short, long)]
    verbose: bool,

    #[arg(long)]
    single_threaded: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.input.is_empty() {
        eprintln!("Error: No input files specified");
        eprintln!(
            "Usage: gemini-watermark-remover-rs --input <file> [file2 file3 ...] [--output <path>]"
        );
        std::process::exit(1);
    }

    let is_batch = args.input.len() > 1 || args.batch;

    if is_batch {
        run_batch_mode(args)
    } else {
        run_single_mode(args)
    }
}

fn run_single_mode(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let input = &args.input[0];

    println!("=== Gemini Watermark Remover (v{}) ===", VERSION);
    println!("Mode: Single Image Processing");
    println!("Input: {}", input.display());

    let mut img = image::open(input)?;
    let (width, height) = img.dimensions();

    println!("Image size: {}x{}", width, height);

    let watermark_size = if args.force_small {
        println!("Forcing 48x48 watermark size");
        48
    } else if args.force_large {
        println!("Forcing 96x96 watermark size");
        96
    } else {
        watermark::detect_watermark_size(width, height)
    };

    println!(
        "Using watermark size: {}x{}",
        watermark_size, watermark_size
    );

    let position = watermark::calculate_watermark_position(width, height, watermark_size);
    println!(
        "Watermark position: x={}, y={}, size={}x{}",
        position.x, position.y, position.width, position.height
    );

    let alpha_map = alpha_map::load_alpha_map(watermark_size)?;
    println!("Loaded alpha map: {}x{}", watermark_size, watermark_size);

    let template = ncc::create_template_from_alpha_map(&alpha_map, watermark_size, watermark_size);
    let gray_img = img.to_luma8();
    let ncc_config = ncc::NccConfig::default();
    let ncc_result = ncc::detect_watermark(&gray_img, &template, &ncc_config);

    println!("NCC Detection:");
    println!("  Watermark found: {}", ncc_result.found);
    println!("  Confidence: {:.1}%", ncc_result.confidence * 100.0);

    if let Some(pos) = ncc_result.position {
        println!("  Detected at: ({}, {})", pos.0, pos.1);
        println!("  Expected at: ({}, {})", position.x, position.y);
        let offset = (
            (pos.0 as i32 - position.x as i32).unsigned_abs(),
            (pos.1 as i32 - position.y as i32).unsigned_abs(),
        );
        println!("  Position offset: {:?}", offset);
    }

    println!("\nRemoving watermark...");
    blend_modes::remove_watermark(&mut img, &alpha_map, &position);

    let output = match &args.output {
        Some(out) => {
            if out.is_dir() {
                let stem = input.file_stem().unwrap_or_default();
                let ext = input.extension().unwrap_or_default();
                out.join(format!(
                    "{}_clean.{}",
                    stem.to_string_lossy(),
                    ext.to_string_lossy()
                ))
            } else {
                out.clone()
            }
        }
        None => {
            let stem = input.file_stem().unwrap_or_default();
            let ext = input.extension().unwrap_or_default();
            input.with_file_name(format!(
                "{}_clean.{}",
                stem.to_string_lossy(),
                ext.to_string_lossy()
            ))
        }
    };

    println!("Saving to: {}", output.display());
    img.save(&output)?;

    if args.compare {
        println!("Generating comparison image...");
        let base = output.parent().unwrap_or_else(|| Path::new("."));
        let stem = output.file_stem().unwrap_or_default().to_string_lossy();
        let comp_path = base.join(format!("{}_compare.png", stem));

        let input_img = image::open(input)?;
        let output_img = image::open(&output)?;

        let (img_width, img_height) = input_img.dimensions();
        let mut comparison = image::RgbaImage::new(img_width * 2, img_height);

        let input_rgba = input_img.to_rgba8();
        let output_rgba = output_img.to_rgba8();

        for y in 0..img_height {
            for x in 0..img_width {
                let pixel = *input_rgba.get_pixel(x, y);
                comparison.put_pixel(x, y, pixel);
            }
        }

        for y in 0..img_height {
            for x in 0..img_width {
                let pixel = *output_rgba.get_pixel(x, y);
                comparison.put_pixel(x + img_width, y, pixel);
            }
        }

        let divider_start = img_width.saturating_sub(2);
        let divider_end = (img_width + 2).min(comparison.width());
        for y in 0..img_height {
            for x in divider_start..divider_end {
                comparison.put_pixel(x, y, image::Rgba([255, 0, 0, 255]));
            }
        }

        comparison.save(&comp_path)?;
        println!("Comparison saved: {}", comp_path.display());
    }

    println!("\n✅ Done!");
    Ok(())
}

fn run_batch_mode(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Gemini Watermark Remover (v{}) ===", VERSION);
    println!("Mode: Batch Processing");
    println!("Inputs: {} file(s)", args.input.len());

    let config = batch::BatchConfig {
        output_dir: args.output.clone(),
        compare: args.compare,
        verbose: args.verbose,
        parallel: !args.single_threaded,
    };

    let results = batch::process_batch(&args.input, &config);
    batch::print_summary(&results, args.verbose);

    let failed = results.iter().filter(|r| !r.success).count();
    if failed > 0 {
        eprintln!("\n⚠️  {} image(s) failed to process", failed);
        std::process::exit(1);
    }

    println!("\n✅ Batch processing complete!");
    Ok(())
}
