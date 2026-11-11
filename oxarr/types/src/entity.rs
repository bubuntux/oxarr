use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::media::{CollectionDetails, MovieDetails};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Movie,
    Collection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Library {
    pub id: Uuid,
    pub name: String,
    pub kind: EntityKind,
    pub root_path: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityMetadata {
    pub overview: Option<String>,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub genres: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum EntityDetails {
    Movie(MovieDetails),
    Collection(CollectionDetails),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entity {
    pub id: Uuid,
    pub kind: EntityKind,
    pub library_id: Option<Uuid>,
    pub title: String,
    pub original_title: Option<String>,
    /// BCP-47 / ISO-639 language tag of the original work (e.g. `"en"`, `"ja"`, `"ko"`).
    pub original_language: Option<String>,
    pub monitored: bool,
    /// External provider -> identifier map (e.g. `{"tmdb": "603", "imdb": "tt0133093"}`).
    pub external_ids: BTreeMap<String, String>,
    pub metadata: EntityMetadata,
    pub details: EntityDetails,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
