use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EntityType {
    Movie,
    Series,
    Season,
    Episode,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub year: Option<u16>,
    pub entity_type: Option<EntityType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub provider_id: String,
    pub title: String,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaDetails {
    pub provider_id: String,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub genres: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseCandidate {
    pub title: String,
    pub download_url: String,
    pub info_hash: Option<String>,
    pub size_bytes: u64,
    pub indexer: String,
    pub seeders: Option<u32>,
    pub leechers: Option<u32>,
    pub published_at: Option<DateTime<Utc>>,
}
