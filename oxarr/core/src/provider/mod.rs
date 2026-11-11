pub mod aggregator;
pub mod tmdb;
pub mod tvmaze;

use anyhow::Result;
use async_trait::async_trait;
use oxarr_types::{EntityKind, ProviderMediaMetadata, SearchQuery, SearchResult};

pub use aggregator::MetadataAggregator;
pub use tmdb::TmdbProvider;
pub use tvmaze::TvMazeProvider;

/// Pluggable interface for external metadata lookup sources (native and Wasm).
#[async_trait]
pub trait MetadataProvider: Send + Sync {
    /// Unique provider identifier (e.g. `"tmdb"`, `"tvmaze"`, `"omdb"`, `"thetvdb"`).
    fn id(&self) -> &str;

    /// Supported media categories.
    fn supported_kinds(&self) -> &[EntityKind];

    /// Queries the provider for media matching a title and optional year/kind.
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>>;

    /// Fetches full canonical details for an item given an external identifier.
    async fn fetch_details(
        &self,
        kind: &EntityKind,
        external_id: &str,
    ) -> Result<ProviderMediaMetadata>;
}
