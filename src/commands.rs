//! Sonarr Plex Cleaner CLI Subcommands

mod tv;
mod version;

use self::{tv::TVCommand, version::VersionCommand};
use crate::config::SonarrPlexCleanerCliConfig;
use abscissa_core::config::Override;
use abscissa_core::{Command, Configurable, FrameworkError, Help, Options, Runnable};
use dirs::{config_dir, home_dir};
use std::path::PathBuf;

/// Sonarr Plex Cleaner config file name.
pub const CONFIG_FILE: &str = "sonarr-plex-cleaner.toml";

/// Sonarr Plex Cleaner Cli Subcommands
#[derive(Command, Debug, Options, Runnable)]
pub enum SonarrPlexCleanerCliCommand {
    /// The `help` subcommand
    #[options(help = "get usage information")]
    Help(Help<Self>),

    /// The `tv` subcommand for cleaning out watched TV seasons
    #[options(help = "clean up TV seasons in sonarr&plex")]
    Tv(TVCommand),

    /// The `version` subcommand
    #[options(help = "display version information")]
    Version(VersionCommand),
}

/// The way we load the CLI file:
///
/// The config file is mandatory, and we search for it in the OS's
/// appropriate `config_dir` (if unknown, the `home_dir`).
impl Configurable<SonarrPlexCleanerCliConfig> for SonarrPlexCleanerCliCommand {
    /// Location of the configuration file
    fn config_path(&self) -> Option<PathBuf> {
        // Tool must be run with a config file in place.
        Some(PathBuf::from(
            config_dir()
                .or_else(home_dir)
                .expect("user home and config dir are unknown")
                .join(CONFIG_FILE),
        ))
    }

    /// Override config settings from the commandline.
    fn process_config(
        &self,
        config: SonarrPlexCleanerCliConfig,
    ) -> Result<SonarrPlexCleanerCliConfig, FrameworkError> {
        match self {
            SonarrPlexCleanerCliCommand::Tv(cmd) => cmd.override_config(config),
            _ => Ok(config),
        }
    }
}
