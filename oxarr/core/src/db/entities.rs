use std::collections::BTreeMap;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use oxarr_types::{CollectionDetails, Entity, EntityDetails, EntityKind, MovieDetails};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, LoaderTrait,
    QueryFilter, QueryOrder, Set, TransactionTrait,
};
use uuid::Uuid;

use super::models::enums::DbEntityKind;
use super::models::json_types::DbEntityMetadata;
use super::models::{collection, entity, entity_external_id, movie};
use super::Database;

pub(super) async fn hydrate_entities<C: ConnectionTrait>(
    conn: &C,
    base_models: Vec<entity::Model>,
) -> Result<Vec<Entity>> {
    if base_models.is_empty() {
        return Ok(Vec::new());
    }

    let has_movies = base_models.iter().any(|m| m.kind == DbEntityKind::Movie);
    let has_collections = base_models
        .iter()
        .any(|m| m.kind == DbEntityKind::Collection);

    let movie_models = if has_movies {
        base_models.load_one(movie::Entity, conn).await?
    } else {
        vec![None; base_models.len()]
    };

    let collection_models = if has_collections {
        base_models.load_one(collection::Entity, conn).await?
    } else {
        vec![None; base_models.len()]
    };

    let ext_id_groups = base_models
        .load_many(entity_external_id::Entity, conn)
        .await?;

    let mut out = Vec::with_capacity(base_models.len());

    for (((base, movie_opt), collection_opt), ext_rows) in base_models
        .into_iter()
        .zip(movie_models)
        .zip(collection_models)
        .zip(ext_id_groups)
    {
        let id = base.id;
        let details = match base.kind {
            DbEntityKind::Movie => {
                let m = movie_opt
                    .with_context(|| format!("missing movies row for movie entity {id}"))?;
                let year = m
                    .year
                    .map(u16::try_from)
                    .transpose()
                    .with_context(|| format!("out-of-range year on movie {id}"))?;
                let runtime_minutes = m
                    .runtime_minutes
                    .map(u32::try_from)
                    .transpose()
                    .with_context(|| format!("negative runtime_minutes on movie {id}"))?;
                EntityDetails::Movie(MovieDetails {
                    year,
                    in_cinemas_date: m.in_cinemas_date,
                    digital_release_date: m.digital_release_date,
                    physical_release_date: m.physical_release_date,
                    runtime_minutes,
                    certification: m.certification,
                    studio: m.studio,
                })
            }
            DbEntityKind::Collection => {
                let _c = collection_opt.with_context(|| {
                    format!("missing collections row for collection entity {id}")
                })?;
                EntityDetails::Collection(CollectionDetails {})
            }
            DbEntityKind::Series => {
                EntityDetails::Series(oxarr_types::SeriesDetails::default())
            }
        };

        let external_ids: BTreeMap<String, String> = ext_rows
            .into_iter()
            .map(|row| (row.provider, row.external_id))
            .collect();

        out.push(Entity {
            id,
            kind: base.kind.into(),
            library_id: base.library_id,
            title: base.title,
            original_title: base.original_title,
            original_language: base.original_language,
            monitored: base.monitored,
            external_ids,
            metadata: base.metadata_json.0,
            details,
            created_at: base.created_at,
            updated_at: base.updated_at,
        });
    }

    Ok(out)
}

async fn insert_subtype_in_tx(
    tx: &DatabaseTransaction,
    entity_id: Uuid,
    details: &EntityDetails,
) -> Result<()> {
    match details {
        EntityDetails::Movie(m) => {
            let runtime_minutes = m
                .runtime_minutes
                .map(i32::try_from)
                .transpose()
                .context("movie runtime_minutes exceeds i32::MAX")?;
            let active = movie::ActiveModel {
                entity_id: Set(entity_id),
                year: Set(m.year.map(i32::from)),
                in_cinemas_date: Set(m.in_cinemas_date),
                digital_release_date: Set(m.digital_release_date),
                physical_release_date: Set(m.physical_release_date),
                runtime_minutes: Set(runtime_minutes),
                certification: Set(m.certification.clone()),
                studio: Set(m.studio.clone()),
            };
            movie::Entity::insert(active)
                .exec(tx)
                .await
                .with_context(|| format!("failed to insert movie details for {entity_id}"))?;
        }
        EntityDetails::Collection(_c) => {
            let active = collection::ActiveModel {
                entity_id: Set(entity_id),
            };
            collection::Entity::insert(active)
                .exec(tx)
                .await
                .with_context(|| format!("failed to insert collection details for {entity_id}"))?;
        }
        EntityDetails::Series(_s) => {}
    }
    Ok(())
}

