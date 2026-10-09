# dominant-colour: Context Document

**Project**: `dominant-colour` - Generate dominant colour base64 data URIs for
use as website image placeholders

**Version**: 0.1.0
**License**: BSD-3-Clause
**Repository**: https://github.com/rodneylab/dominant-colour

---

## Project Overview

A CLI tool that analyses an input image, computes its dominant colour using
k-means clustering, and outputs either:

- a base64 data URI of a solid-colour placeholder image in original format
  (AVIF, JPEG, PNG or WebP supported); or
- an SVG placeholder with the dominant colour

Use case: Generate placeholder images for websites while original images
are loading.

---

## Architecture

### Core Modules

| Module                                                                                     | Purpose                                                                         |
| ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------- |
| `src/main.rs`                                                                              | Entry point; parses CLI, orchestrates image processing, logging setup           |
| `src/placeholder.rs`                                                                       | K-means colour computation (16 seeds, best result), SVG placeholder generation, |
| byte output generation with format optimisation                                            |                                                                                 |
| `src/utilities/image.rs`                                                                   | Image loading with format detection, resizing, base64 data URI generation       |
| `src/utilities/ui.rs` - Clipboard copy via arboard                                         |                                                                                 |
| `src/cli/mod.rs` - CLI argument parsing with clap; subcommands: `generate`,                |                                                                                 |
| `markdown-help` (internal-tools feature)                                                   |                                                                                 |
| `src/io.rs` - File I/O with size limiting (max 16 MiB input)                               |                                                                                 |
| `src/errors.rs` - Error types: `IoError`, `ImageError`, `AppError` with miette diagnostics |                                                                                 |
| `src/cli/styles.rs` - Custom clap styling (green headers, cyan literals)                   |                                                                                 |

### Key Types

| Type                                                                           | Description |
| ------------------------------------------------------------------------------ | ----------- |
| `DominantColourOptions` - k-configuration: `k` (cluster count, default 8),     |             |
| `max_iter` (default 20), `converge` threshold (default 5.0)                    |             |
| `SvgPlaceholder` - Askama template-rendered SVG struct with `width`, `height`, |             |
| `dominant_colour`                                                              |             |
| `GenerateArgs` - CLI args: `input` (required `PathBuf`), `output` (optional    |             |
| `PathBuf`), `clipboard` (`bool`), `svg` (`bool`)                               |             |
| `FitMode` - Enum with `Clip` variant (default); resizes within bounds          |             |
| maintaining aspect ratio                                                       |             |

### Data Flow

1. `main()` parses CLI arguments → `GenerateArgs` → `initialise_logging()`
2. `generate_placeholder()`:
   - `open_file()` reads image bytes, validates ≤16 MiB limit
   - `load_image()` detects format (AVIF/JPEG/PNG/WebP) and decodes into `DynamicImage`
   - `resize_image()` resizes to 100×100 with `FitMode::Clip` (maintains aspect ratio)
   - `determine_dominant_colour()`:
     - Converts RGB → linear sRGB → LAB colour space
     - Runs k-means clustering with 16 different seeds
     - Returns best result (lowest score)
   - If `--svg`: renders SVG template (`templates/placeholder.svg`) with colour hex;
     writes to file or outputs data URI
   - If not `--svg`: generates solid-colour image bytes via `generate_placeholder_bytes()`,
     optionally optimises with mozjpeg (JPEG) or oxipng (PNG), outputs data URI or writes to file

---

## Build & Run

```bash
# Build
cargo build

# Build with internal-tools feature (enables markdown help generation)
cargo build --features internal-tools

# Run generate on an image
cargo run -- generate input.jpg

# Output as SVG
cargo run -- generate input.jpg --svg

# Copy URI to clipboard
cargo run -- generate input.jpg --clipboard

# Generate markdown help docs
cargo run -- markdown-help
```

---

## Usage Examples

### Basic Data URI Output

```bash
dominant-colour generate photo.png
# Output: data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAA...
```

### SVG Placeholder

```bash
dominant-colour generate photo.png --svg
# Writes photo.svg with solid-colour rectangle
```

### Clipboard

```bash
dominant-colour generate photo.png --clipboard
# Copies data URI to system clipboard
```

### Program Overview

```bash
dominant-colour --help
# Shows: dominant-colour [OPTIONS] <COMMAND>
# Options: -v/--verbose, -q/--quiet
# Commands: generate, markdown-help (internal-tools feature)
```

---

## API Functions (for Library Reuse)

