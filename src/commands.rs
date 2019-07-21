//! Sonarr Plex Cleaner Cli Subcommands
//!
//! This is where you specify the subcommands of your application.
//!
//! The default application comes with two subcommands:
//!
//! - `start`: launches the application
//! - `version`: print application version
//!
//! See the `impl Configurable` below for how to specify the path to the
//! application's configuration file.

mod tv;
mod version;

use self::{tv::TVCommand, version::VersionCommand};
use crate::config::SonarrPlexCleanerCliConfig;
use abscissa_core::config::Override;
use abscissa_core::{Command, Configurable, FrameworkError, Help, Options, Runnable};
use dirs::{config_dir, home_dir};
use std::path::PathBuf;

/// Sonarr Plex Cleaner config file
pub const CONFIG_FILE: &str = "sonarr-plex-cleaner.toml";

/// Sonarr Plex Cleaner Cli Subcommands
#[derive(Command, Debug, Options, Runnable)]
pub enum SonarrPlexCleanerCliCommand {
    /// The `help` subcommand
    #[options(help = "get usage information")]
    Help(Help<Self>),

    /// The `tv` subcommand for TV seasons
    #[options(help = "clean up TV seasons in sonarr&plex")]
    Tv(TVCommand),

    /// The `version` subcommand
    #[options(help = "display version information")]
    Version(VersionCommand),
}

/// This trait allows you to define how application configuration is loaded.
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