async fn insert_external_ids_in_tx(
    tx: &DatabaseTransaction,
    entity_id: Uuid,
    kind: EntityKind,
    external_ids: &BTreeMap<String, String>,
) -> Result<()> {
    let entity_kind: DbEntityKind = kind.into();
    for (provider, external_id) in external_ids {
        let active = entity_external_id::ActiveModel {
            entity_id: Set(entity_id),
            provider: Set(provider.clone()),
            entity_kind: Set(entity_kind),
            external_id: Set(external_id.clone()),
        };
        entity_external_id::Entity::insert(active)
            .exec(tx)
            .await
            .with_context(|| {
                format!(
                    "failed to insert external_id '{provider}:{external_id}' for entity {entity_id}"
                )
            })?;
    }
    Ok(())
}

impl Database {
    /// Inserts an entity, its subtype record (movies/collections), and its external IDs
    /// atomically inside a transaction.
    pub async fn insert_entity(&self, item: &Entity) -> Result<()> {
        let tx = self.conn.begin().await?;

        let base = entity::ActiveModel {
            id: Set(item.id),
            library_id: Set(item.library_id),
            kind: Set(item.kind.into()),
            title: Set(item.title.clone()),
            original_title: Set(item.original_title.clone()),
            original_language: Set(item.original_language.clone()),
            monitored: Set(item.monitored),
            metadata_json: Set(DbEntityMetadata(item.metadata.clone())),
            created_at: Set(item.created_at),
            updated_at: Set(item.updated_at),
        };

        entity::Entity::insert(base)
            .exec(&tx)
            .await
            .with_context(|| format!("failed to insert entity base record for {}", item.id))?;

        insert_subtype_in_tx(&tx, item.id, &item.details).await?;
        insert_external_ids_in_tx(&tx, item.id, item.kind, &item.external_ids).await?;

        tx.commit().await?;
        Ok(())
    }

