use std::{
    cmp::{max, min},
    io::Cursor,
};

use base64::{Engine, engine::general_purpose::STANDARD_NO_PAD};
use image::{
    DynamicImage::{self, ImageRgba8},
    ImageFormat, ImageReader,
    error::{ImageFormatHint, UnsupportedErrorKind},
    imageops::{self, FilterType},
};

use crate::errors::{AppError, ImageDimensionError, ImageError, IoError};

pub fn load_image(bytes: &[u8]) -> Result<(ImageFormat, DynamicImage), AppError> {
    let image_reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|err| IoError::new(&err, "reading input image bytes"))?;

    let format = image_reader.format().ok_or(ImageError {
        advice: String::from(
            "Check image file is not corrupted or change file name to indicate format",
        ),
        detail: String::from("Unable to determine image format"),
    })?;
    match format {
        ImageFormat::Avif | ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::WebP => {}
        _ => {
            let hint = ImageFormatHint::Exact(format);
            return Err(AppError::Image(ImageError::from(
                image::ImageError::Unsupported(
                    image::error::UnsupportedError::from_format_and_kind(
                        hint.clone(),
                        UnsupportedErrorKind::Format(hint),
                    ),
                ),
            )));
        }
    }
    log::debug!("Detected {format:?} image");
    let image = image_reader.decode().map_err(ImageError::from)?;
    log::debug!("Successfully decoded image");

    Ok((format, image))
}

#[derive(Debug, Default)]
#[non_exhaustive]
pub enum FitMode {
    /// Resizes the image to fit within the width and height boundaries without cropping or
    /// distorting the image. The resulting image will match one of the constraining dimensions,
    /// while the other dimension is altered to maintain the same aspect ratio of the input image.
    /// <https://docs.imgix.com/en-US/apis/rendering/size/resize-fit-mode#clip>
    #[default]
    Clip,
}

/// Output width and height are floored at one.
fn clip_fit_dimensions(
    input_width: u32,
    input_height: u32,
    width: Option<ImageDimension>,
    height: Option<ImageDimension>,
) -> (u32, u32) {
    let input_width = max(1, input_width);
    let input_height = max(1, input_height);
    let input_aspect_ratio = f64::from(input_width) / f64::from(input_height);
    match (width, height) {
        (Some(ImageDimension(width_value)), Some(ImageDimension(height_value))) => {
            let aspect_ratio = f64::from(width_value) / f64::from(height_value);
            // requested dimensions are for image taller than input
            if aspect_ratio < input_aspect_ratio {
                let output_width = min(width_value, input_width);
                (
                    output_width,
                    #[expect(clippy::cast_possible_truncation)]
                    #[expect(clippy::cast_sign_loss)]
                    {
                        max(
                            1,
                            (f64::from(output_width) / input_aspect_ratio).round() as u32,
                        )
                    },
                )
            }
            // requested dimensions are for image wider than input
            else {
                let output_height = min(height_value, input_height);
                (
                    #[expect(clippy::cast_possible_truncation)]
                    #[expect(clippy::cast_sign_loss)]
                    {
                        max(
                            1,
                            (input_aspect_ratio * f64::from(output_height)).round() as u32,
                        )
                    },
                    output_height,
                )
            }
        }
        (Some(ImageDimension(width_value)), None) => (
            width_value,
            #[expect(clippy::cast_possible_truncation)]
            #[expect(clippy::cast_sign_loss)]
            {
                max(
                    1,
                    (f64::from(width_value) / input_aspect_ratio).round() as u32,
                )
            },
        ),
        (None, Some(ImageDimension(height_value))) => (
            #[expect(clippy::cast_possible_truncation)]
            #[expect(clippy::cast_sign_loss)]
            {
                max(
                    1,
                    (input_aspect_ratio * f64::from(height_value)).round() as u32,
                )
            },
            height_value,
        ),
        (None, None) => (input_width, input_height),
    }
}

#[must_use]
fn output_dimensions(
    input_width: u32,
    input_height: u32,
    width: Option<ImageDimension>,
    height: Option<ImageDimension>,
    fit: Option<FitMode>,
) -> (u32, u32) {
    match fit {
        Some(FitMode::Clip) => clip_fit_dimensions(input_width, input_height, width, height),
        other => unimplemented!("Fit Mode {other:?} not yet implemented"),
    }
}

#[derive(Clone, Debug)]
struct ImageDimension(u32);

