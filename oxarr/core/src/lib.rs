use anyhow::Result;
use async_trait::async_trait;
use oxarr_types::{MediaDetails, SearchQuery, SearchResult};

#[async_trait]
pub trait MetadataProvider: Send + Sync {
    fn id(&self) -> &str;
    async fn search(&self, query: &SearchQuery) -> Result<Vec<SearchResult>>;
    async fn fetch_details(&self, external_id: &str) -> Result<MediaDetails>;
}
