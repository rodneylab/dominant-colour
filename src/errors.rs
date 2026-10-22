use std::io;

use image::error::ImageFormatHint;

/// Input/output error
#[derive(Debug, miette::Diagnostic, thiserror::Error)]
#[error("{detail}")]
pub struct IoError {
    /// User-focused remedial suggestion
    #[help]
    pub advice: String,

    /// Error detail
    pub detail: String,
}

impl From<io::Error> for IoError {
    fn from(value: io::Error) -> Self {
        match value.kind() {
            io::ErrorKind::NotFound => Self {
                advice: "Check the path exists".to_owned(),
                detail: value.to_string(),
            },
            io::ErrorKind::PermissionDenied => Self {
                advice: "Check you have access permissions".to_owned(),
                detail: value.to_string(),
            },
            _ => Self {
                advice: "Check the path exists with access permissions".to_owned(),
                detail: value.to_string(),
            },
        }
    }
}

impl IoError {
    pub fn new(error: &io::Error, cause_action: &str) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => Self {
                advice: "Check path exists".to_owned(),
                detail: format!("{error} while {cause_action}"),
            },
            io::ErrorKind::PermissionDenied => Self {
                advice: "Check you have access permissions".to_owned(),
                detail: format!("{error} while {cause_action}"),
            },
            _ => Self {
                advice: "Check the path exists with access permissions".to_owned(),
                detail: format!("{error} while {cause_action}"),
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("dimension must be strictly positive: {0} is not a valid dimension")]
pub struct ImageDimensionError(pub u32);

/// Input/output error
#[derive(Debug, miette::Diagnostic, thiserror::Error)]
#[error("{detail}")]
pub struct ImageError {
    /// User-focused remedial suggestion
    #[help]
    pub advice: String,

    /// Error detail
    pub detail: String,
}

impl From<image::ImageError> for ImageError {
    fn from(value: image::ImageError) -> Self {
        match value {
            image::ImageError::Unsupported(ref error) => {
                let common_advice = String::from(
                    "Try converting the image to AVIF, JPEG, PNG or WebP externally, then \
                    retrying.",
                );
                let advice = if let ImageFormatHint::Exact(format) = error.format_hint() {
                    format!("{format:?} image format is not supported.  {common_advice}")
                } else {
                    format!(
                        "Unable to determine image format, or the format is not supported.  \
                        {common_advice}"
                    )
                };

                Self {
                    advice,
                    detail: value.to_string(),
                }
            }
            _ => Self {
                advice: "Check image file is not corrupted".to_owned(),
                detail: value.to_string(),
            },
        }
    }
}

impl From<mozjpeg_rs::Error> for ImageError {
    fn from(value: mozjpeg_rs::Error) -> Self {
        Self {
            advice: "Check input image file is not corrupted".to_owned(),
            detail: value.to_string(),
        }
    }
}

impl From<ImageDimensionError> for ImageError {
    fn from(value: ImageDimensionError) -> Self {
        Self {
            advice: "Check input width and height values".to_owned(),
            detail: value.to_string(),
        }
    }
}

impl From<oxipng::PngError> for ImageError {
    fn from(value: oxipng::PngError) -> Self {
        Self {
            advice: "Try to repair the input image file".to_owned(),
            detail: value.to_string(),
        }
    }
}

impl From<askama::Error> for ImageError {
    fn from(value: askama::Error) -> Self {
        Self {
            advice: "Check template file exists".to_owned(),
            detail: value.to_string(),
        }
    }
}

/// Global app error
#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum AppError {
    /// App image error
    #[diagnostic(transparent)]
    #[error(transparent)]
    Image(#[from] ImageError),

    /// App input/output error
    #[diagnostic(transparent)]
    #[error(transparent)]
    Io(#[from] IoError),
}

#[cfg(test)]
mod tests {

    use std::io::ErrorKind;

    use image::{
        ImageFormat,
        error::{DecodingError, ImageFormatHint},
    };
    use miette::Diagnostic;

    use crate::errors::{ImageDimensionError, ImageError, IoError};

    #[test]
    fn io_error_generates_expected_help_message() {
        // arrange
        let error = IoError {
            advice: String::from("Try freeing up some storage space."),
            detail: String::from("Something went wrong."),
        };

        // act
        let help = error.help();

        // assert
        assert_eq!(&error.to_string(), "Something went wrong.");
        assert_eq!(
            &format!("{}", help.unwrap()),
            "Try freeing up some storage space."
        );
    }

    #[test]
    fn io_error_from_std_io_error_generates_expected_help_message() {
        // arrange
        let error = IoError::from(std::io::Error::from(ErrorKind::NotFound));

        // act
        let help = error.help();

        // assert
        assert_eq!(&error.to_string(), "entity not found");
        assert_eq!(&format!("{}", help.unwrap()), "Check the path exists");
    }

    #[test]
    fn image_error_from_image_image_error_generates_expected_help_message() {
        // arrange
        let error = ImageError::from(image::error::ImageError::Decoding(
            DecodingError::from_format_hint(ImageFormatHint::Exact(ImageFormat::Jpeg)),
        ));

        // act
        let help = error.help();

        // assert
        assert_eq!(&error.to_string(), "Format error decoding Jpeg");
        assert_eq!(
            &format!("{}", help.unwrap()),
            "Check image file is not corrupted"
        );
    }

    #[test]
    fn image_error_from_mozjpeg_error_generates_expected_help_message() {
        // arrange
        let error = ImageError::from(mozjpeg_rs::Error::InvalidDimensions {
            width: 0,
            height: 96,
        });

        // act
        let help = error.help();

        // assert
        assert_eq!(&error.to_string(), "Invalid image dimensions: 0x96");
        assert_eq!(
            &format!("{}", help.unwrap()),
            "Check input image file is not corrupted"
        );
    }

    #[test]
    fn image_error_from_image_dimension_error_generates_expected_help_message() {
        // arrange
        let error = ImageError::from(ImageDimensionError(0));

        // act
        let help = error.help();

        // assert
        assert_eq!(
            &error.to_string(),
            "dimension must be strictly positive: 0 is not a valid dimension"
        );
        assert_eq!(
            &format!("{}", help.unwrap()),
            "Check input width and height values"
        );
    }

    #[test]
    fn image_error_from_oxipng_error_generates_expected_help_message() {
        // arrange
        let error = ImageError::from(oxipng::PngError::NotPNG);

        // act
        let help = error.help();

        // assert
        assert_eq!(
            &error.to_string(),
            "Invalid header detected; Not a PNG file"
        );
        assert_eq!(
            &format!("{}", help.unwrap()),
            "Try to repair the input image file"
        );
    }

    #[test]
    fn image_error_from_askama_error_generates_expected_help_message() {
        // arrange
        let error = ImageError::from(askama::Error::Fmt);

        // act
        let help = error.help();

        // assert
        assert_eq!(
            &error.to_string(),
            "an error occurred when formatting an argument"
        );
        assert_eq!(&format!("{}", help.unwrap()), "Check template file exists");
    }
}