impl ImageDimension {
    pub fn new(raw_dimension: u32) -> Result<Self, ImageDimensionError> {
        if raw_dimension > 0 {
            Ok(Self(raw_dimension))
        } else {
            Err(ImageDimensionError(raw_dimension))
        }
    }
}

pub fn resize_image(
    input_image: &DynamicImage,
    output_width: Option<u32>,
    output_height: Option<u32>,
    fit: Option<FitMode>,
) -> Result<DynamicImage, ImageDimensionError> {
    let output_width = if let Some(output_width) = output_width {
        Some(ImageDimension::new(output_width)?)
    } else {
        None
    };
    let output_height = if let Some(output_height) = output_height {
        Some(ImageDimension::new(output_height)?)
    } else {
        None
    };
    let (resized_width, resized_height) = output_dimensions(
        input_image.width(),
        input_image.height(),
        output_width,
        output_height,
        fit,
    );

    // Use `FilterType::Triangle` for faster computation (`FilterType::Lanczos3` should offer a
    // higher quality reduction, but quality is less important for determining the dominant colour).
    Ok(ImageRgba8(imageops::resize(
        input_image,
        resized_width,
        resized_height,
        FilterType::Triangle,
    )))
}

/// Generates an image data URI embedding a base 64 representation of the image for use in
/// web contexts.
///
/// This function does not check that the provided mime type is valid, or that it matches the raw
/// bytes provided in `image_bytes`.
///
/// # Arguments
///
/// * `image_bytes` - Raw bytes of the image to create the data URI for.
/// * `mime_type` - Image [MIME type](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/MIME_types/Common_types).
///
/// # Returns
///
/// Image data URI.
///
/// # Examples
///
/// ```
/// # use axum_inertia_svelte_server::domain::image::utilities::image_to_data_uri;
/// # use std::{fs::File,io::{BufReader,Read}};
/// #
/// # fn bleh() -> std::io::Result<()> {
/// let mut buffer: Vec<u8> = Vec::new();
/// let f = File::open("src/domain/image/fixtures/fixture.jpg")?;
/// let mut reader = BufReader::new(f);
/// reader.read_to_end(&mut buffer)?;
///
/// let data_uri = image_to_data_uri(&buffer, "image/jpeg");
///
/// assert_eq!(&data_uri[..32], "data:image/jpeg;base64,/9j/4QDeR");
/// # Ok(())
/// # }
/// # bleh().unwrap();
/// ```
#[must_use]
pub fn image_to_data_uri(image_bytes: &[u8], mime_type: &str) -> String {
    let mut base64 = String::new();
    STANDARD_NO_PAD.encode_string(image_bytes, &mut base64);

    format!("data:{mime_type};base64,{base64}")
}

#[cfg(test)]
mod tests {
    use std::{fs::read, path::PathBuf};

    use image::ImageFormat;
    use proptest::proptest;

    use crate::{
        errors::{AppError, IoError},
        utilities::image::{
            FitMode, ImageDimension, clip_fit_dimensions, image_to_data_uri, load_image,
            output_dimensions, resize_image,
        },
    };

    #[test]
    fn load_image_returns_expected_output() {
        // arrange
        let image = read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/utilities/fixtures/fixture.jpg"),
        )
        .unwrap();

        // act
        let outcome = load_image(&image).unwrap();

