use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedReleaseInfo {
    pub clean_title: String,
    pub year: Option<u16>,
    pub matched_classifier_ids: Vec<Uuid>,
}
