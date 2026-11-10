use std::io::{Cursor, Read, Seek};

use image::{
    DynamicImage::ImageRgb8, ImageBuffer, ImageFormat, Rgb, RgbImage, codecs::avif::AvifEncoder,
};
use kmeans_colors::{Kmeans, Sort, get_kmeans};
use log::Level;
use mozjpeg_rs::Preset;
use oxipng::{Deflater::Zopfli, ZopfliOptions, optimize_from_memory};
use palette::{IntoColor, Lab, Srgb};

use crate::errors::{AppError, ImageError, IoError};

const AVIF_QUALITY: u8 = 100;
const AVIF_SPEED: u8 = 1;

#[cfg_attr(test, derive(serde::Serialize))]
pub struct DominantColourOptions {
    pub k: usize,
    pub max_iter: usize,
    pub converge: f32,
}

impl Default for DominantColourOptions {
    fn default() -> Self {
        Self {
            k: 8,
            max_iter: 20,
            converge: 5.0,
        }
    }
}

#[derive(askama::Template)]
#[template(path = "placeholder.svg")]
pub struct SvgPlaceholder {
    pub width: u32,
    pub height: u32,
    pub dominant_colour: String,
}

pub fn compute_kmeans(options: &DominantColourOptions, lab: &[Lab]) -> Kmeans<Lab> {
    log::info!("Lab: {lab:?}");
    // [`Kmeans::new`] sets result.score to [`f32::MAX`]
    let mut result = Kmeans::new();
    let verbose = log::log_enabled!(Level::Debug);
    for seed in 1..=16 {
        let run_result = get_kmeans(
            options.k,
            options.max_iter,
            options.converge,
            verbose,
            lab,
            seed,
        );
        if run_result.score < result.score {
            result = run_result;
        }
    }

    result
}

pub fn compute_dominant_colour(kmeans: &Kmeans<Lab>) -> Result<Rgb<u8>, ImageError> {
    let res = Lab::sort_indexed_colors(&kmeans.centroids, &kmeans.indices);
    let dominant_colour = Lab::get_dominant_color(&res).ok_or(ImageError {
        advice: String::from("Try with another image"),
        detail: String::from("No centroid found in k-means analysis"),
    })?;
    let dominant_colour: Srgb = dominant_colour.into_color();
    let dominant_colour: Srgb<u8> = dominant_colour.into();
    let dominant_colour: [u8; 3] = dominant_colour.into();

    Ok(Rgb::from(dominant_colour))
}

fn output_bytes(cursor: &mut Cursor<Vec<u8>>) -> Result<Vec<u8>, IoError> {
    let mut buffer = Vec::new();
    cursor.rewind().map_err(IoError::from)?;
    cursor.read_to_end(&mut buffer).map_err(IoError::from)?;

    Ok(buffer)
}

fn compress_and_encode_avif(
    image: ImageBuffer<Rgb<u8>, Vec<u8>>,
    cursor: &mut Cursor<Vec<u8>>,
) -> Result<Vec<u8>, AppError> {
    let encoder = AvifEncoder::new_with_speed_quality(&mut *cursor, AVIF_SPEED, AVIF_QUALITY);
    let placeholder_image = ImageRgb8(image);
    placeholder_image
        .write_with_encoder(encoder)
        .map_err(ImageError::from)?;

    Ok(output_bytes(cursor)?)
}

fn compress_and_encode_png(
    image: ImageBuffer<Rgb<u8>, Vec<u8>>,
    cursor: &mut Cursor<Vec<u8>>,
) -> Result<Vec<u8>, AppError> {
    let placeholder_image = ImageRgb8(image);
    placeholder_image
        .write_to(&mut *cursor, ImageFormat::Png)
        .map_err(ImageError::from)?;
    let buffer = output_bytes(cursor)?;

    Ok(optimize_from_memory(
        &buffer,
        &oxipng::Options {
            deflater: Zopfli(ZopfliOptions::default()),
            ..Default::default()
        },
    )
    .map_err(ImageError::from)?)
}

