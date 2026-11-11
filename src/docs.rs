use std::{fs::create_dir_all, path::Path};

use crate::{Cli, errors::IoError};

/// Write the CLI documentation to a file
pub fn write_markdown_docs_to_file<P: AsRef<Path>>(path: P) -> Result<(), IoError> {
    let markdown = clap_markdown::help_markdown::<Cli>();

    let directory = path.as_ref().parent().ok_or(IoError {
        advice: format!(
            "Check the documentation path `{}` is valid",
            path.as_ref().display()
        ),
        detail: String::from("Unable to determine docs directory"),
    })?;

    create_dir_all(directory).map_err(|err| IoError {
        advice: format!(
            "Check you have write access to markdown documentation directory `{}`",
            directory.display()
        ),
        detail: err.to_string(),
    })?;
    std::fs::write(&path, markdown).map_err(|err| IoError {
        advice: format!(
            "Check you have write access to markdown documentation path `{}`",
            path.as_ref().display()
        ),
        detail: err.to_string(),
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::docs::write_markdown_docs_to_file;

    #[test]
    fn write_markdown_docs_to_file_generates_file_at_expected_path() {
        // arrange
        let temp_dir = assert_fs::TempDir::new().unwrap();
        let path = temp_dir.join("help.md");

        // act
        let outcome = write_markdown_docs_to_file(&path);

        // assert
        assert!(outcome.is_ok());
        assert!(path.is_file());
    }
}
