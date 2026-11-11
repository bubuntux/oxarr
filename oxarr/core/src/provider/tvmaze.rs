use std::collections::BTreeMap;

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::NaiveDate;
use oxarr_types::{EntityKind, ProviderMediaMetadata, SearchQuery, SearchResult, SeriesMetadata};
use reqwest::Client;
use serde::Deserialize;

use super::MetadataProvider;

pub const TVMAZE_DEFAULT_BASE_URL: &str = "https://api.tvmaze.com";

/// Native zero-config TVMaze metadata provider for television series.
pub struct TvMazeProvider {
    client: Client,
    base_url: String,
}

#[derive(Debug, Deserialize)]
struct TvMazeSearchItem {
    show: TvMazeShow,
}

#[derive(Debug, Deserialize)]
struct TvMazeShow {
    id: u64,
    name: String,
    premiered: Option<String>,
    summary: Option<String>,
    genres: Option<Vec<String>>,
    status: Option<String>,
    image: Option<TvMazeImage>,
    network: Option<TvMazeNetwork>,
    web_channel: Option<TvMazeNetwork>,
    externals: Option<TvMazeExternals>,
}

#[derive(Debug, Deserialize)]
struct TvMazeImage {
    medium: Option<String>,
    original: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TvMazeNetwork {
    name: String,
}

#[derive(Debug, Deserialize)]
struct TvMazeExternals {
    imdb: Option<String>,
    thetvdb: Option<u64>,
}

fn strip_html_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut inside = false;
    for c in input.chars() {
        if c == '<' {
            inside = true;
        } else if c == '>' {
            inside = false;
        } else if !inside {
            out.push(c);
        }
    }
    out.trim().to_string()
}

impl TvMazeProvider {
    pub fn new() -> Self {
        Self::with_base_url(TVMAZE_DEFAULT_BASE_URL)
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::builder()
                .user_agent("oxarr/0.1 (+https://github.com/bubuntux/oxarr)")
                .build()
                .unwrap_or_default(),
            base_url: base_url.into(),
        }
    }

    fn map_show_to_metadata(&self, show: TvMazeShow) -> SeriesMetadata {
        let mut external_ids = BTreeMap::new();
        external_ids.insert("tvmaze".to_owned(), show.id.to_string());

        if let Some(externals) = show.externals {
            if let Some(imdb) = externals.imdb.filter(|s| !s.is_empty()) {
                external_ids.insert("imdb".to_owned(), imdb);
            }
            if let Some(thetvdb) = externals.thetvdb {
                external_ids.insert("thetvdb".to_owned(), thetvdb.to_string());
            }
        }

        let year = show.premiered.as_deref().and_then(|p| {
            NaiveDate::parse_from_str(p, "%Y-%m-%d")
                .ok()
                .map(|d| d.format("%Y").to_string().parse::<u16>().ok())
                .flatten()
        });

        let overview = show.summary.map(|s| strip_html_tags(&s));
        let poster_url = show.image.as_ref().and_then(|img| img.original.clone().or_else(|| img.medium.clone()));
        let backdrop_url = show.image.and_then(|img| img.original);
        let network = show.network.map(|n| n.name).or_else(|| show.web_channel.map(|w| w.name));

        SeriesMetadata {
            provider_id: format!("tvmaze:{}", show.id),
            external_ids,
            title: show.name,
            original_title: None,
            original_language: None,
            overview,
            poster_url,
            backdrop_url,
            genres: show.genres.unwrap_or_default(),
            year,
            status: show.status,
            network,
        }
    }
}

impl Default for TvMazeProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MetadataProvider for TvMazeProvider {
    fn id(&self) -> &str {
        "tvmaze"
    }

    fn supported_kinds(&self) -> &[EntityKind] {
        static KINDS: [EntityKind; 1] = [EntityKind::Series];
        &KINDS
    }

    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let url = format!("{}/search/shows", self.base_url);
        let resp = self
            .client
            .get(&url)
            .query(&[("q", &query.query)])
            .send()
            .await
            .with_context(|| format!("failed to query TVMaze search for '{}'", query.query))?;

        if !resp.status().is_success() {
            return Ok(Vec::new());
        }

        let items: Vec<TvMazeSearchItem> = resp.json().await.unwrap_or_default();
        let results = items
            .into_iter()
            .map(|item| {
                let show = item.show;
                let year = show.premiered.as_deref().and_then(|p| {
                    p.split('-').next().and_then(|y| y.parse::<u16>().ok())
                });
                let overview = show.summary.map(|s| strip_html_tags(&s));
                let poster_url = show.image.and_then(|img| img.medium.or(img.original));

                SearchResult {
                    provider_id: format!("tvmaze:{}", show.id),
                    entity_kind: EntityKind::Series,
                    title: show.name,
                    year,
                    overview,
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
        if *kind != EntityKind::Series {
            anyhow::bail!("TVMaze only supports Series entity kind");
        }

        let url = if external_id.starts_with("tt") {
            format!("{}/lookup/shows?imdb={}", self.base_url, external_id)
        } else if external_id.chars().all(|c| c.is_ascii_digit()) {
            format!("{}/shows/{}", self.base_url, external_id)
        } else {
            anyhow::bail!("unsupported TVMaze identifier format: {}", external_id);
        };

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("failed to fetch TVMaze show details for {}", external_id))?;

        if !resp.status().is_success() {
            anyhow::bail!("TVMaze show not found for {}", external_id);
        }

        let show: TvMazeShow = resp.json().await?;
        let metadata = self.map_show_to_metadata(show);
        Ok(ProviderMediaMetadata::Series(Box::new(metadata)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_html_tags_from_summary() {
        let raw = "<p><b>Breaking Bad</b> is a show about chemistry.</p>";
        assert_eq!(strip_html_tags(raw), "Breaking Bad is a show about chemistry.");
    }

    #[test]
    fn parses_tvmaze_show_json() {
        let raw = r#"{
            "id": 139,
            "name": "Breaking Bad",
            "premiered": "2008-01-20",
            "summary": "<p>A high school chemistry teacher turns to crime.</p>",
            "genres": ["Drama", "Crime"],
            "status": "Ended",
            "image": {
                "medium": "https://example.com/medium.jpg",
                "original": "https://example.com/original.jpg"
            },
            "network": { "name": "AMC" },
            "externals": {
                "imdb": "tt0903747",
                "thetvdb": 81189
            }
        }"#;

        let show: TvMazeShow = serde_json::from_str(raw).expect("deserializes show");
        let provider = TvMazeProvider::new();
        let meta = provider.map_show_to_metadata(show);

        assert_eq!(meta.title, "Breaking Bad");
        assert_eq!(meta.year, Some(2008));
        assert_eq!(meta.status.as_deref(), Some("Ended"));
        assert_eq!(meta.network.as_deref(), Some("AMC"));
        assert_eq!(meta.genres, vec!["Drama", "Crime"]);
        assert_eq!(meta.external_ids.get("tvmaze").map(String::as_str), Some("139"));
        assert_eq!(meta.external_ids.get("imdb").map(String::as_str), Some("tt0903747"));
        assert_eq!(meta.external_ids.get("thetvdb").map(String::as_str), Some("81189"));
        assert_eq!(
            meta.overview.as_deref(),
            Some("A high school chemistry teacher turns to crime.")
        );
    }
}
