use chrono::{DateTime, Utc};
use reqwest;
use reqwest::header;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct SeasonStats {
    pub episode_file_count: u32,
    pub total_episode_count: u32,
    pub episode_count: u32,

    pub next_airing: Option<DateTime<Utc>>,
    pub previous_airing: Option<DateTime<Utc>>,
    pub size_on_disk: u128,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Season {
    pub season_number: u32,
    pub monitored: bool,
    pub statistics: SeasonStats,
}

pub trait IdEd {
    fn id(&self) -> u32;
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub struct Series {
    pub title: String,
    pub id: u32,
    pub tags: Vec<TagId>,
    pub seasons: Vec<Season>,
}

impl IdEd for Series {
    fn id(&self) -> u32 {
        self.id
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Episode {
    pub series_id: u32,
    pub id: u32,

    // The actual ID we can use to delete the episode on disk
    pub episode_file_id: u32,

    pub season_number: u32,
    pub episode_number: u32,
    pub title: String,
    pub has_file: bool,
    pub monitored: bool,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeFile {
    pub id: u32,

    pub series_id: u32,
    pub season_number: u32,
    pub path: PathBuf,
    pub size: u128,
}

pub struct SonarrClient {
    client: reqwest::Client,
    base_url: reqwest::Url,
    simple_auth: Option<(String, String)>,
}

#[derive(Deserialize, Serialize, PartialEq, Debug, Clone, Eq, PartialOrd, Ord, Hash)]
pub struct Tag {
    pub label: String,
    pub id: TagId,
}

#[derive(Deserialize, Serialize, Debug, Eq, Clone, Copy, PartialEq, PartialOrd, Ord, Hash)]
pub struct TagId(u32);

#[derive(Deserialize, Debug)]
pub struct Tags {
    tags: Vec<Tag>,
}

impl Tags {
    pub fn get(&self, name: String) -> Option<&Tag> {
        self.tags.iter().find(|t| t.label == name)
    }
}

impl SonarrClient {
    pub fn new(base_url: String, api_key: String) -> Result<SonarrClient, Box<dyn Error>> {
        let base_url = reqwest::Url::parse(&base_url)?;
        let mut headers = header::HeaderMap::new();
        headers.insert("X-Api-Key", header::HeaderValue::from_str(&api_key)?);
        let mut simple_auth = None;
        if let (username, Some(password)) = (base_url.username(), base_url.password()) {
            simple_auth = Some((username.to_string(), password.to_string()));
        }
        let client = reqwest::Client::builder()
            .default_headers(headers)
            .redirect(reqwest::RedirectPolicy::none()) // getting redirected means we're doing it wrong
            .build()?;
        Ok(SonarrClient {
            client,
            base_url,
            simple_auth,
        })
    }

    pub fn fetch_tags(&self) -> Result<Tags, Box<dyn Error>> {
        let url = self.base_url.join("tag")?;
        let mut req = self.client.get(url);
        req = self.add_auth(req);
        let mut response = req.send()?.error_for_status()?;
        let tags: Vec<Tag> = response.json()?;
        Ok(Tags { tags })
    }

    pub fn add_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some((user, pass)) = &self.simple_auth {
            return req.basic_auth(user, Some(pass));
        }
        req
    }

    pub fn fetch_all_series(&self) -> Result<Vec<Series>, Box<dyn Error>> {
        let url = self.base_url.join("series")?;
        let mut req = self.client.get(url);
        req = self.add_auth(req);
        let mut response = req.send()?.error_for_status()?;
        let series: Vec<Series> = response.json()?;
        Ok(series)
    }

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

    pub fn update_series<S: Serialize + IdEd>(&self, series: &S) -> Result<Series, Box<dyn Error>> {
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

    pub fn delete_episode_file(&self, ef: &EpisodeFile) -> Result<(), Box<dyn Error>> {
        let url = self.base_url.join(
            PathBuf::from("episodefile")
                .join(&ef.id.to_string())
                .to_str()
                .unwrap(),
        )?;
        let mut req = self.client.delete(url);
        req = self.add_auth(req);
        let _response = req.send()?.error_for_status()?;
        Ok(())
    }
}
