use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Atomic condition specification for matching releases or media files.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClassifierCondition {
    /// Matches regex pattern against the release title, file name, or path.
    Regex {
        pattern: String,
        #[serde(default)]
        negate: bool,
    },
    /// Matches regex pattern against the indexer name or indexer tags.
    IndexerFlag {
        pattern: String,
        #[serde(default)]
        negate: bool,
    },
    /// Numeric range comparison against properties like `size_bytes`, `audio_channels`, `bitrate`, `runtime_minutes`.
    NumericProperty {
        property: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min: Option<i64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        max: Option<i64>,
    },
    /// Text comparison or regex against arbitrary metadata fields (e.g. `container`, `audio_codec`, `video_codec`).
    TextProperty {
        property: String,
        pattern: String,
        #[serde(default)]
        negate: bool,
    },
}

/// A named classification label (e.g. "2160p", "Dolby Vision", "TrueHD Atmos", "Remux").
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Classifier {
    pub id: Uuid,
    pub name: String,
    pub conditions: Vec<ClassifierCondition>,
    /// When true, all conditions must match (AND). When false, any condition match qualifies (OR).
    pub match_all: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