/// Generate the placeholder byes in native format.
///
/// Optimises output for JPEG and PNG files, using `mozjpeg` and `OxiPNG` respectively.
/// Optimisation is currently aggressive and it is worth considering backing off where speed is
/// important.  Other output formats are not currently optimised.
pub fn generate_placeholder_bytes(
    dominant_colour: Rgb<u8>,
    width: u32,
    height: u32,
    format: ImageFormat,
) -> Result<Vec<u8>, AppError> {
    let output_image = RgbImage::from_pixel(width, height, dominant_colour);
    let mut cursor = Cursor::new(Vec::new());

    let output_bytes = match format {
        ImageFormat::Avif => compress_and_encode_avif(output_image, &mut cursor)?,
        ImageFormat::Png => compress_and_encode_png(output_image, &mut cursor)?,
        ImageFormat::Jpeg => mozjpeg_rs::Encoder::new(Preset::ProgressiveSmallest)
            .quality(100)
            .fast_color(true)
            .encode_rgb(output_image.as_raw(), width, height)
            .map_err(ImageError::from)?,
        format => {
            debug_assert!(matches!(format, ImageFormat::WebP));
            // Logic here is generic.  Crate only supports AVIF, PNG, JPEG and WebP, so this
            // branch is currently only executed for WebP code.  If other formats that need
            // compression or other special handling are introduced, add a specialised match case.
            //
            // WebP - uses image crate WebP encoder, which uses lossless compression.  Consider
            // `libwebp` as an alternative if appropriate.
            let placeholder_image = ImageRgb8(output_image);
            placeholder_image
                .write_to(&mut cursor, format)
                .map_err(ImageError::from)?;

            output_bytes(&mut cursor)?
        }
    };

    Ok(output_bytes)
}

#[cfg(test)]
mod tests {
    use std::{fs::read, io::Cursor, path::PathBuf};

    use askama::Template;
    use image::{ImageFormat, Rgb};
    use kmeans_colors::get_kmeans;
    use palette::{IntoColor, Lab, Srgb, cast::from_component_vec};

    use crate::placeholder::{
        DominantColourOptions, SvgPlaceholder, compute_dominant_colour, compute_kmeans,
        generate_placeholder_bytes,
    };

    #[test]
    fn dominant_colout_options_default_returns_expected_values() {
        // arrange
        // act
        let outcome = DominantColourOptions::default();

        // assert
        insta::assert_json_snapshot!(outcome);
    }

    #[test]
    fn svg_placeholder_generates_expected_svg_content() {
        // arrange
        let placeholder = SvgPlaceholder {
            width: 75,
            height: 50,
            dominant_colour: String::from("#abcdef"),
        };

        // act
        let outcome = placeholder.render().unwrap();

        // assert
        insta::assert_snapshot!(outcome);
    }

    #[test]
    fn compute_kmeans_generates_expected_result() {
        // arrange
        let options = DominantColourOptions {
            k: 8,
            max_iter: 20,
            converge: 5.0,
        };
        let image_bytes =
            read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/997-75x75.avif"))
                .unwrap();
        let image = image::load(Cursor::new(image_bytes), ImageFormat::Avif).unwrap();
        let rgb8_image: Vec<u8> = image.to_rgb8().into_raw();
        let lab: Vec<Lab> = from_component_vec::<Srgb<u8>>(rgb8_image)
            .iter()
            .map(|x| x.into_linear().into_color())
            .collect();

        // act
        let outcome = compute_kmeans(&options, &lab);

        // assert
        insta::assert_snapshot!(format!("{outcome:?}"));
    }

    #[test]
    fn compute_dominant_colour_returns_expected_result() {
        // arrange
        let options = DominantColourOptions {
            k: 8,
            max_iter: 20,
            converge: 5.0,
        };
        let image_bytes =
            read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/997-75x75.avif"))
                .unwrap();
        let image = image::load(Cursor::new(image_bytes), ImageFormat::Avif).unwrap();
        let rgb8_image: Vec<u8> = image.to_rgb8().into_raw();
        let lab: Vec<Lab> = from_component_vec::<Srgb<u8>>(rgb8_image)
            .iter()
            .map(|x| x.into_linear().into_color())
            .collect();
        let kmeans = get_kmeans(
            options.k,
            options.max_iter,
            options.converge,
            false,
            &lab,
            5678,
        );

        // act
        let outcome = compute_dominant_colour(&kmeans).unwrap();

        // assert
        assert_eq!(outcome.0, [52, 87, 141]);
    }

    #[test]
    fn generate_placeholder_bytes_returns_expected_value() {
        // arrange
        let dominant_colour = Rgb::from([0, 128, 255]);
        let width = 80;
        let height = 50;
        let format = ImageFormat::Png;

        // act
        let outcome = generate_placeholder_bytes(dominant_colour, width, height, format).unwrap();

        // assert
        assert_eq!(
            outcome,
            vec![
                137u8, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 80, 0, 0,
                0, 50, 1, 3, 0, 0, 0, 241, 227, 206, 191, 0, 0, 0, 3, 80, 76, 84, 69, 0, 128, 255,
                177, 251, 74, 28, 0, 0, 0, 14, 73, 68, 65, 84, 120, 218, 99, 32, 8, 70, 193, 40, 0,
                0, 2, 38, 0, 1, 9, 62, 35, 185, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130
            ]
        );
    }
}
