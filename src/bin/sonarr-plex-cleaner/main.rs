//! Main entry point for Sonarr Plex Cleaner Cli

#![deny(warnings, missing_docs, trivial_casts, unused_qualifications)]
#![forbid(unsafe_code)]

use sonarr_plex_cleaner::application::APPLICATION;

/// Boot Sonarr Plex Cleaner Cli
fn main() {
    abscissa_core::boot(&APPLICATION);
}