    /// Inserts multiple entities in a single atomic database transaction.
    pub async fn insert_entities_batch(&self, items: &[Entity]) -> Result<()> {
        let tx = self.conn.begin().await?;

        for item in items {
            let base = entity::ActiveModel {
                id: Set(item.id),
                library_id: Set(item.library_id),
                kind: Set(item.kind.into()),
                title: Set(item.title.clone()),
                original_title: Set(item.original_title.clone()),
                original_language: Set(item.original_language.clone()),
                monitored: Set(item.monitored),
                metadata_json: Set(DbEntityMetadata(item.metadata.clone())),
                created_at: Set(item.created_at),
                updated_at: Set(item.updated_at),
            };

            entity::Entity::insert(base)
                .exec(&tx)
                .await
                .with_context(|| format!("failed to insert batch entity base record {}", item.id))?;

            insert_subtype_in_tx(&tx, item.id, &item.details).await?;
            insert_external_ids_in_tx(&tx, item.id, item.kind, &item.external_ids).await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Fetches an entity by ID, including its subtype row and external IDs.
    pub async fn get_entity(&self, id: Uuid) -> Result<Option<Entity>> {
        let Some(base) = entity::Entity::find_by_id(id)
            .one(&self.conn)
            .await
            .with_context(|| format!("failed to get entity {id}"))?
        else {
            return Ok(None);
        };

        let hydrated = hydrate_entities(&self.conn, vec![base]).await?;
        Ok(hydrated.into_iter().next())
    }

    /// Resolves an entity given a provider identifier scoped to its kind.
    pub async fn get_entity_by_external_id(
        &self,
        kind: EntityKind,
        provider: &str,
        external_id: &str,
    ) -> Result<Option<Entity>> {
        let db_kind = DbEntityKind::from(kind);
        let row = entity_external_id::Entity::find()
            .filter(entity_external_id::Column::Provider.eq(provider))
            .filter(entity_external_id::Column::EntityKind.eq(db_kind))
            .filter(entity_external_id::Column::ExternalId.eq(external_id))
            .one(&self.conn)
            .await
            .with_context(|| {
                format!("failed to query external id {provider}:{kind:?}:{external_id}")
            })?;

        let Some(row) = row else {
            return Ok(None);
        };

        self.get_entity(row.entity_id).await
    }

    /// Finds all entities associated with a provider identifier regardless of `EntityKind`.
    pub async fn find_entities_by_external_id(
        &self,
        provider: &str,
        external_id: &str,
    ) -> Result<Vec<Entity>> {
        let ext_rows = entity_external_id::Entity::find()
            .filter(entity_external_id::Column::Provider.eq(provider))
            .filter(entity_external_id::Column::ExternalId.eq(external_id))
            .all(&self.conn)
            .await
            .with_context(|| {
                format!("failed to query entities for external id {provider}:{external_id}")
            })?;

        if ext_rows.is_empty() {
            return Ok(Vec::new());
        }

        let ids: Vec<Uuid> = ext_rows.into_iter().map(|r| r.entity_id).collect();
        let base_models = entity::Entity::find()
            .filter(entity::Column::Id.is_in(ids))
            .all(&self.conn)
            .await?;

        hydrate_entities(&self.conn, base_models).await
    }

    /// Lists entities in a library, ordered by title and id.
    pub async fn list_entities_in_library(&self, library_id: Uuid) -> Result<Vec<Entity>> {
        let models = entity::Entity::find()
            .filter(entity::Column::LibraryId.eq(library_id))
            .order_by_asc(entity::Column::Title)
            .order_by_asc(entity::Column::Id)
            .all(&self.conn)
            .await
            .with_context(|| format!("failed to list entities in library {library_id}"))?;

        hydrate_entities(&self.conn, models).await
    }

    /// Updates an existing entity of the same `EntityKind`. Returns `Ok(false)` if no row
    /// with `(id, kind)` exists.
    pub async fn update_entity(&self, item: &Entity) -> Result<bool> {
        let tx = self.conn.begin().await?;
        let kind = item.kind;

        let Some(existing) = entity::Entity::find_by_id(item.id)
            .filter(entity::Column::Kind.eq(DbEntityKind::from(kind)))
            .one(&tx)
            .await?
        else {
            return Ok(false);
        };

        let mut active: entity::ActiveModel = existing.into();
        active.library_id = Set(item.library_id);
        active.title = Set(item.title.clone());
        active.original_title = Set(item.original_title.clone());
        active.original_language = Set(item.original_language.clone());
        active.monitored = Set(item.monitored);
        active.metadata_json = Set(DbEntityMetadata(item.metadata.clone()));
        active.updated_at = Set(item.updated_at);
        active.update(&tx).await?;

        match &item.details {
            EntityDetails::Movie(m) => {
                let runtime_minutes = m
                    .runtime_minutes
                    .map(i32::try_from)
                    .transpose()
                    .context("movie runtime_minutes exceeds i32::MAX")?;
                let active_movie = movie::ActiveModel {
                    entity_id: Set(item.id),
                    year: Set(m.year.map(i32::from)),
                    in_cinemas_date: Set(m.in_cinemas_date),
                    digital_release_date: Set(m.digital_release_date),
                    physical_release_date: Set(m.physical_release_date),
                    runtime_minutes: Set(runtime_minutes),
                    certification: Set(m.certification.clone()),
                    studio: Set(m.studio.clone()),
                };
                active_movie.update(&tx).await?;
            }
            EntityDetails::Collection(_c) => {
                let active_col = collection::ActiveModel {
                    entity_id: Set(item.id),
                };
                active_col.update(&tx).await?;
            }
            EntityDetails::Series(_s) => {}
        }

        entity_external_id::Entity::delete_many()
            .filter(entity_external_id::Column::EntityId.eq(item.id))
            .exec(&tx)
            .await?;
        insert_external_ids_in_tx(&tx, item.id, kind, &item.external_ids).await?;

        tx.commit().await?;
        Ok(true)
    }

    pub async fn set_entity_monitored(
        &self,
        id: Uuid,
        monitored: bool,
        updated_at: DateTime<Utc>,
    ) -> Result<bool> {
        let Some(existing) = entity::Entity::find_by_id(id).one(&self.conn).await? else {
            return Ok(false);
        };

        let mut active: entity::ActiveModel = existing.into();
        active.monitored = Set(monitored);
        active.updated_at = Set(updated_at);
        active
            .update(&self.conn)
            .await
            .with_context(|| format!("failed to update monitored state on entity {id}"))?;

        Ok(true)
    }

    pub async fn delete_entity(&self, id: Uuid) -> Result<bool> {
        let res = entity::Entity::delete_by_id(id)
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to delete entity {id}"))?;

        Ok(res.rows_affected > 0)
    }
}
