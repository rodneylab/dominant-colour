//! # dominant-colour
//! `dominant-colour` Generate dominant colour base64 data uris for use as website image placeholders

mod cli;
mod errors;
mod io;
mod placeholder;
mod utilities;

#[cfg(feature = "internal-tools")]
mod docs;

use std::{
    fs,
    io::{BufWriter, Write},
};

use askama::Template;
use clap::Parser;
use image::{DynamicImage, Rgb};
use palette::{IntoColor, Lab, Srgb, cast::from_component_vec};

use crate::{
    cli::{Cli, Commands, GenerateArgs},
    errors::{AppError, ImageError, IoError},
    io::open_file,
    placeholder::{
        DominantColourOptions, SvgPlaceholder, compute_dominant_colour, compute_kmeans,
        generate_placeholder_bytes,
    },
    utilities::{
        image::{FitMode, image_to_data_uri, load_image, resize_image},
        ui::copy_text_to_clipboard,
    },
};

const PALETTE_EMOJI: &str = "\u{1f3a8}";

fn handle_output<W: Write>(
    writer: &mut W,
    bytes: &[u8],
    mime_type: &str,
    clipboard: bool,
) -> Result<(), IoError> {
    let data_uri = image_to_data_uri(bytes, mime_type);
    if clipboard {
        copy_text_to_clipboard(&data_uri, Some("image data uri"));
    } else {
        writeln!(writer, "{data_uri}")
            .map_err(|err| IoError::new(&err, "writing generated data-uri to stdout"))?;
        writer
            .flush()
            .map_err(|err| IoError::new(&err, "outputting generated data-uri to stdout"))?;
    }

    Ok(())
}

fn determine_dominant_colour(image: &DynamicImage) -> Result<Rgb<u8>, ImageError> {
    let rgb8_image: Vec<u8> = image.to_rgb8().into_raw();
    let lab: Vec<Lab> = from_component_vec::<Srgb<u8>>(rgb8_image)
        .iter()
        .map(|x| x.into_linear().into_color())
        .collect();

    let kmeans = compute_kmeans(&DominantColourOptions::default(), &lab);

    compute_dominant_colour(&kmeans)
}

fn handle_svg_output<W: Write>(
    writer: &mut W,
    generate_args: &GenerateArgs,
    dominant_colour: Rgb<u8>,
    width: u32,
    height: u32,
) -> Result<(), AppError> {
    let dominant_colour_hex = format!(
        "#{:0x}{:0x}{:0x}",
        dominant_colour[0], dominant_colour[1], dominant_colour[2]
    );
    let svg_placeholder = SvgPlaceholder {
        width,
        height,
        dominant_colour: dominant_colour_hex,
    };
    let svg_content = svg_placeholder.render().map_err(ImageError::from)?;
    if let Some(output_path) = &generate_args.output.output {
        let output_path = output_path.with_extension("svg");
        fs::write(&output_path, svg_content.as_bytes())
            .map_err(|err| IoError::new(&err, &format!("writing to {}", output_path.display())))?;
        log::info!("Wrote output to \"{}\"", output_path.display());
    } else {
        handle_output(
            writer,
            svg_content.as_bytes(),
            "image/svg+xml",
            generate_args.output.clipboard,
        )?;
    }

    Ok(())
}

