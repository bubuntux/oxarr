use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::entity::EntityKind;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub year: Option<u16>,
    pub entity_kind: Option<EntityKind>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchResult {
    pub provider_id: String,
    pub entity_kind: EntityKind,
    pub title: String,
    pub year: Option<u16>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionSummary {
    pub provider_id: String,
    pub title: String,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MovieMetadata {
    pub provider_id: String,
    pub external_ids: BTreeMap<String, String>,
    pub title: String,
    pub original_title: Option<String>,
    pub original_language: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub genres: Vec<String>,
    pub year: Option<u16>,
    pub in_cinemas_date: Option<NaiveDate>,
    pub digital_release_date: Option<NaiveDate>,
    pub physical_release_date: Option<NaiveDate>,
    pub runtime_minutes: Option<u32>,
    pub certification: Option<String>,
    pub studio: Option<String>,
    pub collection: Option<CollectionSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionMetadata {
    pub provider_id: String,
    pub external_ids: BTreeMap<String, String>,
    pub title: String,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub movies: Vec<SearchResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SeriesMetadata {
    pub provider_id: String,
    pub external_ids: BTreeMap<String, String>,
    pub title: String,
    pub original_title: Option<String>,
    pub original_language: Option<String>,
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub genres: Vec<String>,
    pub year: Option<u16>,
    pub status: Option<String>,
    pub network: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ProviderMediaMetadata {
    Movie(Box<MovieMetadata>),
    Collection(CollectionMetadata),
    Series(Box<SeriesMetadata>),
}

impl ProviderMediaMetadata {
    pub fn title(&self) -> &str {
        match self {
            Self::Movie(m) => &m.title,
            Self::Collection(c) => &c.title,
            Self::Series(s) => &s.title,
        }
    }

    pub fn external_ids(&self) -> &BTreeMap<String, String> {
        match self {
            Self::Movie(m) => &m.external_ids,
            Self::Collection(c) => &c.external_ids,
            Self::Series(s) => &s.external_ids,
        }
    }

    pub fn overview(&self) -> Option<&str> {
        match self {
            Self::Movie(m) => m.overview.as_deref(),
            Self::Collection(c) => c.overview.as_deref(),
            Self::Series(s) => s.overview.as_deref(),
        }
    }

    pub fn poster_url(&self) -> Option<&str> {
        match self {
            Self::Movie(m) => m.poster_url.as_deref(),
            Self::Collection(c) => c.poster_url.as_deref(),
            Self::Series(s) => s.poster_url.as_deref(),
        }
    }

    pub fn backdrop_url(&self) -> Option<&str> {
        match self {
            Self::Movie(m) => m.backdrop_url.as_deref(),
            Self::Collection(c) => c.backdrop_url.as_deref(),
            Self::Series(s) => s.backdrop_url.as_deref(),
        }
    }

    pub fn genres(&self) -> &[String] {
        match self {
            Self::Movie(m) => &m.genres,
            Self::Collection(_) => &[],
            Self::Series(s) => &s.genres,
        }
    }
}
