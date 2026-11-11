use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MovieDetails {
    pub year: Option<u16>,
    pub in_cinemas_date: Option<NaiveDate>,
    pub digital_release_date: Option<NaiveDate>,
    pub physical_release_date: Option<NaiveDate>,
    pub runtime_minutes: Option<u32>,
    pub certification: Option<String>,
    pub studio: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CollectionDetails {}
