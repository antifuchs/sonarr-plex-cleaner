use reqwest;
use serde::Deserialize;
use serde_xml_rs;
use std::error::Error;
use std::path::Path;
use std::path::PathBuf;

use crate::config;

pub struct PlexClient {
    base_url: reqwest::Url,
    client: reqwest::Client,
}

#[derive(Debug, PartialEq, Deserialize)]
pub enum MediaKind {
    #[serde(rename = "movie")]
    Movie,

    #[serde(rename = "show")]
    TV,

    #[serde(rename = "season")]
    TVSeason,

    #[serde(rename = "season")]
    TVEpisode,

    AllEpisodes,

    #[serde(rename = "artist")]
    Music,
}

#[derive(Debug, Deserialize)]
pub struct Directory {
    #[serde(rename = "key")]
    pub id: u32,

    #[serde(rename = "type")]
    pub kind: MediaKind,

    pub title: String,
}

#[derive(Debug, Deserialize)]
struct LibraryOverview {
    #[serde(rename = "Directory", default)]
    directories: Vec<Directory>,
}

#[derive(Debug, Deserialize)]
pub struct Show {
    #[serde(rename = "key")]
    pub id: String,

    #[serde(rename = "type")]
    pub kind: MediaKind,

    pub title: String,
}

fn all_episodes_pseudoseason() -> MediaKind {
    MediaKind::AllEpisodes
}

#[derive(Debug, Deserialize)]
pub struct Season {
    #[serde(rename = "key")]
    pub id: String,

    #[serde(rename = "parentTitle", default)]
    pub show_name: String,

    pub title: String,

    #[serde(rename = "type", default = "all_episodes_pseudoseason")]
    pub kind: MediaKind,

    #[serde(rename = "leafCount", default)]
    pub episodes: u32,

    #[serde(rename = "viewedLeafCount", default)]
    pub viewed_episodes: u32,
}

impl Season {
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

    pub fn libraries(&self) -> Result<Vec<Directory>, Box<dyn Error>> {
        let url = self.build_url(vec!["library/sections"]);

        let resp = self.client.get(url).send()?.error_for_status()?;
        let container: LibraryOverview = serde_xml_rs::from_reader(resp)?;
        Ok(container.directories)
    }

    pub fn list_shows(&self, library: Directory) -> Result<Vec<Show>, Box<dyn Error>> {
        let url = self.build_url(vec!["library", "sections", &library.id.to_string(), "all"]);

        let resp = self.client.get(url).send()?.error_for_status()?;
        let container: TVListing = serde_xml_rs::from_reader(resp)?;
        Ok(container.shows)
    }

    pub fn list_seasons(&self, show: Show) -> Result<Vec<Season>, Box<dyn Error>> {
        let url = self.build_url(vec![show.id]);
        let resp = self.client.get(url).send()?.error_for_status()?;
        let container: TVShow = serde_xml_rs::from_reader(resp)?;
        Ok(container.seasons)
    }

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
