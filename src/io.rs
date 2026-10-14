use std::{
    fs::{self, read},
    path::Path,
};

use humansize::format_size;

use crate::errors::IoError;

const MAX_INPUT_IMAGE_SIZE: u64 = 16 * 1_024 * 1_024; // 16 mebibytes

/// Read a file into a buffer.
///
/// Adds a limit on maximum file size which although low, serves current use cases;
/// consider increasing if required.
pub fn open_file<P: AsRef<Path>>(path: P, max_input_size: Option<u64>) -> Result<Vec<u8>, IoError> {
    let max_input_size = max_input_size.unwrap_or(MAX_INPUT_IMAGE_SIZE);
    let metadata = fs::metadata(&path).map_err(|err| {
        IoError::new(
            &err,
            &format!("reading {} metadata", path.as_ref().display()),
        )
    })?;
    if metadata.len() > max_input_size {
        Err(IoError {
            advice: format!(
                "Try optimising the file `{}` to below {}",
                path.as_ref().display(),
                format_size(max_input_size, humansize::BINARY),
            ),
            detail: format!("File size of {} exceeds maximum input size", metadata.len()),
        })?;
    }

    read(path).map_err(IoError::from)
}

#[cfg(test)]
mod tests {
    use assert_fs::fixture::{FileWriteStr, PathChild};
    use miette::Diagnostic;

    use crate::open_file;

    #[test]
    fn open_file_successfully_loads_data() {
        // arrange
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let text = "Lorem Ipsum";
        let _ = temp_dir.child("file.txt").write_str(text);
        let path = temp_dir.join("file.txt");

        // act
        let outcome = open_file(&path, None).unwrap();

        // assert
        assert_eq!(&outcome, text.as_bytes());
    }

    #[test]
    fn open_file_returns_error_for_large_file() {
        // arrange
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let text = "Lorem Ipsum";
        let _ = temp_dir.child("file.txt").write_str(text);
        let path = temp_dir.join("file.txt");

        // act
        let outcome = open_file(&path, Some(8)).unwrap_err();

        // assert
        assert_eq!(
            &outcome.to_string(),
            "File size of 11 exceeds maximum input size"
        );

        let help = outcome.help();
        assert_eq!(
            format!("{}", help.unwrap()),
            format!("Try optimising the file `{}` to below 8 B", path.display())
        );
    }

    #[test]
    fn open_file_returns_error_for_file_not_found() {
        // arrange
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let path = temp_dir.join("does-not-exist.txt");

        // act
        let outcome = open_file(&path, Some(8)).unwrap_err();

        // assert
        assert_eq!(
            outcome.to_string(),
            format!(
                "No such file or directory (os error 2) while reading {} metadata",
                path.display()
            )
        );

        let help = outcome.help();
        assert_eq!(
            format!("{}", help.unwrap()),
            String::from("Check path exists")
        );
    }
}
