use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Ephemeral search or RSS item emitted by an indexer (not persisted directly).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseCandidate {
    pub indexer: String,
    pub guid: String,
    pub title: String,
    pub download_url: String,
    pub info_hash: Option<String>,
    pub size_bytes: u64,
    pub seeders: Option<u32>,
    pub leechers: Option<u32>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackedReleaseStatus {
    Grabbed,
    Imported,
    Failed,
    Blocklisted,
}

/// Persisted release lifecycle record (grabs, imports, failures, and blocklist entries).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrackedRelease {
    pub id: Uuid,
    pub entity_id: Uuid,
    pub indexer: String,
    pub guid: String,
    pub title: String,
    pub download_url: String,
    pub info_hash: Option<String>,
    pub size_bytes: u64,
    pub score: i32,
    pub matched_classifier_ids: Vec<Uuid>,
    pub status: TrackedReleaseStatus,
    pub download_client_id: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
