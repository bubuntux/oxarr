use extism_pdk::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
}

#[derive(Serialize, Deserialize)]
pub struct SearchResult {
    pub provider_id: String,
    pub title: String,
}

#[plugin_fn]
pub fn search(Json(input): Json<SearchQuery>) -> FnResult<Json<Vec<SearchResult>>> {
    Ok(Json(vec![SearchResult {
        provider_id: "tmdb:placeholder".into(),
        title: input.query,
    }]))
}
