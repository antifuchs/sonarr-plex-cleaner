//! `tv` subcommand - cleans out entirely-watched TV seasons.

use crate::config::SonarrPlexCleanerCliConfig;
use crate::prelude::*;

use abscissa_core::config::Override;
use abscissa_core::FrameworkError;

use byte_unit::{Byte, ByteUnit};
use chrono::Utc;
use humantime::{format_duration, Duration};
use std::collections::HashMap;
use std::collections::HashSet;

use crate::services::{plex, sonarr};

use abscissa_core::{
    // config,
    Command,
    // FrameworkError,
    Options,
    Runnable,
};

/// `tv` subcommand - run over a Sonarr-managed plex TV library, find
/// the fully-downloaded, entirely watched seasons and delete them if
/// they're past the retention period.
#[derive(Command, Debug, Options, Default)]
pub struct TVCommand {
    /// Whether to actually delete files.
    #[options(short = "f")]
    delete_files: bool,

    /// How long we should retain a fully-watched season after airing.
    ///
    /// If unset, does not retain anything.
    #[options(no_short)]
    retain_for: Option<Duration>,
}

impl Override<SonarrPlexCleanerCliConfig> for TVCommand {
    fn override_config(
        &self,
        config: SonarrPlexCleanerCliConfig,
    ) -> Result<SonarrPlexCleanerCliConfig, FrameworkError> {
        let mut new_cfg = config.clone();
        if let Some(duration) = self.retain_for {
            new_cfg.retention.retain_duration = *duration;
        }
        Ok(new_cfg)
    }
}

impl Runnable for TVCommand {
    /// Start the application.
    fn run(&self) {
        let config = app_config();

        let sonarr =
            sonarr::SonarrClient::from_config(&config.tv).expect("Could not set up sonarr client");
        let retain_tag = config.retention.retain_tag.as_ref().map(|tag_name| {
            let tags = sonarr.fetch_tags().expect("sonarr tags");
            let tag = tags
                .get(&tag_name)
                .expect(&format!("Tag {:?} not found in {:?}", &tag_name, tags));
            (tag.label.to_string(), tag.id)
        });
        let retain_duration = chrono::Duration::from_std(config.retention.retain_duration)
            .expect("Weird retain duration (past max chrono duration?)");
        let plex =
            plex::PlexClient::from_config(&config.plex).expect("Could not set up plex client");
        let watched_seasons: HashSet<(String, String)> = plex
            .all_tv_seasons()
            .expect("plex season listing")
            .into_iter()
            .filter(|s| s.fully_watched())
            .map(|s| (s.show_name, s.title))
            .collect();

        let serieses = sonarr
            .fetch_all_series()
            .expect("sonarr: fetching serieses");

        let to_delete: HashMap<&sonarr::Series, Vec<&sonarr::Season>> = serieses
            .iter()
            .filter_map(|series| {
                if let Some((name, id)) = &retain_tag {
                    if series.tags.contains(&id) {
                        debug!("Skipping {} because tagged {:?}", series.title, name);
                        return None;
                    }
                }

                let seasons: Vec<&sonarr::Season> = series
                    .seasons
                    .iter()
                    .filter_map(|season| {
                        let still_airing = season.statistics.next_airing.is_some();
                        let old_enough = season
                            .statistics
                            .previous_airing
                            .map(|air| air + retain_duration < Utc::now())
                            .unwrap_or(false);

                        let is_watched = watched_seasons
                            .get(&(
                                series.title.clone(),
                                format!("Season {}", season.season_number),
                            ))
                            .is_none();
                        if is_watched {
                            debug!(
                                "Skipping {} - Season {:?} because unwatched",
                                series.title, season.season_number
                            );
                            return None;
                        }

                        if still_airing {
                            // season isn't done airing yet / isn't old enough:
                            if season.statistics.previous_airing.is_some() {
                                info!(
                                    "Skipping {} - Season {:?} because still airing",
                                    series.title, season.season_number
                                );
                            }
                            return None;
                        }
                        if !old_enough {
                            if let Some(air) = season.statistics.previous_airing {
                                info!(
                                    "Skipping {} - Season {:?} because age:{} < desired:{}",
                                    series.title,
                                    season.season_number,
                                    format_duration(
                                        (Utc::now() - air + retain_duration)
                                            .to_std()
                                            .expect("duration out of range")
                                    ),
                                    format_duration(
                                        retain_duration.to_std().expect("duration out of range")
                                    ),
                                );
                            }
                            return None;
                        }

                        if season.statistics.size_on_disk == 0 {
                            return None;
                        }
                        Some(season)
                    })
                    .collect();
                if seasons.is_empty() {
                    None
                } else {
                    Some((series, seasons))
                }
            })
            .collect();

        for (series, seasons) in to_delete.iter() {
            let series_files = sonarr
                .fetch_episode_files(series.id)
                .expect(&format!("fetching files for {}", series.title));

            for season in seasons {
                let season_files: Vec<&sonarr::EpisodeFile> = series_files
                    .iter()
                    .filter(|f| f.season_number == season.season_number)
                    .collect();
                info!(
                    "delete {} files: {} S{:02}: {}",
                    season_files.len(),
                    series.title,
                    season.season_number,
                    Byte::from_bytes(season.statistics.size_on_disk.into())
                        .get_adjusted_unit(ByteUnit::GiB),
                );
                if self.delete_files {
                    sonarr
                        .unmonitor_season(series.id, season.season_number)
                        .expect(&format!(
                            "Unmonitoring season {} S{:02}",
                            series.title, season.season_number
                        ));
                    for file in season_files.iter() {
                        sonarr
                            .delete_episode_file(file)
                            .expect(&format!("deleting file {:?}", file));
                    }
                }
            }
        }
    }
}
