mod styles;

use std::path::PathBuf;

use clap::Parser;
use clap_verbosity_flag::Verbosity;

use crate::cli::styles::CARGO_STYLING;

#[derive(Parser)]
#[group(multiple = false)]
pub struct OutputArgs {
    /// Write generated image to path
    #[clap(short, long)]
    pub output: Option<PathBuf>,

    /// Copy generated URI to clipboard
    #[clap(short, long, action)]
    pub clipboard: bool,
}

#[derive(Parser)]
pub struct GenerateArgs {
    /// Path to input image file.
    pub input: PathBuf,

    #[clap(flatten)]
    pub output: OutputArgs,

    /// Create an SVG, instead of preserving the input format
    #[clap(long)]
    pub svg: bool,
}

/// App commands
#[derive(clap::Subcommand)]
pub enum Commands {
    /// Generate a placeholder
    Generate(GenerateArgs),

    /// Generate CLI documentation
    #[cfg(feature = "internal-tools")]
    #[command(hide = true)]
    MarkdownHelp,
}

/// App CLI arguments
#[derive(Parser)]
#[clap(author,version,about,long_about=None)]
#[command(styles=CARGO_STYLING)]
pub struct Cli {
    /// Command
    #[command(subcommand)]
    pub command: Commands,

    /// verbosity -v
    #[clap(flatten)]
    pub verbose: Verbosity,
}

impl Cli {
    /// Configure `env_logger` logging using verbosity flags provided in CLI arguments.
    ///
    /// Initialises a global logger, and should be called once at most.
    pub fn initialise_logging(&self) {
        env_logger::Builder::new()
            .filter_level(self.verbose.log_level_filter())
            .init();
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use clap::Parser;
    use log::Level;

    use crate::cli::Cli;

    /// Parses CLI arguments and returns a `Cli` instance.
    fn parse_args<I, T>(args: I) -> Cli
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString> + Clone,
    {
        Cli::try_parse_from(args).expect("Failed to parse CLI arguments")
    }

    #[test]
    fn initialise_logging_uses_env_log_level() {
        // arrange
        let args = vec!["program", "generate", "input.png", "-v"];
        let cli = parse_args(&args);

        // act
        cli.initialise_logging();

        // assert
        assert!(log::log_enabled!(Level::Error));
        assert!(log::log_enabled!(Level::Warn));
        assert!(!log::log_enabled!(Level::Info));
        assert!(!log::log_enabled!(Level::Debug));
        assert!(!log::log_enabled!(Level::Trace));
    }
}
