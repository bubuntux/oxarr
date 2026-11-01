use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use oxarr_types::MediaAsset;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait,
    Set, TransactionTrait,
};
use uuid::Uuid;

use super::models::json_types::UuidList;
use super::models::{entity_asset, media_asset};
use super::Database;

async fn hydrate_assets<C: ConnectionTrait>(
    conn: &C,
    models: Vec<media_asset::Model>,
) -> Result<Vec<MediaAsset>> {
    if models.is_empty() {
        return Ok(Vec::new());
    }

    let mut entity_ids_by_asset: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for link in entity_asset::Entity::find()
        .filter(entity_asset::Column::AssetId.is_in(models.iter().map(|model| model.id)))
        .order_by_asc(entity_asset::Column::Position)
        .order_by_asc(entity_asset::Column::EntityId)
        .all(conn)
        .await?
    {
        entity_ids_by_asset
            .entry(link.asset_id)
            .or_default()
            .push(link.entity_id);
    }

    let mut out = Vec::with_capacity(models.len());

    for model in models {
        let size_bytes = u64::try_from(model.size_bytes)
            .with_context(|| format!("negative size_bytes on media asset {}", model.id))?;
        let entity_ids = entity_ids_by_asset.remove(&model.id).unwrap_or_default();

        out.push(MediaAsset {
            id: model.id,
            entity_ids,
            path: model.path,
            size_bytes,
            matched_classifier_ids: model.matched_classifier_ids_json.0,
            quality_score: model.quality_score,
            media_info: model.media_info_json,
            imported_at: model.imported_at,
        });
    }

    Ok(out)
}

impl Database {
    pub async fn insert_media_asset(&self, asset: &MediaAsset) -> Result<()> {
        if asset.entity_ids.is_empty() {
            bail!(
                "media asset '{}' must reference at least one entity",
                asset.path
            );
        }

        let size_bytes = i64::try_from(asset.size_bytes)
            .context("media asset size_bytes exceeds SQLite i64::MAX")?;

        let tx = self.conn.begin().await?;

        let active = media_asset::ActiveModel {
            id: Set(asset.id),
            path: Set(asset.path.clone()),
            size_bytes: Set(size_bytes),
            quality_score: Set(asset.quality_score),
            matched_classifier_ids_json: Set(UuidList(asset.matched_classifier_ids.clone())),
            media_info_json: Set(asset.media_info.clone()),
            imported_at: Set(asset.imported_at),
        };
        media_asset::Entity::insert(active)
            .exec(&tx)
            .await
            .with_context(|| format!("failed to insert media asset '{}'", asset.path))?;

        let links: Vec<entity_asset::ActiveModel> = asset
            .entity_ids
            .iter()
            .enumerate()
            .map(|(idx, entity_id)| {
                let pos = i32::try_from(idx).context("entity_ids position overflow")?;
                Ok(entity_asset::ActiveModel {
                    entity_id: Set(*entity_id),
                    asset_id: Set(asset.id),
                    position: Set(pos),
                })
            })
            .collect::<Result<_>>()?;

        entity_asset::Entity::insert_many(links)
            .exec(&tx)
            .await
            .with_context(|| format!("failed to link entities to media asset {}", asset.id))?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_media_asset(&self, id: Uuid) -> Result<Option<MediaAsset>> {
        let Some(model) = media_asset::Entity::find_by_id(id)
            .one(&self.conn)
            .await
            .with_context(|| format!("failed to fetch media asset {id}"))?
        else {
            return Ok(None);
        };

        let mut hydrated = hydrate_assets(&self.conn, vec![model]).await?;
        Ok(hydrated.pop())
    }

    pub async fn list_entity_media_assets(&self, entity_id: Uuid) -> Result<Vec<MediaAsset>> {
        let models = media_asset::Entity::find()
            .filter(
                media_asset::Column::Id.in_subquery(
                    entity_asset::Entity::find()
                        .select_only()
                        .column(entity_asset::Column::AssetId)
                        .filter(entity_asset::Column::EntityId.eq(entity_id))
                        .into_query(),
                ),
            )
            .order_by_asc(media_asset::Column::ImportedAt)
            .all(&self.conn)
            .await
            .with_context(|| format!("failed to list media assets for entity {entity_id}"))?;

        hydrate_assets(&self.conn, models).await
    }

    pub async fn link_media_asset_to_entity(&self, asset_id: Uuid, entity_id: Uuid) -> Result<()> {
        let existing_links = entity_asset::Entity::find()
            .filter(entity_asset::Column::AssetId.eq(asset_id))
            .all(&self.conn)
            .await?;
        let next_pos = existing_links
            .iter()
            .map(|link| link.position)
            .max()
            .map_or(0, |max_pos| max_pos.saturating_add(1));

        let active = entity_asset::ActiveModel {
            entity_id: Set(entity_id),
            asset_id: Set(asset_id),
            position: Set(next_pos),
        };
        entity_asset::Entity::insert(active)
            .exec(&self.conn)
            .await
            .with_context(|| {
                format!("failed to link media asset {asset_id} to entity {entity_id}")
            })?;

        Ok(())
    }

    /// Removes the link between `asset_id` and `entity_id`. If no entities remain linked,
    /// the database trigger automatically deletes the orphaned `media_assets` row.
    pub async fn unlink_media_asset_from_entity(
        &self,
        asset_id: Uuid,
        entity_id: Uuid,
    ) -> Result<bool> {
        let res = entity_asset::Entity::delete_by_id((entity_id, asset_id))
            .exec(&self.conn)
            .await
            .with_context(|| {
                format!("failed to unlink media asset {asset_id} from entity {entity_id}")
            })?;

        Ok(res.rows_affected > 0)
    }

    pub async fn delete_media_asset(&self, id: Uuid) -> Result<bool> {
        let res = media_asset::Entity::delete_by_id(id)
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to delete media asset {id}"))?;

        Ok(res.rows_affected > 0)
    }
}