fn generate_placeholder<W: Write>(
    writer: &mut W,
    generate_args: &GenerateArgs,
) -> Result<(), AppError> {
    let image_bytes = open_file(&generate_args.input, None)?;
    let (format, image) = load_image(&image_bytes)?;
    log::info!("Loaded {format:?} image");
    let resized_image = resize_image(&image, Some(100), Some(100), Some(FitMode::Clip))
        .map_err(ImageError::from)?;
    let width = resized_image.width();
    let height = resized_image.height();
    let dominant_colour = determine_dominant_colour(&resized_image)?;

    if generate_args.svg {
        handle_svg_output(writer, generate_args, dominant_colour, width, height)?;
    } else {
        let output_bytes = generate_placeholder_bytes(dominant_colour, width, height, format)?;

        if let Some(output_path) = &generate_args.output.output {
            fs::write(output_path, &output_bytes).map_err(|err| {
                IoError::new(&err, &format!("writing to {}", output_path.display()))
            })?;
            log::info!(r#"Wrote output to "{}""#, output_path.display());
        } else {
            handle_output(
                writer,
                &output_bytes,
                format.to_mime_type(),
                generate_args.output.clipboard,
            )?;
        }
    }

    Ok(())
}

fn main() -> Result<(), miette::Error> {
    let cli = Cli::parse();

    let stdout = std::io::stdout();
    let mut stdout_handle = BufWriter::new(stdout.lock());

    match cli.command {
        Commands::Generate(ref generate_args) => {
            cli.initialise_logging();
            log::info!("{PALETTE_EMOJI} dominant_colour");

            Ok(generate_placeholder(&mut stdout_handle, generate_args)?)
        }

        #[cfg(feature = "internal-tools")]
        Commands::MarkdownHelp => {
            docs::write_markdown_docs_to_file("docs/help.md")?;

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs::{read, read_to_string},
        io::Cursor,
        path::PathBuf,
    };

    use image::{ImageFormat, ImageReader};
    use miette::Diagnostic;

    use crate::{
        cli::{GenerateArgs, OutputArgs},
        determine_dominant_colour,
        errors::IoError,
        generate_placeholder, handle_output,
    };

    #[test]
    fn handle_output_write_output_to_buffer() {
        // arrange
        let mut writer = Vec::<u8>::new();
        let bytes =
            read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/482-80x45.jpg"))
                .map_err(IoError::from)
                .unwrap();
        let mime_type = "image/jpeg";

        // act
        let outcome = handle_output(&mut writer, &bytes, mime_type, false);

        // assert
        assert!(outcome.is_ok());
        insta::assert_snapshot!(&String::from_utf8_lossy(&writer));
    }

    #[test]
    fn determine_dominant_colour_returns_expected_value_for_valid_input() {
        // arrange
        let bytes =
            read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/482-80x45.jpg"))
                .map_err(IoError::from)
                .unwrap();
        let image = ImageReader::with_format(Cursor::new(bytes), ImageFormat::Jpeg)
            .decode()
            .unwrap();

        // act
        let outcome = determine_dominant_colour(&image).unwrap();

        // assert
        assert_eq!(outcome.0, [82, 64, 46]);
    }

    #[test]
    fn generate_placeholder_produces_expected_svg_output() {
        // arrange
        let mut writer = Vec::<u8>::new();
        let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/482-80x45.jpg");
        let output_args = OutputArgs {
            output: None,
            clipboard: false,
        };
        let generate_args = GenerateArgs {
            input,
            output: output_args,
            svg: true,
        };

        // act
        let outcome = generate_placeholder(&mut writer, &generate_args);

        // assert
        assert!(outcome.is_ok());
        insta::assert_snapshot!(&String::from_utf8_lossy(&writer));
    }

    #[test]
    fn generate_placeholder_does_not_write_to_stdout_when_clipboard_is_true() {
        // arrange
        let mut writer = Vec::<u8>::new();
        let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/482-80x45.jpg");
        let output_args = OutputArgs {
            output: None,
            clipboard: true,
        };
        let generate_args = GenerateArgs {
            input,
            output: output_args,
            svg: true,
        };

        // act
        let outcome = generate_placeholder(&mut writer, &generate_args);

        // assert
        assert!(outcome.is_ok());
        assert_eq!(writer, [] as [u8; 0]);
    }

    #[test]
    fn generate_placeholder_produces_expected_svg_file() {
        // arrange
        let mut writer = Vec::<u8>::new();
        let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/482-80x45.jpg");
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let output_path = temp_dir.join("output.svg");
        let output_args = OutputArgs {
            output: Some(output_path.clone()),
            clipboard: false,
        };
        let generate_args = GenerateArgs {
            input,
            output: output_args,
            svg: true,
        };

        // act
        let outcome = generate_placeholder(&mut writer, &generate_args);

        // assert
        assert!(outcome.is_ok());
        let svg = read_to_string(output_path).unwrap();
        insta::assert_snapshot!(svg);
    }

    #[test]
    fn generate_placeholder_produces_expected_native_format_output() {
        // arrange
        let mut writer = Vec::<u8>::new();
        let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/482-80x45.jpg");
        let output_args = OutputArgs {
            output: None,
            clipboard: false,
        };
        let generate_args = GenerateArgs {
            input,
            output: output_args,
            svg: false,
        };

        // act
        let outcome = generate_placeholder(&mut writer, &generate_args);

        // assert
        assert!(outcome.is_ok());
        insta::assert_snapshot!(&String::from_utf8_lossy(&writer));
    }

    #[test]
    fn generate_placeholder_does_not_write_to_stdout_when_clipboard_is_true_for_native_format_file()
    {
        // arrange
        let mut writer = Vec::<u8>::new();
        let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/482-80x45.jpg");
        let output_args = OutputArgs {
            output: None,
            clipboard: true,
        };
        let generate_args = GenerateArgs {
            input,
            output: output_args,
            svg: false,
        };

        // act
        let outcome = generate_placeholder(&mut writer, &generate_args);

        // assert
        assert!(outcome.is_ok());
        assert_eq!(writer, [] as [u8; 0]);
    }

    #[test]
    fn generate_placeholder_produces_expected_native_format_file() {
        // arrange
        let mut writer = Vec::<u8>::new();
        let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/482-80x45.jpg");
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let output_path = temp_dir.join("output.jpg");
        let output_args = OutputArgs {
            output: Some(output_path.clone()),
            clipboard: false,
        };
        let generate_args = GenerateArgs {
            input,
            output: output_args,
            svg: false,
        };

        // act
        let outcome = generate_placeholder(&mut writer, &generate_args);

        // assert
        assert!(outcome.is_ok());
        let output_bytes = read(output_path).unwrap();
        assert_eq!(
            output_bytes,
            vec![
                255u8, 216, 255, 224, 0, 16, 74, 70, 73, 70, 0, 1, 1, 1, 0, 72, 0, 72, 0, 0, 255,
                219, 0, 132, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 255, 194, 0,
                17, 8, 0, 45, 0, 80, 3, 1, 34, 0, 2, 17, 1, 3, 17, 1, 255, 196, 0, 40, 0, 1, 1, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 7, 255, 218, 0, 12, 3, 1, 0, 2, 16, 3, 16, 0, 0, 0, 130, 226, 63,
                80, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 15, 255, 196, 0, 20, 16, 1, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 80, 255, 218, 0, 8, 1, 1, 0, 1, 63, 0, 115, 255, 196, 0,
                20, 17, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 48, 255, 218, 0, 8, 1, 2,
                1, 1, 63, 0, 127, 255, 196, 0, 20, 17, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 48, 255, 218, 0, 8, 1, 3, 1, 1, 63, 0, 127, 255, 217
            ]
        );
    }

    #[test]
    fn generate_placeholder_returns_error_for_output_directory_not_found() {
        // arrange
        let mut writer = Vec::<u8>::new();
        let input = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/fixtures/482-80x45.jpg");
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let output_path = temp_dir.join("does-not-exist/output.jpg");
        let output_args = OutputArgs {
            output: Some(output_path.clone()),
            clipboard: false,
        };
        let generate_args = GenerateArgs {
            input,
            output: output_args,
            svg: false,
        };

        // act
        let outcome = generate_placeholder(&mut writer, &generate_args).unwrap_err();

        // assert
        assert_eq!(
            outcome.to_string(),
            format!(
                "No such file or directory (os error 2) while writing to {}",
                output_path.display()
            )
        );

        let help = outcome.help();
        assert_eq!(
            format!("{}", help.unwrap()),
            String::from("Check path exists")
        );
    }
}
