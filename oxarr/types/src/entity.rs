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
    Series,
}

impl EntityKind {
    /// The physical library kinds configurable on disk.
    pub const LIBRARY_KINDS: &'static [EntityKind] = &[EntityKind::Movie, EntityKind::Series];

    pub fn value(&self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::Series => "series",
            Self::Collection => "collection",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Movie => "Movies",
            Self::Series => "TV Shows",
            Self::Collection => "Collections",
        }
    }

    pub fn from_value(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "movie" => Some(Self::Movie),
            "series" => Some(Self::Series),
            "collection" => Some(Self::Collection),
            _ => None,
        }
    }
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

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SeriesDetails {
    pub year: Option<u16>,
    pub status: Option<String>,
    pub network: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum EntityDetails {
    Movie(MovieDetails),
    Collection(CollectionDetails),
    Series(SeriesDetails),
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
