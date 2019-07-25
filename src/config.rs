//! Sonarr Plex Cleaner CLI Config

use abscissa_core::Config;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Url,
};
use secrecy::{CloneableSecret, DebugSecret, ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::time::Duration;
use zeroize::Zeroize;

#[derive(PartialEq, Eq, Clone, Zeroize, Default, Debug, Deserialize, Serialize)]
#[zeroize(drop)]
/// Represents an API key.
pub struct APIKey(String);

impl CloneableSecret for APIKey {}
impl DebugSecret for APIKey {
    fn debug_secret() -> &'static str {
        "*****[API KEY]*****"
    }
}

/// Marker for Plex server settings.
#[derive(Clone, PartialEq, Debug)]
pub enum Plex {}

/// Marker for Sonarr server settings.
#[derive(Clone, PartialEq, Debug)]
pub enum Sonarr {}

/// Sonarr Plex Cleaner CLI Configuration. Does not support
/// serializing back to the config file.
#[derive(Clone, Config, Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct SonarrPlexCleanerCliConfig {
    /// Settings for TV shows (managed by Sonarr). Extract the the Sonarr API key
    /// from Settings -> General.
    pub tv: ServerSettings<Sonarr>,

    /// Settings for the media viewing application (Plex). See Plex
    /// help: https://bit.ly/2p7RtOu for API key instructions.
    pub plex: ServerSettings<Plex>,

    /// Settings that govern the retention policy.
    pub retention: RetentionSettings,
}

/// Server settings. These are common across all media management
/// apps: There is a URL and an API key.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServerSettings<T> {
    /// Where to reach the server.
    #[serde(with = "url_serde")]
    pub url: Url,

    /// API key for the server.
    pub api_key: Secret<APIKey>,

    #[serde(skip_deserializing, skip_serializing)]
    spoopy: PhantomData<T>,
}

impl<T> Default for ServerSettings<T> {
    fn default() -> Self {
        ServerSettings {
            url: Url::parse("https://example.com/please/set/a/url").unwrap(),
            api_key: Secret::new(Default::default()),
            spoopy: PhantomData,
        }
    }
}

impl ServerSettings<Plex> {
    /// Returns a URL and a set of headers that can be used to access plex.
    pub fn plex_base(&self) -> (Url, HeaderMap) {
        (
            self.url.clone(),
            vec![(
                HeaderName::from_static("x-plex-token"),
                HeaderValue::from_str(&self.api_key.expose_secret().0).unwrap(),
            )]
            .into_iter()
            .collect(),
        )
    }
}

impl ServerSettings<Sonarr> {
    /// Returns a URL and request headers that can be used to access
    /// the sonarr API.
    pub fn sonarr_base(&self) -> (Url, HeaderMap) {
        (
            self.url.clone(),
            vec![(
                HeaderName::from_static("x-api-key"),
                HeaderValue::from_str(&self.api_key.expose_secret().0).unwrap(),
            )]
            .into_iter()
            .collect(),
        )
    }
}

/// Settings that govern how long any item is kept.
#[derive(Clone, Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct RetentionSettings {
    /// The tag name indicating that an item should be kept even if it
    /// fulfills all criteria. If unset, all items are subject to the
    /// retention policy.
    pub retain_tag: Option<String>,

    /// The amount of time an item should be kept even after
    /// fulfilling all other criteria.
    ///
    /// ## Example
    /// ``` toml
    /// retain_duration = "12 days"
    /// ```
    #[serde(with = "serde_humantime", default)]
    pub retain_duration: Duration,
}
