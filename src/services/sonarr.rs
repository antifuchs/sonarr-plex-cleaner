//! The Sonarr media indexer & broadcatcher API.

use crate::prelude::*;

use chrono::{DateTime, Utc};
use reqwest;
use retry::{delay::Exponential, retry};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

use crate::config;

/// Statistics about a season known to sonarr (via the TV db).
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct SeasonStats {
    /// Number of episodes that have downloaded files.
    pub episode_file_count: u32,

    /// Total episodes that the TV db knows about in the season
    /// (announced episodes).
    pub total_episode_count: u32,

    /// Episodes that have been downloaded.
    pub episode_count: u32,

    /// Time&date that the next episode airs. None if there is no
    /// known next air date.
    pub next_airing: Option<DateTime<Utc>>,

    /// Time & date that the previous episode aired. None if there was
    /// no last-aired episode.
    pub previous_airing: Option<DateTime<Utc>>,

    /// Amount of space in bytes consumed by the season.
    pub size_on_disk: u128,
}

/// A TV show season known to Sonarr.
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Season {
    /// 1-based index of the season.
    ///
    /// That is, `Season 01` is `1`.
    pub season_number: u32,

    /// Whether the season is "monitored" (i.e., still fetches new episodes).
    pub monitored: bool,

    /// Statistics for the season.
    pub statistics: SeasonStats,
}

/// A thing that has a Sonarr API object ID.
pub trait IdEd {
    /// Returns the Sonarr API ID of the object.
    fn id(&self) -> u32;
}

/// A TV show (series).
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    /// Title of the series. E.g., "Zeit im Bild".
    pub title: String,

    /// Sonarr API object ID.
    pub id: u32,

    /// Tags (as Tag ID) associated with the show.
    pub tags: Vec<TagId>,

    /// Seasons known to Sonarr (via the TV metadata DB).
    pub seasons: Vec<Season>,
}

impl IdEd for Series {
    fn id(&self) -> u32 {
        self.id
    }
}

/// A TV show episode (e.g., `S01E03`).
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    /// ID of the TV series this belongs to.
    pub series_id: u32,

    /// ID of the episode itself.
    pub id: u32,

    /// ID of the [`EpisodeFile`] entry for this episode, if there is
    /// a downloaded episode.
    pub episode_file_id: u32,

    /// 1-based index of the season.
    pub season_number: u32,

    /// 1-based number of the episode.
    pub episode_number: u32,

    /// Title of the episode.
    pub title: String,

    /// Whether the episode has an `EpisodeFile`.
    pub has_file: bool,

    /// True if the episode is "monitored" in sonarr.
    pub monitored: bool,
}

/// A file associated with an episode in Sonarr.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeFile {
    /// API object ID.
    pub id: u32,

    /// ID of the series.
    pub series_id: u32,

    /// 1-based season number.
    pub season_number: u32,

    /// Path to the file.
    pub path: PathBuf,

    /// Number of bytes that this file occupies.
    pub size: u128,
}

/// Sonarr API client.
pub struct SonarrClient {
    client: reqwest::Client,
    base_url: reqwest::Url,
    simple_auth: Option<(String, String)>,
}

/// A Sonarr tag.
///
/// Tags can be used to organize shows, and to give them additional
/// properties (like marking them as manually managed via a `retain`
/// tag).
#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Eq, PartialOrd, Ord, Hash)]
pub struct Tag {
    /// Readable name of the tag.
    pub label: String,

    /// ID of the tag.
    pub id: TagId,
}

/// A tag ID.
///
/// Represented as a number in the Sonarr API and interned there.
#[derive(Deserialize, Serialize, Debug, Eq, Clone, Copy, PartialEq, PartialOrd, Ord, Hash)]
pub struct TagId(u32);

/// A list of tags from the Sonarr API.
#[derive(Deserialize, Debug)]
pub struct Tags {
    tags: Vec<Tag>,
}

impl Tags {
    /// Returns the tag with a given name.
    pub fn get(&self, name: &str) -> Option<&Tag> {
        self.tags.iter().find(|t| t.label == name)
    }
}

impl SonarrClient {
    /// Constructs a Sonarr API client from configuration.
    pub fn from_config(
        conf: &config::ServerSettings<config::Sonarr>,
    ) -> Result<SonarrClient, Box<dyn Error>> {
        let (base_url, auth_headers) = conf.sonarr_base();
        let mut simple_auth = None;
        if let (username, Some(password)) = (base_url.username(), base_url.password()) {
            simple_auth = Some((username.to_string(), password.to_string()));
        }
        let client = reqwest::Client::builder()
            .default_headers(auth_headers)
            .redirect(reqwest::RedirectPolicy::none()) // getting redirected means we're doing it wrong
            .build()?;
        Ok(SonarrClient {
            client,
            base_url,
            simple_auth,
        })
    }

