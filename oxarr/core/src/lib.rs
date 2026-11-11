pub mod db;
pub mod scanner;

use anyhow::Result;
use async_trait::async_trait;
use oxarr_types::{EntityKind, ProviderMediaMetadata, SearchQuery, SearchResult};

pub use db::{new_id, Database};
pub use scanner::watcher::LibraryWatcher;
pub use scanner::{inspect_file, LibraryScanner, ScannedMediaFile};

#[async_trait]
pub trait MetadataProvider: Send + Sync {
    fn id(&self) -> &str;
    fn supported_kinds(&self) -> &[EntityKind];
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>>;
    async fn fetch_details(
        &self,
        kind: &EntityKind,
        external_id: &str,
    ) -> Result<ProviderMediaMetadata>;
}
