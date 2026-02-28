# gemini-watermark-remover-rs

A Rust CLI and library for removing visible Gemini image watermarks using reverse alpha blending, with optional NCC-based watermark validation.

## Features

- Removes visible Gemini watermark overlays with deterministic math (no AI inpainting).
- Supports single-image and multi-image batch processing.
- Uses pre-calibrated 48x48 and 96x96 alpha maps.
- Reports NCC confidence scores for watermark presence checks.
- Can generate side-by-side comparison images.
- Runs locally with no network calls.

## Installation

### From source

```bash
git clone https://github.com/JohnDotOwl/gemini-watermark-remover-rs.git
cd gemini-watermark-remover-rs
cargo install --path .
```

### Build a release binary manually

```bash
cargo build --release
./target/release/gemini-watermark-remover-rs --help
```

## Usage

### Basic example

```bash
gemini-watermark-remover-rs --input photo.jpg --output photo_clean.jpg
```

### Batch examples

```bash
# Process multiple files
gemini-watermark-remover-rs --input photo1.jpg photo2.jpg photo3.jpg --batch

# Output to a directory
gemini-watermark-remover-rs --input *.jpg --output ./cleaned --batch

# Enable verbose progress and comparison image generation
gemini-watermark-remover-rs --input *.webp --batch --verbose --compare
```

### Force watermark size

```bash
# Force 48x48 watermark mode
gemini-watermark-remover-rs --input photo.jpg --force-small

# Force 96x96 watermark mode
gemini-watermark-remover-rs --input photo.jpg --force-large
```

### Command reference

| Option | Short | Description |
| --- | --- | --- |
| `--input <FILE...>` | `-i` | Input image file(s). |
| `--output <PATH>` | `-o` | Output file (single mode) or directory (batch mode). |
| `--force-small` |  | Force 48x48 watermark size. |
| `--force-large` |  | Force 96x96 watermark size. |
| `--batch` |  | Force batch mode. |
| `--compare` |  | Write side-by-side comparison image(s). |
| `--verbose` | `-v` | Print detailed progress output. |
| `--single-threaded` |  | Disable Rayon parallelism in batch mode. |
| `--help` | `-h` | Show help. |
| `--version` | `-V` | Show version. |

## How It Works

1. Detect watermark size from image dimensions (or use forced size flags).
2. Compute expected watermark position from Gemini margin rules.
3. Load alpha template (`bg_48.png` or `bg_96.png`).
4. Optionally run normalized cross-correlation (NCC) to estimate confidence.
5. Invert the alpha blending equation to recover underlying pixels.

Reverse blending equation:

```text
watermarked = alpha * logo + (1 - alpha) * original
original = (watermarked - alpha * 255) / (1 - alpha)
```

## Watermark Rules

| Condition | Watermark | Right Margin | Bottom Margin |
| --- | --- | --- | --- |
| `width > 1024 && height > 1024` | `96x96` | `64px` | `64px` |
| Otherwise | `48x48` | `32px` | `32px` |

## Supported Formats

Supports common image formats handled by the `image` crate, including:

- JPEG (`.jpg`, `.jpeg`)
- PNG (`.png`)
- WebP (`.webp`)
- BMP (`.bmp`)

## Library Usage

This repository also exposes a Rust library crate.

```rust
use gemini_watermark_remover_rs::{alpha_map, blend_modes, watermark};

let mut img = image::open("photo.jpg")?;
let (w, h) = img.dimensions();
let size = watermark::detect_watermark_size(w, h);
let position = watermark::calculate_watermark_position(w, h, size);
let alpha = alpha_map::load_alpha_map(size)?;
blend_modes::remove_watermark(&mut img, &alpha, &position);
img.save("photo_clean.jpg")?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Development

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

## Contributing

Contributions are welcome.

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/my-change`).
3. Make focused commits with tests where possible.
4. Open a pull request describing behavior changes.

For release notes and version history, see [CHANGELOG.md](CHANGELOG.md).

## Limitations and Disclaimer

- This tool targets visible Gemini watermark overlays only.
- It does not remove invisible watermarking systems (for example SynthID).
- Results depend on watermark position and template compatibility.
- Confirm your usage complies with platform terms and applicable law.

## License

MIT License. See [LICENSE](LICENSE) for details.
