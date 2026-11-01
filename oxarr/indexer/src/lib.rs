use anyhow::Result;
use async_trait::async_trait;
use oxarr_types::{ReleaseCandidate, SearchQuery};

#[async_trait]
pub trait Indexer: Send + Sync {
    fn name(&self) -> &str;
    async fn search(&self, query: &SearchQuery) -> Result<Vec<ReleaseCandidate>>;
    async fn fetch_recent(&self) -> Result<Vec<ReleaseCandidate>>;
}