    /// Returns all tags known to Sonarr.
    pub fn fetch_tags(&self) -> Result<Tags, Box<dyn Error>> {
        let url = self.base_url.join("tag")?;
        let mut req = self.client.get(url);
        req = self.add_auth(req);
        let mut response = req.send()?.error_for_status()?;
        let tags: Vec<Tag> = response.json()?;
        Ok(Tags { tags })
    }

    /// Add HTTP simple auth to a request to the Sonarr API.
    fn add_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some((user, pass)) = &self.simple_auth {
            return req.basic_auth(user, Some(pass));
        }
        req
    }

    /// Fetches all the TV series that Sonarr knows about.
    pub fn fetch_all_series(&self) -> Result<Vec<Series>, Box<dyn Error>> {
        let url = self.base_url.join("series")?;
        let mut req = self.client.get(url);
        req = self.add_auth(req);
        let mut response = req.send()?.error_for_status()?;
        let series: Vec<Series> = response.json()?;
        Ok(series)
    }

    /// Fetches information about a single series.
    pub fn fetch_series<S: DeserializeOwned>(&self, series_id: u32) -> Result<S, Box<dyn Error>> {
        let url = self.base_url.join(
            PathBuf::from("series")
                .join(&series_id.to_string())
                .to_str()
                .unwrap(),
        )?;
        let mut req = self.client.get(url);
        req = self.add_auth(req);
        let mut response = req.send()?.error_for_status()?;
        Ok(response.json()?)
    }

    /// Updates information about a single TV show.
    fn update_series<S: Serialize + IdEd>(&self, series: &S) -> Result<Series, Box<dyn Error>> {
        let url = self.base_url.join(
            PathBuf::from("series")
                .join(&series.id().to_string())
                .to_str()
                .unwrap(),
        )?;
        let mut req = self.client.put(url).body(serde_json::to_vec(&series)?);
        req = self.add_auth(req);
        let mut response = req.send()?.error_for_status()?;
        Ok(response.json()?)
    }

    /// Returns all [`EpisodeFile`]s in a TV series.
    pub fn fetch_episode_files(&self, series_id: u32) -> Result<Vec<EpisodeFile>, Box<dyn Error>> {
        let url = self
            .base_url
            .join(&format!("episodefile?seriesId={}", series_id))?;
        let mut req = self.client.get(url);
        req = self.add_auth(req);

        let mut response = req.send()?.error_for_status()?;
        let epfiles: Vec<EpisodeFile> = response.json()?;
        Ok(epfiles)
    }

    /// Marks a TV season as unmonitored.
    ///
    /// This makes Sonarr skip downloading more/updated episodes for
    /// the season.
    pub fn unmonitor_season(&self, series_id: u32, season: u32) -> Result<(), Box<dyn Error>> {
        #[derive(Deserialize, Serialize, Debug, PartialEq)]
        #[serde(rename_all = "camelCase")]
        struct UpdateSeries {
            id: u32,
            seasons: Vec<UpdateSeason>,
            #[serde(flatten)]
            extra: HashMap<String, Value>,
        }

        impl IdEd for UpdateSeries {
            fn id(&self) -> u32 {
                self.id
            }
        }

        #[derive(Deserialize, Serialize, Debug, PartialEq)]
        #[serde(rename_all = "camelCase")]
        pub struct UpdateSeason {
            season_number: u32,
            monitored: bool,
            #[serde(flatten)]
            extra: HashMap<String, Value>,
        }

        let mut series: UpdateSeries = self.fetch_series(series_id)?;
        if let Some((i, _)) = series
            .seasons
            .iter()
            .enumerate()
            .find(|(_, s)| s.season_number == season)
        {
            series.seasons[i].monitored = false;
            self.update_series(&series)?;
        }
        Ok(())
    }

    /// Deletes a list of [`EpisodeFile`]s.
    pub fn delete_episode_file(&self, ef: &EpisodeFile) -> Result<(), Box<dyn Error>> {
        let url = self.base_url.join(
            PathBuf::from("episodefile")
                .join(&ef.id.to_string())
                .to_str()
                .unwrap(),
        )?;
        let mut req = self.client.delete(url.clone());
        req = self.add_auth(req);
        match req.send()? {
            resp if resp.status().is_success() => Ok(()),
            resp if resp.status().is_server_error() => {
                // retry on failure and don't worry if the file is gone already:
                retry(Exponential::from_millis(200), || {
                    info!(
                        "HTTP DELETE failed with status {:?}. Retrying...",
                        resp.status()
                    );
                    let mut req = self.client.delete(url.clone());
                    req = self.add_auth(req);
                    match req.send()? {
                        resp if resp.status().is_success()
                            || resp.status() == reqwest::StatusCode::NOT_FOUND =>
                        {
                            Ok(())
                        }
                        resp => resp.error_for_status().map(|_| ()),
                    }
                })?;
                Ok(())
            }
            resp => {
                resp.error_for_status().map(|_| ())?;
                Ok(())
            }
        }
    }
}
