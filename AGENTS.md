# AGENTS.md — dominant-colour

## Build & Run

```bash
# Build
cargo build

# Run tests (all features, includes integration & unit tests)
cargo test --all-features

# Lint
cargo clippy

# Format
cargo fmt
```

## CLI Usage

```bash
# Basic data URI output
cargo run -- generate input.jpg

# Output as SVG
cargo run -- generate input.jpg --svg

# Copy URI to clipboard
cargo run -- generate input.jpg --clipboard

# Generate markdown help (requires internal-tools feature)
cargo run --features internal-tools -- markdown-help
```

## Key Behaviours

- **Default k-means options**: `k=8`, `max_iter=20`, `converge=5.0`
  (`placeholder.rs:17-25`). Override via `DominantColourOptions::default()` or
  custom options.
- **Input size limit**: 16 mebibyte max. Files larger than this will return an
  `IoError` with advice to optimise the file.
- **Image resizing**: All images resized to 100×100 with `FitMode::Clip` before
  k-means analysis (`main.rs:104`).
- **Output modes**:
  - `--svg`: renders Askama SVG template (`templates/placeholder.svg`) with
    colour hex, width, height
  - Without `--svg`: generates solid-colour image in native format, optimised
    (PNG→oxipng, JPEG→mozjpeg)
  - `--clipboard`: copies data URI to system clipboard (arboard)
  - `--output <path>`: writes output to file instead of standard output

## API Functions (Library Reuse)

| Function                     | Signature                                                                                                                                | Notes                                         |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| `image_to_data_uri`          | `pub fn image_to_data_uri(image_bytes: &[u8], mime_type: &str) -> String`                                                                | No mime-type validation                       |
| `load_image`                 | `pub fn load_image(bytes: &[u8]) -> Result<(ImageFormat, DynamicImage), AppError>`                                                       | Detects format, decodes image                 |
| `resize_image`               | `pub fn resize_image(input: &DynamicImage, width: Option<u32>, height: Option<u32>, fit: Option<FitMode>) -> DynamicImage`               | Uses `FitMode::Clip` by default               |
| `compute_kmeans`             | `pub fn compute_kmeans(options: &DominantColourOptions, lab: &[Lab]) -> Kmeans<Lab>`                                                     | Runs 16 seeds, returns best result            |
| `compute_dominant_colour`    | `pub fn compute_dominant_colour(kmeans: &Kmeans<Lab>) -> Result<Rgb<u8>, ImageError>`                                                    | Extracts dominant RGB                         |
| `generate_placeholder_bytes` | `pub fn generate_placeholder_bytes(dominant_colour: Rgb<u8>, width: u32, height: u32, format: ImageFormat) -> Result<Vec<u8>, AppError>` | Optimises PNG/JPEG; other formats unoptimised |

## Testing

- Tests use `insta` snapshots. Run `cargo insta review` to accept/reject new
  snapshots.
- Some tests reference fixtures in `src/fixtures/` and
  `src/utilities/fixtures/`.
- To add a new test for a feature, follow existing patterns:
  1. arrange input;
  2. act on function; and
  3. assert outcome.

## Code Conventions

- `missing_docs = "deny"` in `Cargo.toml` — all public items must be documented.
- Clippy runs at pedantic level (`Cargo.toml:48-67`). Fix all warnings.
- `rustfmt` configured via `.rustfmt.toml` (edition=2024, UNIX newlines).
- Clipping styles in `cli/styles.rs` give green headers, cyan literals in
  CLI output.
- K-means colour space: sRGB → Linear sRGB → LAB (`main.rs:55-58`).
