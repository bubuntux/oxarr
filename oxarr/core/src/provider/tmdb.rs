use std::collections::BTreeMap;

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::NaiveDate;
use oxarr_types::{
    CollectionSummary, EntityKind, MovieMetadata, ProviderMediaMetadata, SearchQuery, SearchResult,
};
use reqwest::Client;
use serde::Deserialize;

use super::MetadataProvider;

pub const TMDB_DEFAULT_BASE_URL: &str = "https://api.themoviedb.org/3";
pub const TMDB_IMAGE_BASE_URL: &str = "https://image.tmdb.org/t/p/original";

/// Native TMDB metadata provider for movies and movie collections.
pub struct TmdbProvider {
    client: Client,
    api_key: Option<String>,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct TmdbMovieSearchResponse {
    results: Vec<TmdbMovieSearchItem>,
}

#[derive(Debug, Deserialize)]
struct TmdbMovieSearchItem {
    id: u64,
    title: String,
    release_date: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TmdbMovieDetails {
    id: u64,
    title: String,
    original_title: Option<String>,
    original_language: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    release_date: Option<String>,
    runtime: Option<u32>,
    genres: Option<Vec<TmdbGenre>>,
    belongs_to_collection: Option<TmdbBelongsToCollection>,
    external_ids: Option<TmdbExternalIds>,
}

#[derive(Debug, Deserialize)]
struct TmdbGenre {
    name: String,
}

#[derive(Debug, Deserialize)]
struct TmdbBelongsToCollection {
    id: u64,
    name: String,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TmdbExternalIds {
    imdb_id: Option<String>,
}

impl TmdbProvider {
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_base_url(api_key, TMDB_DEFAULT_BASE_URL)
    }

    pub fn with_base_url(api_key: Option<String>, base_url: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .user_agent("oxarr/0.1 (+https://github.com/bubuntux/oxarr)")
                .build()
                .unwrap_or_default(),
            api_key,
            base_url: base_url.into(),
        }
    }

    fn format_image_url(path: Option<&str>) -> Option<String> {
        path.map(|p| {
            if p.starts_with("http") {
                p.to_owned()
            } else {
                format!("{TMDB_IMAGE_BASE_URL}{p}")
            }
        })
    }

    fn map_details_to_metadata(&self, details: TmdbMovieDetails) -> MovieMetadata {
        let mut external_ids = BTreeMap::new();
        external_ids.insert("tmdb".to_owned(), details.id.to_string());

        if let Some(externals) = details.external_ids {
            if let Some(imdb) = externals.imdb_id.filter(|s| !s.is_empty()) {
                external_ids.insert("imdb".to_owned(), imdb);
            }
        }

        let (year, digital_date) = match details.release_date.as_deref() {
            Some(date_str) => {
                let parsed = NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok();
                let year = parsed
                    .as_ref()
                    .and_then(|d| d.format("%Y").to_string().parse::<u16>().ok());
                (year, parsed)
            }
            None => (None, None),
        };

        let poster_url = Self::format_image_url(details.poster_path.as_deref());
        let backdrop_url = Self::format_image_url(details.backdrop_path.as_deref());

        let genres = details
            .genres
            .unwrap_or_default()
            .into_iter()
            .map(|g| g.name)
            .collect();

        let collection = details.belongs_to_collection.map(|c| CollectionSummary {
            provider_id: format!("tmdb:{}", c.id),
            title: c.name,
            overview: None,
            poster_url: Self::format_image_url(c.poster_path.as_deref()),
            backdrop_url: Self::format_image_url(c.backdrop_path.as_deref()),
        });

        MovieMetadata {
            provider_id: format!("tmdb:{}", details.id),
            external_ids,
            title: details.title,
            original_title: details.original_title,
            original_language: details.original_language,
            overview: details.overview,
            poster_url,
            backdrop_url,
            genres,
            year,
            in_cinemas_date: digital_date,
            digital_release_date: digital_date,
            physical_release_date: None,
            runtime_minutes: details.runtime,
            certification: None,
            studio: None,
            collection,
        }
    }
}

#[async_trait]
impl MetadataProvider for TmdbProvider {
    fn id(&self) -> &str {
        "tmdb"
    }