| Function                     | Signature                                                                                                                                | Description                                              |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `image_to_data_uri`          | `pub fn image_to_data_uri(image_bytes: &[u8], mime_type: &str) -> String`                                                                | Converts raw bytes to data URI (no mime-type validation) |
| `load_image`                 | `pub fn load_image(bytes: &[u8]) -> Result<(ImageFormat, DynamicImage), AppError>`                                                       | Decodes image, returns format + image                    |
| `resize_image`               | `pub fn resize_image(input: &DynamicImage, width: Option<u32>, height: Option<u32>, fit: Option<FitMode>) -> DynamicImage`               | Resizes with aspect-ratio preservation                   |
| `compute_kmeans`             | `pub fn compute_kmeans(options: &DominantColourOptions, lab: &[Lab]) -> Kmeans<Lab>`                                                     | Runs k-means across 16 seeds, returns best result        |
| `compute_dominant_colour`    | `pub fn compute_dominant_colour(kmeans: &Kmeans<Lab>) -> Result<Rgb<u8>, ImageError>`                                                    | Extracts dominant RGB from k-means result                |
| `generate_placeholder_bytes` | `pub fn generate_placeholder_bytes(dominant_colour: Rgb<u8>, width: u32, height: u32, format: ImageFormat) -> Result<Vec<u8>, AppError>` | Creates optimised placeholder bytes                      |

---

## Development Notes

### Testing

```bash
cargo test       # Run all tests (all features, includes integration & unit tests)
cargo test --lib # Run library tests only
```

Tests use `insta` snapshots. Run `cargo insta review` to accept/reject new snapshots.
Some tests reference fixtures in `src/fixtures/` and `src/utilities/fixtures/`.

### Linting

```bash
cargo clippy   # Run clippy linter (pedantic level, all warnings must be fixed)
cargo fmt      # Format code (rustfmt, edition=2024, unix newlines)
```

### Adding New Features

1. **New output format**: Add variant to `image::ImageFormat` match arm in
   `generate_placeholder_bytes()` (`src/placeholder.rs:85-95`). Note: only AVIF,
   PNG, JPEG and WebP are currently supported with optimisation; other formats
   use lossless encoding.
2. **New CLI subcommand**: Add variant to `Commands` enum in `src/cli/mod.rs`,
   implement handler in `main()`.
3. **New utility function**: Add to `src/utilities/image.rs` or
   `src/placeholder.rs`, re-export from `src/main.rs` if public API.

### Configuration

- Default k-means options: `k=8`, `max_iter=20`, `converge=5.0`
  (`src/placeholder.rs:17-25`)
- Max input image size: 16 mebibytes (`src/io.rs:10`) - files larger than this
  return an `IoError` with advice to optimise the file
- Colour space pipeline: sRGB → Linear sRGB → LAB for k-means analysis
  (`src/main.rs:55-58`)
- Image resizing: All images resized to 100×100 with `FitMode::Clip` before
  k-means analysis (`main.rs:104`)

### Key Dependencies

- `image` - Image I/O and processing
- `kmeans_colors` - K-means clustering
- `palette` - Colour space conversions (sRGB/LAB)
- `clap` + `clap-markdown` + `clap-verbosity-flag` - CLI parsing
- `askama` - SVG template rendering
- `base64` - Base64 encoding for data URIs
- `oxipng` - PNG optimization (Zopfli deflater)
- `miette` - Error handling with diagnostics
- `arboard` - Clipboard access
- `mozjpeg_rs` - JPEG optimization

---

## AI Assistant Notes

### Common Entry Points

- User typically invokes via `cargo run -- generate <image>` or
  the compiled binary
- Look at `src/main.rs:98-153` for CLI dispatch logic and
  `generate_placeholder()` function
- Key functions:
  - `determine_dominant_colour()` - runs k-means, returns dominant RGB
  - `generate_placeholder()` - main orchestration function
  - `resize_image()` - resizes input to 100×100 with FitMode::Clip
  - `generate_placeholder_bytes()` - creates optimised output bytes

### Templates

- SVG template at `templates/placeholder.svg` uses Askama syntax: `{{ width }}`,
  `{{ height }}`, `{{ dominant_colour }}`
- Template rendering via `#[derive(askama::Template)]` on `SvgPlaceholder`
  struct
- SVG output writes to file with `.svg` extension if `output` argument provided,
  otherwise outputs data URI with `image/svg+xml` mime type

### Error Handling

- All errors go through `AppError` → `ImageError` / `IoError`
- Each error type has `advice` (user remedy) and `detail` (technical details)
- Use `#[from]` conversions for seamless error translation between error types
- Common error scenarios:
  - File too large (>16 MiB): `IoError` with advice to optimise the file
  - Unsupported format: `ImageError` with advice to check file
    corruption/extension
  - Output directory not found: `IoError` with "Check path exists" help

### Typical modification patterns

- To change output size: modify `resize_image()` call in `src/main.rs:112`
  (currently 100×100 with FitMode::Clip)
- To change k-means parameters: modify `DominantColourOptions::default()` or
  pass custom options to `compute_kmeans()`
- To support new image format: add match arm in `generate_placeholder_bytes()`
  format optimisation (currently AVIF, PNG, JPEG, WebP)
- To change resize dimensions: modify the `Some(100), Some(100)` args in
  `src/main.rs:112`
