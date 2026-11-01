use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: Uuid,
    /// Catalog entities satisfied by this file (1 movie, or multiple episodes for `S01E01E02.mkv`).
    pub entity_ids: Vec<Uuid>,
    pub path: String,
    pub size_bytes: u64,
    /// Classifier IDs matched by this asset against configured classifiers.
    pub matched_classifier_ids: Vec<Uuid>,
    pub quality_score: Option<i32>,
    /// Container, duration, stream details, codec parameters used by Jellyfin `/PlaybackInfo` and format matching.
    pub media_info: serde_json::Value,
    pub imported_at: DateTime<Utc>,
}
