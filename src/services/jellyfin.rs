//! The jellyfin/emby media server API, with only the endpoints that serve our purposes.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use serde::Deserialize;

use crate::config;

/// Makes requests to a jellyfin/emby server API.
///
/// You may ask, why not use the openapi spec? Well, if you can find
/// an openapi rust code generator that can parse the jellyfin openapi
/// spec & generates working code... be my guest and port it.
#[derive(Debug)]
pub struct JellyfinClient {
    client: BaseClient,
    user_id: String,
}

impl JellyfinClient {
    /// Construct a new client
    pub fn from_config(conf: &config::JellyfinSettings) -> Result<JellyfinClient> {
        let (base_url, auth_headers) = conf.server.jellyfin_base();
        let client = reqwest::Client::builder()
            .redirect(reqwest::RedirectPolicy::none())
            .default_headers(auth_headers)
            .build()?;
        let client = BaseClient { base_url, client };
        let user_id = client.get_user_id(&conf.user)?;
        Ok(JellyfinClient { client, user_id })
    }

    /// Retrieve all TV seasons available to the given user on the server.
    pub fn all_tv_seasons(&self) -> Result<Vec<Season>> {
        let url = self.client.build_url(["/Users", &self.user_id, "Items"]);
        let resp: SeasonResponse = self
            .client
            .client
            .get(url)
            .query(&[("Recursive", "true"), ("includeItemTypes", "Season")])
            .send()?
            .error_for_status()?
            .json()?;
        Ok(resp.items)
    }
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SeasonResponse {
    items: Vec<Season>,
}

/// A season of TV shows in Jellyfin.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Season {
    /// Name of the season
    pub name: String,

    /// Name of the series
    pub series_name: String,
    id: String,
    user_data: SeasonUserData,
}

impl Season {
    /// Return true of user has no unwatched episodes left in this season (i.e., is fully caught up).
    pub fn fully_watched(&self) -> bool {
        self.user_data.unplayed_item_count == 0
    }
}

/// User-specific data for a season of TV in Jellyfin.
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct SeasonUserData {
    unplayed_item_count: usize,
}

#[derive(Debug)]
struct BaseClient {
    base_url: reqwest::Url,
    client: reqwest::Client,
}

impl BaseClient {
    fn build_url<S: AsRef<Path>>(&self, path_bits: impl IntoIterator<Item = S>) -> reqwest::Url {
        let mut path = PathBuf::new();
        for bit in path_bits {
            path.push(bit);
        }
        self.base_url
            .join(path.to_str().unwrap_or(""))
            .expect("hoped for a valid URL")
    }

    /// Retrieve a user ID corresponding to a user name.
    fn get_user_id(&self, name: &str) -> Result<String> {
        let url = self.build_url(["/Users"]);
        let mut resp = self.client.get(url).send()?.error_for_status()?;
        let users: Vec<User> = resp.json()?;
        users
            .iter()
            .find(|user| user.name == name)
            .map(|found| found.id.clone())
            .ok_or_else(|| anyhow!("user not found"))
    }
}

/// A JellyFin API response to the /Users route
#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct User {
    name: String,
    id: String,
}
