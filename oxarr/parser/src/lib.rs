use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution {
    R2160p,
    R1080p,
    R720p,
    R480p,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedReleaseInfo {
    pub clean_title: String,
    pub year: Option<u16>,
    pub resolution: Resolution,
    pub source: Option<String>,
    pub codec: Option<String>,
    pub release_group: Option<String>,
}
