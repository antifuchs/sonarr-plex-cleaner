//! Clean up a plex&sonarr library by deleting completed&fully watched seasons.
//!
//! If you use Sonarr, you might be pleased by how good it is at
//! downloading data. Unfortunately, it's not as good at cleaning up
//! that data you no longer need (somewhat because sonarr doesn't know
//! when you no longer need that data!).
//!
//! This tool exists to fill that gap: It queries both sonarr (the
//! thing downloading media) and plex (the thing keeping track of
//! whether you watched that media), and deletes everything that has
//! been fully downloaded and watched.

// Documentation used:
//  Sonarr: https://github.com/Sonarr/Sonarr/wiki/API
//  Plex: https://support.plex.tv/articles/201638786-plex-media-server-url-commands/

use byte_unit::{Byte, ByteUnit};
use chrono::{Duration, Utc};
use log::*;
use std::collections::HashMap;
use std::collections::HashSet;
use structopt::StructOpt;

mod plex;
mod sonarr;

use self::{plex::*, sonarr::*};

#[derive(StructOpt, Debug)]
#[structopt(name = "sonarr-plex-cleaner")]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
/// Clean up a TV "broadcatching" library by deleting seasons that
/// have been completed downloaded in Sonarr and fully watched in
/// Plex.
struct Opt {
    /// The URL (including /api/) to your Sonarr installation.
    #[structopt(long = "sonarr")]
    sonarr_url: String,

    /// The Sonarr API key (from Settings -> General)
    #[structopt(long = "sonarr-api-key")]
    sonarr_api_key: String,

    /// The Plex auth token --- see Plex help: https://bit.ly/2p7RtOu
    #[structopt(long = "plex-api-key")]
    plex_api_key: String,

    /// The URL to your Plex installation.
    #[structopt(long = "plex")]
    plex_url: String,

    /// A tag name that causes a series to not be eligible for collection.
    #[structopt(long = "retain", default_value = "retain")]
    retain_tag: String,

    /// Whether to actually delete files.
    #[structopt(long = "delete-files", short = "f")]
    delete_files: bool,
}

fn main() {
    flexi_logger::Logger::with_env_or_str("info")
        .print_message()
        .start()
        .unwrap();

    let opt = Opt::from_args();
    let sonarr =
        SonarrClient::new(opt.sonarr_url, opt.sonarr_api_key).expect("sonarr client setup");
    let tags = sonarr.fetch_tags().expect("sonarr tags");
    let retain_tag = tags.get(opt.retain_tag);
    let plex = PlexClient::new(opt.plex_url, opt.plex_api_key).expect("plex client setup");

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
            if let Some(tag) = retain_tag {
                if series.tags.contains(&tag.id) {
                    debug!("Skipping {} because tagged {:?}", series.title, tag.label);
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
                        .map(|air| air + Duration::days(60) < Utc::now())
                        .unwrap_or(false);
                    if still_airing || !old_enough {
                        // season isn't done airing yet / isn't old enough:
                        return None;
                    }

                    let is_watched = watched_seasons
                        .get(&(
                            series.title.clone(),
                            format!("Season {}", season.season_number),
                        ))
                        .is_none();
                    if is_watched {
                        // season isn't fully watched on plex yet:
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
            if opt.delete_files {
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