    fn supported_kinds(&self) -> &[EntityKind] {
        static KINDS: [EntityKind; 2] = [EntityKind::Movie, EntityKind::Collection];
        &KINDS
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let Some(api_key) = &self.api_key else {
            return Ok(Vec::new());
        };

        let mut req = self
            .client
            .get(format!("{}/search/movie", self.base_url))
            .query(&[("api_key", api_key.as_str()), ("query", query.query.as_str())]);

        if let Some(year) = query.year {
            let year_str = year.to_string();
            req = req.query(&[("year", year_str.as_str())]);
        }

        let resp = req
            .send()
            .await
            .with_context(|| format!("failed to query TMDB search for '{}'", query.query))?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let search_res: TmdbMovieSearchResponse = resp.json().await.unwrap_or(TmdbMovieSearchResponse { results: Vec::new() });
        let results = search_res
            .results
            .into_iter()
            .map(|item| {
                let year = item
                    .release_date
                    .as_deref()
                    .and_then(|d| d.split('-').next().and_then(|y| y.parse::<u16>().ok()));
                let poster_url = Self::format_image_url(item.poster_path.as_deref());

                SearchResult {
                    provider_id: format!("tmdb:{}", item.id),
                    entity_kind: EntityKind::Movie,
                    title: item.title,
                    year,
                    overview: item.overview,
                    poster_url,
                }
            })
            .collect();

        Ok(results)
    }

    async fn fetch_details(
        &self,
        kind: &EntityKind,
        external_id: &str,
    ) -> Result<ProviderMediaMetadata> {
        if *kind != EntityKind::Movie {
            anyhow::bail!("TMDB native provider only supports Movie entity kind for fetch_details");
        }

        let Some(api_key) = &self.api_key else {
            anyhow::bail!("TMDB API key not configured");
        };

        let id = external_id.trim_start_matches("tmdb:");
        let url = format!("{}/movie/{}", self.base_url, id);
        let resp = self
            .client
            .get(&url)
            .query(&[("api_key", api_key.as_str()), ("append_to_response", "external_ids")])
            .send()
            .await
            .with_context(|| format!("failed to fetch TMDB movie details for {id}"))?;

        if !resp.status().is_success() {
            anyhow::bail!("TMDB movie not found for {id}");
        }

        let details: TmdbMovieDetails = resp.json().await?;
        let metadata = self.map_details_to_metadata(details);
        Ok(ProviderMediaMetadata::Movie(Box::new(metadata)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tmdb_movie_json() {
        let raw = r#"{
            "id": 603,
            "title": "The Matrix",
            "original_title": "The Matrix",
            "original_language": "en",
            "overview": "Set in the 22nd century...",
            "poster_path": "/f89U3ADr1oiB1s9GkdPOEpXUk5H.jpg",
            "backdrop_path": "/oE6bhqqMrvBPaignomcLLRUQIYR.jpg",
            "release_date": "1999-03-30",
            "runtime": 136,
            "genres": [{"name": "Action"}, {"name": "Science Fiction"}],
            "belongs_to_collection": {
                "id": 2344,
                "name": "The Matrix Collection",
                "poster_path": "/bVVaWBud0vOX9MmURTMYQ575YJk.jpg"
            },
            "external_ids": {
                "imdb_id": "tt0133093"
            }
        }"#;

        let details: TmdbMovieDetails = serde_json::from_str(raw).expect("deserializes movie");
        let provider = TmdbProvider::new(None);
        let meta = provider.map_details_to_metadata(details);

        assert_eq!(meta.title, "The Matrix");
        assert_eq!(meta.year, Some(1999));
        assert_eq!(meta.runtime_minutes, Some(136));
        assert_eq!(meta.genres, vec!["Action", "Science Fiction"]);
        assert_eq!(meta.external_ids.get("tmdb").map(String::as_str), Some("603"));
        assert_eq!(meta.external_ids.get("imdb").map(String::as_str), Some("tt0133093"));
        assert!(meta.poster_url.as_ref().unwrap().contains("f89U3ADr1oiB1s9GkdPOEpXUk5H.jpg"));
        assert_eq!(
            meta.collection.as_ref().map(|c| c.title.as_str()),
            Some("The Matrix Collection")
        );
    }
}
