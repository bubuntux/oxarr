use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use oxarr_types::{TrackedRelease, TrackedReleaseStatus};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use uuid::Uuid;

use super::models::json_types::UuidList;
use super::models::tracked_release;
use super::Database;

fn into_domain(model: tracked_release::Model) -> Result<TrackedRelease> {
    let size_bytes = u64::try_from(model.size_bytes)
        .with_context(|| format!("negative size_bytes on tracked release {}", model.id))?;

    Ok(TrackedRelease {
        id: model.id,
        entity_id: model.entity_id,
        indexer: model.indexer,
        guid: model.guid,
        title: model.title,
        download_url: model.download_url,
        info_hash: model.info_hash,
        size_bytes,
        score: model.score,
        matched_classifier_ids: model.matched_classifier_ids_json.0,
        status: model.status.into(),
        download_client_id: model.download_client_id,
        published_at: model.published_at,
        created_at: model.created_at,
        updated_at: model.updated_at,
    })
}

impl Database {
    pub async fn insert_tracked_release(&self, release: &TrackedRelease) -> Result<()> {
        let size_bytes = i64::try_from(release.size_bytes)
            .context("tracked release size_bytes exceeds SQLite i64::MAX")?;

        let active = tracked_release::ActiveModel {
            id: Set(release.id),
            entity_id: Set(release.entity_id),
            indexer: Set(release.indexer.clone()),
            guid: Set(release.guid.clone()),
            title: Set(release.title.clone()),
            download_url: Set(release.download_url.clone()),
            info_hash: Set(release.info_hash.clone()),
            size_bytes: Set(size_bytes),
            score: Set(release.score),
            matched_classifier_ids_json: Set(UuidList(release.matched_classifier_ids.clone())),
            status: Set(release.status.into()),
            download_client_id: Set(release.download_client_id.clone()),
            published_at: Set(release.published_at),
            created_at: Set(release.created_at),
            updated_at: Set(release.updated_at),
        };

        tracked_release::Entity::insert(active)
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to insert tracked release '{}'", release.title))?;

        Ok(())
    }

    pub async fn get_tracked_release(&self, id: Uuid) -> Result<Option<TrackedRelease>> {
        let model = tracked_release::Entity::find_by_id(id)
            .one(&self.conn)
            .await
            .with_context(|| format!("failed to fetch tracked release {id}"))?;

        model.map(into_domain).transpose()
    }

    pub async fn get_tracked_release_by_guid(
        &self,
        indexer: &str,
        guid: &str,
    ) -> Result<Option<TrackedRelease>> {
        let model = tracked_release::Entity::find()
            .filter(tracked_release::Column::Indexer.eq(indexer))
            .filter(tracked_release::Column::Guid.eq(guid))
            .one(&self.conn)
            .await
            .with_context(|| format!("failed to fetch tracked release ({indexer}, {guid})"))?;

        model.map(into_domain).transpose()
    }

    pub async fn list_entity_tracked_releases(
        &self,
        entity_id: Uuid,
    ) -> Result<Vec<TrackedRelease>> {
        let models = tracked_release::Entity::find()
            .filter(tracked_release::Column::EntityId.eq(entity_id))
            .order_by_desc(tracked_release::Column::CreatedAt)
            .all(&self.conn)
            .await
            .with_context(|| format!("failed to list tracked releases for entity {entity_id}"))?;

        models.into_iter().map(into_domain).collect()
    }

    pub async fn update_tracked_release_status(
        &self,
        id: Uuid,
        status: TrackedReleaseStatus,
        download_client_id: Option<&str>,
        updated_at: DateTime<Utc>,
    ) -> Result<bool> {
        let Some(existing) = tracked_release::Entity::find_by_id(id)
            .one(&self.conn)
            .await?
        else {
            return Ok(false);
        };

        let mut active: tracked_release::ActiveModel = existing.into();
        if let Some(dl_id) = download_client_id {
            active.download_client_id = Set(Some(dl_id.to_owned()));
        }
        active.status = Set(status.into());
        active.updated_at = Set(updated_at);

        active
            .update(&self.conn)
            .await
            .with_context(|| format!("failed to update tracked release {id}"))?;

        Ok(true)
    }

    pub async fn delete_tracked_release(&self, id: Uuid) -> Result<bool> {
        let res = tracked_release::Entity::delete_by_id(id)
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to delete tracked release {id}"))?;

        Ok(res.rows_affected > 0)
    }
}
