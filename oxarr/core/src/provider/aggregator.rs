use std::collections::BTreeMap;
use std::sync::Arc;

use anyhow::Result;
use oxarr_types::{
    EntityKind, MovieMetadata, ProviderMediaMetadata, SearchQuery, SearchResult, SeriesMetadata,
};

use super::MetadataProvider;

/// Coordinates multiple metadata providers to aggregate, cross-link, and merge canonical metadata.
#[derive(Default, Clone)]
pub struct MetadataAggregator {
    providers: Vec<Arc<dyn MetadataProvider>>,
}

impl MetadataAggregator {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Registers a provider with the aggregator.
    pub fn add_provider(&mut self, provider: Arc<dyn MetadataProvider>) {
        self.providers.push(provider);
    }

    /// Returns the registered providers.
    pub fn providers(&self) -> &[Arc<dyn MetadataProvider>] {
        &self.providers
    }

    /// Searches across all compatible providers in parallel and returns aggregated results.
    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
        let compatible: Vec<_> = self
            .providers
            .iter()
            .filter(|p| match &query.entity_kind {
                Some(kind) => p.supported_kinds().contains(kind),
                None => true,
            })
            .cloned()
            .collect();

        if compatible.is_empty() {
            return Ok(Vec::new());
        }

        let handles: Vec<_> = compatible
            .into_iter()
            .map(|p| {
                let q = query.clone();
                tokio::spawn(async move { p.search(&q).await })
            })
            .collect();

        let mut results = Vec::new();
        for handle in handles {
            if let Ok(Ok(items)) = handle.await {
                results.extend(items);
            }
        }

        // Deduplicate results by title + year
        let mut seen = std::collections::HashSet::new();
        results.retain(|r| {
            let key = format!("{}:{}:{:?}", r.title.to_lowercase(), r.year.unwrap_or(0), r.entity_kind);
            seen.insert(key)
        });

        Ok(results)
    }

    /// Resolves metadata from known external IDs and cross-enriches across providers.
    pub async fn fetch_and_aggregate(
        &self,
        kind: &EntityKind,
        external_ids: &BTreeMap<String, String>,
    ) -> Result<Option<ProviderMediaMetadata>> {
        let mut primary_meta: Option<ProviderMediaMetadata> = None;

        // Step 1: Find primary metadata using an ID directly recognized by a provider
        for provider in &self.providers {
            if !provider.supported_kinds().contains(kind) {
                continue;
            }

            if let Some(id_val) = external_ids.get(provider.id()) {
                if let Ok(meta) = provider.fetch_details(kind, id_val).await {
                    primary_meta = Some(meta);
                    break;
                }
            }
        }

        // Fallback: If no provider matched directly by its ID name, check for standard cross-IDs (e.g. imdb)
        if primary_meta.is_none() {
            if let Some(imdb_id) = external_ids.get("imdb") {
                for provider in &self.providers {
                    if !provider.supported_kinds().contains(kind) {
                        continue;
                    }
                    if let Ok(meta) = provider.fetch_details(kind, imdb_id).await {
                        primary_meta = Some(meta);
                        break;
                    }
                }
            }
        }

        let Some(mut result) = primary_meta else {
            return Ok(None);
        };

        // Step 2: Cross-enrich with secondary providers using all discovered external IDs
        let current_ids = result.external_ids().clone();
        for provider in &self.providers {
            if !provider.supported_kinds().contains(kind) {
                continue;
            }

            // If this provider's external ID is already known or if imdb ID is known, query it
            let query_id = current_ids
                .get(provider.id())
                .or_else(|| current_ids.get("imdb"));

            if let Some(id_val) = query_id {
                if let Ok(secondary) = provider.fetch_details(kind, id_val).await {
                    merge_metadata(&mut result, secondary);
                }
            }
        }

        Ok(Some(result))
    }

    /// Convenience workflow: searches providers, selects top hit, and aggregates full metadata.
    pub async fn resolve_and_aggregate(
        &self,
        title: &str,
        year: Option<u16>,
        kind: EntityKind,
    ) -> Result<Option<ProviderMediaMetadata>> {
        let query = SearchQuery {
            query: title.to_owned(),
            year,
            entity_kind: Some(kind),
        };

        let search_results = self.search(&query).await?;
        let Some(best_match) = search_results.into_iter().next() else {
            return Ok(None);
        };

        let mut ids = BTreeMap::new();
        if let Some((prov, id)) = best_match.provider_id.split_once(':') {
            ids.insert(prov.to_owned(), id.to_owned());
        }

        self.fetch_and_aggregate(&kind, &ids).await
    }
}