        // assert
        let (format, image) = outcome;
        assert!(matches!(format, ImageFormat::Jpeg));
        assert_eq!(image.width(), 1600);
        assert_eq!(image.height(), 900);
    }

    #[test]
    fn clip_fit_dimensions_maintains_aspect_ratio_on_taller_requested_aspect_ratio() {
        // arrange
        let input_width = 300;
        let input_height = 200;
        let width = Some(ImageDimension::new(100).unwrap());
        let height = Some(ImageDimension::new(100).unwrap());

        // act
        let outcome = clip_fit_dimensions(input_width, input_height, width, height);

        // assert
        assert_eq!(outcome, (100, 67));
    }

    #[test]
    fn clip_fit_dimensions_maintains_aspect_ratio_on_wider_requested_aspect_ratio() {
        // arrange
        let input_width = 300;
        let input_height = 200;
        let width = Some(ImageDimension::new(200).unwrap());
        let height = Some(ImageDimension::new(100).unwrap());

        // act
        let outcome = clip_fit_dimensions(input_width, input_height, width, height);

        // assert
        assert_eq!(outcome, (150, 100));
    }

    #[test]
    fn clip_fit_dimensions_maintains_aspect_ratio_on_no_width() {
        // arrange
        let input_width = 300;
        let input_height = 200;
        let width = None;
        let height = Some(ImageDimension::new(150).unwrap());

        // act
        let outcome = clip_fit_dimensions(input_width, input_height, width, height);

        // assert
        assert_eq!(outcome, (225, 150));
    }

    #[test]
    fn clip_fit_dimensions_returns_inputs_on_no_height() {
        // arrange
        let input_width = 300;
        let input_height = 200;
        let width = Some(ImageDimension::new(75).unwrap());
        let height = None;

        // act
        let outcome = clip_fit_dimensions(input_width, input_height, width, height);

        // assert
        assert_eq!(outcome, (75, 50));
    }

    #[test]
    fn clip_fit_dimensions_returns_inputs_on_no_width_or_height() {
        // arrange
        let input_width = 300;
        let input_height = 200;
        let width = None;
        let height = None;

        // act
        let outcome = clip_fit_dimensions(input_width, input_height, width, height);

        // assert
        assert_eq!(outcome, (300, 200));
    }

    #[test]
    fn clip_fit_dimensions_returns_expected_value_for_low_aspect_ratio_input_images() {
        // arrange
        let input_width = 1;
        let input_height = u32::MAX;
        let width = Some(ImageDimension::new(100).unwrap());
        let height = Some(ImageDimension::new(100).unwrap());

        // act
        let outcome = clip_fit_dimensions(input_width, input_height, width, height);

        // assert
        assert_eq!(outcome, (1, 100));
    }

    #[test]
    fn clip_fit_dimensions_returns_expected_value_for_large_aspect_ratio_input_images() {
        // arrange
        let input_width = u32::MAX;
        let input_height = 1;
        let width = Some(ImageDimension::new(100).unwrap());
        let height = Some(ImageDimension::new(100).unwrap());

        // act
        let outcome = clip_fit_dimensions(input_width, input_height, width, height);

        // assert
        assert_eq!(outcome, (100, 1));
    }

    proptest! {
        #[test]
        fn clip_fit_dimensions_handles_valid_input(
            input_width in 1..=4_294_967_295_u32,
            input_height in 1..=4_294_967_295_u32,
            width in 1..=4_294_967_295_u32,
            height in 1..=4_294_967_295_u32,
        ) {
        // arrange
        let width = ImageDimension::new(width).unwrap();
        let height = ImageDimension::new(height).unwrap();

        // act
        let (output_width, output_height) =
            clip_fit_dimensions(input_width, input_height, Some(width), Some(height));

        // assert
        assert!(output_width > 0);
        assert!(output_height > 0);

        let input_aspect_ratio = f64::from(input_width) / f64::from(input_height);
        let output_aspect_ratio = f64::from(output_width) / f64::from(output_height);
        assert!(
            (input_aspect_ratio - output_aspect_ratio).abs()
                < (f64::from(output_width) + f64::from(output_height))
                    / (f64::from(output_height) * (f64::from(output_height) - 1.0))
        );
        }
    }

    #[test]
    fn output_dimensions_returns_value_on_valid_input() {
        // arrange
        let input_width = 300;
        let input_height = 200;
        let width = None;
        let height = None;

        // act
        let outcome = output_dimensions(
            input_width,
            input_height,
            width,
            height,
            Some(FitMode::Clip),
        );

        // assert
        assert_eq!(outcome, (300, 200));
    }

    #[test]
    fn load_image_returns_value_on_valid_input() -> Result<(), AppError> {
        // arrange
        let image_bytes = read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/utilities/fixtures/fixture.png"),
        )
        .map_err(IoError::from)?;
        let (_, image) = load_image(&image_bytes)?;

        // act
        let outcome = resize_image(&image, Some(75), Some(75), Some(FitMode::Clip)).unwrap();

        // assert
        assert_eq!(outcome.width(), 75);
        assert_eq!(outcome.height(), 42);

        Ok(())
    }

    #[test]
    fn image_to_data_uri_returns_value_on_valid_input() {
        // arrange
        let image_bytes = read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/utilities/fixtures/75x50.svg"),
        )
        .map_err(IoError::from)
        .unwrap();

        // act
        let outcome = image_to_data_uri(&image_bytes, "image/svg+xml");

        // assert
        insta::assert_snapshot!(outcome);
    }
}
