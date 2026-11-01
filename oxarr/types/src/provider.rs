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
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ProviderMediaMetadata {
    Movie(Box<MovieMetadata>),
    Collection(CollectionMetadata),
}