/// Merges secondary provider details into primary metadata without clobbering existing values.
fn merge_metadata(primary: &mut ProviderMediaMetadata, secondary: ProviderMediaMetadata) {
    match (primary, secondary) {
        (
            ProviderMediaMetadata::Movie(ref mut p),
            ProviderMediaMetadata::Movie(s),
        ) => {
            merge_movie(p, *s);
        }
        (
            ProviderMediaMetadata::Series(ref mut p),
            ProviderMediaMetadata::Series(s),
        ) => {
            merge_series(p, *s);
        }
        _ => {}
    }
}

fn merge_movie(primary: &mut MovieMetadata, secondary: MovieMetadata) {
    // Merge external IDs
    for (k, v) in secondary.external_ids {
        primary.external_ids.entry(k).or_insert(v);
    }
    // Fill missing overview
    if primary.overview.is_none() {
        primary.overview = secondary.overview;
    }
    // Fill missing images
    if primary.poster_url.is_none() {
        primary.poster_url = secondary.poster_url;
    }
    if primary.backdrop_url.is_none() {
        primary.backdrop_url = secondary.backdrop_url;
    }
    // Union genres
    for g in secondary.genres {
        if !primary.genres.contains(&g) {
            primary.genres.push(g);
        }
    }
}

fn merge_series(primary: &mut SeriesMetadata, secondary: SeriesMetadata) {
    // Merge external IDs
    for (k, v) in secondary.external_ids {
        primary.external_ids.entry(k).or_insert(v);
    }
    // Fill missing overview
    if primary.overview.is_none() {
        primary.overview = secondary.overview;
    }
    // Fill missing images
    if primary.poster_url.is_none() {
        primary.poster_url = secondary.poster_url;
    }
    if primary.backdrop_url.is_none() {
        primary.backdrop_url = secondary.backdrop_url;
    }
    // Union genres
    for g in secondary.genres {
        if !primary.genres.contains(&g) {
            primary.genres.push(g);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct MockMovieProvider {
        id: &'static str,
    }

    #[async_trait]
    impl MetadataProvider for MockMovieProvider {
        fn id(&self) -> &str {
            self.id
        }

        fn supported_kinds(&self) -> &[EntityKind] {
            &[EntityKind::Movie]
        }

        async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>> {
            Ok(vec![SearchResult {
                provider_id: format!("{}:100", self.id),
                entity_kind: EntityKind::Movie,
                title: query.query.clone(),
                year: query.year,
                overview: Some(format!("Overview from {}", self.id)),
                poster_url: Some(format!("https://example.com/{}.jpg", self.id)),
            }])
        }

        async fn fetch_details(
            &self,
            _kind: &EntityKind,
            external_id: &str,
        ) -> Result<ProviderMediaMetadata> {
            let mut external_ids = BTreeMap::new();
            external_ids.insert(self.id.to_owned(), external_id.to_owned());
            if self.id == "tmdb" {
                external_ids.insert("imdb".to_owned(), "tt0133093".to_owned());
            }

            Ok(ProviderMediaMetadata::Movie(Box::new(MovieMetadata {
                provider_id: format!("{}:{}", self.id, external_id),
                external_ids,
                title: "The Matrix".to_owned(),
                original_title: None,
                original_language: None,
                overview: Some(format!("Overview from {}", self.id)),
                poster_url: Some(format!("https://example.com/{}.jpg", self.id)),
                backdrop_url: None,
                genres: vec![format!("{}-Genre", self.id)],
                year: Some(1999),
                in_cinemas_date: None,
                digital_release_date: None,
                physical_release_date: None,
                runtime_minutes: Some(136),
                certification: None,
                studio: None,
                collection: None,
            })))
        }
    }

    #[tokio::test]
    async fn aggregates_and_merges_across_providers() {
        let mut aggregator = MetadataAggregator::new();
        aggregator.add_provider(Arc::new(MockMovieProvider { id: "tmdb" }));
        aggregator.add_provider(Arc::new(MockMovieProvider { id: "omdb" }));

        let mut ids = BTreeMap::new();
        ids.insert("tmdb".to_owned(), "603".to_owned());

        let result = aggregator
            .fetch_and_aggregate(&EntityKind::Movie, &ids)
            .await
            .expect("fetch and aggregate")
            .expect("found metadata");

        assert_eq!(result.title(), "The Matrix");
        assert_eq!(result.external_ids().get("tmdb").map(String::as_str), Some("603"));
        assert_eq!(result.external_ids().get("imdb").map(String::as_str), Some("tt0133093"));
        // Genres merged from primary and secondary
        assert!(result.genres().contains(&"tmdb-Genre".to_owned()));
    }
}
