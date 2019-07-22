//! The Plex Media Server API.

use reqwest;
use serde::Deserialize;
use serde_xml_rs;
use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use crate::config;

/// Makes requests to a Plex media server API.
pub struct PlexClient {
    base_url: reqwest::Url,
    client: reqwest::Client,
}

/// The kind of media in a plex media server library.
#[derive(Debug, PartialEq, Deserialize)]
pub enum MediaKind {
    /// A movie.
    #[serde(rename = "movie")]
    Movie,

    /// A TV show.
    #[serde(rename = "show")]
    TV,

    /// A TV show's season.
    #[serde(rename = "season")]
    TVSeason,

    /// A single TV show's season's episode.
    #[serde(rename = "season")]
    TVEpisode,

    /// The pseudo-season entry "All Episodes".
    AllEpisodes,

    /// A music library entry.
    #[serde(rename = "artist")]
    Music,
}

/// A top-level media library entry.
#[derive(Debug, Deserialize)]
pub struct Directory {
    /// ID of the entry
    #[serde(rename = "key")]
    pub id: u32,

    /// Kind of media that this library section contains.
    #[serde(rename = "type")]
    pub kind: MediaKind,

    /// Name assigned in the UI.
    pub title: String,
}

#[derive(Debug, Deserialize)]
struct LibraryOverview {
    #[serde(rename = "Directory", default)]
    directories: Vec<Directory>,
}

/// A TV show media library entry.
#[derive(Debug, Deserialize)]
pub struct Show {
    /// ID of this library subsection.
    #[serde(rename = "key")]
    pub id: String,

    /// Kind of media (should be `TV`).
    #[serde(rename = "type")]
    pub kind: MediaKind,

    /// Name of the show.
    pub title: String,
}

fn all_episodes_pseudoseason() -> MediaKind {
    MediaKind::AllEpisodes
}

/// A season of a TV show.
#[derive(Debug, Deserialize)]
pub struct Season {
    /// ID of the library entry.
    #[serde(rename = "key")]
    pub id: String,

    /// Name of the show that this season is for.
    #[serde(rename = "parentTitle", default)]
    pub show_name: String,

    /// Title of the season
    pub title: String,

    /// Kind of the season. Either `TVSeason` or `AllEpisodes`.
    #[serde(rename = "type", default = "all_episodes_pseudoseason")]
    pub kind: MediaKind,

    /// Number of episodes in the season that are indexed by Plex.
    #[serde(rename = "leafCount", default)]
    pub episodes: u32,

    /// Number of episodes that have been marked "viewed".
    #[serde(rename = "viewedLeafCount", default)]
    pub viewed_episodes: u32,
}

impl Season {
    /// True if all episodes in the season (that Plex knows about)
    /// have been watched.
    pub fn fully_watched(&self) -> bool {
        self.viewed_episodes == self.episodes
    }
}

#[derive(Debug, Deserialize)]
struct TVListing {
    #[serde(rename = "Directory", default)]
    shows: Vec<Show>,
}

#[derive(Debug, Deserialize)]
struct TVShow {
    #[serde(rename = "Directory", default)]
    seasons: Vec<Season>,
}

impl PlexClient {
    /// Constructs a plex client from the application config.
    pub fn from_config(
        conf: &config::ServerSettings<config::Plex>,
    ) -> Result<PlexClient, Box<dyn Error>> {
        let (base_url, auth_headers) = conf.plex_base();
        let client = reqwest::Client::builder()
            .redirect(reqwest::RedirectPolicy::none())
            .default_headers(auth_headers)
            .build()?;
        Ok(PlexClient { base_url, client })
    }

    fn build_url<S: AsRef<Path>>(&self, path_bits: Vec<S>) -> reqwest::Url {
        let mut path = PathBuf::new();
        for bit in path_bits {
            path.push(bit);
        }
        self.base_url
            .join(path.to_str().unwrap_or(""))
            .expect("hoped for a valid URL")
    }

    /// Lists all libraries known to the plex server.
    fn libraries(&self) -> Result<Vec<Directory>, Box<dyn Error>> {
        let url = self.build_url(vec!["library/sections"]);

        let resp = self.client.get(url).send()?.error_for_status()?;
        let container: LibraryOverview = serde_xml_rs::from_reader(resp)?;
        Ok(container.directories)
    }

    /// Lists all TV shows in a directory.
    fn list_shows(&self, library: Directory) -> Result<Vec<Show>, Box<dyn Error>> {
        let url = self.build_url(vec!["library", "sections", &library.id.to_string(), "all"]);

        let resp = self.client.get(url).send()?.error_for_status()?;
        let container: TVListing = serde_xml_rs::from_reader(resp)?;
        Ok(container.shows)
    }

    /// Lists all seasons in a TV show.
    fn list_seasons(&self, show: Show) -> Result<Vec<Season>, Box<dyn Error>> {
        let url = self.build_url(vec![show.id]);
        let resp = self.client.get(url).send()?.error_for_status()?;
        let container: TVShow = serde_xml_rs::from_reader(resp)?;
        Ok(container.seasons)
    }

    /// Returns a list of all TV show seasons (in all TV libraries)
    /// known to Plex.
    pub fn all_tv_seasons(&self) -> Result<Vec<Season>, Box<dyn Error>> {
        Ok(self
            .libraries()?
            .into_iter()
            .filter(|d| d.kind == MediaKind::TV)
            .flat_map(|l| self.list_shows(l).expect("could not list library"))
            .flat_map(|s| self.list_seasons(s).expect("could not list show"))
            .filter(|s| s.kind != MediaKind::AllEpisodes)
            .collect())
    }
}
