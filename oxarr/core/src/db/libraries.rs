use anyhow::{Context, Result};
use oxarr_types::Library;
use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, Set};
use uuid::Uuid;

use super::models::library;
use super::Database;

fn into_domain(model: library::Model) -> Library {
    Library {
        id: model.id,
        name: model.name,
        kind: model.kind.into(),
        root_path: model.root_path,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

impl Database {
    pub async fn insert_library(&self, lib: &Library) -> Result<()> {
        let active = library::ActiveModel {
            id: Set(lib.id),
            name: Set(lib.name.clone()),
            kind: Set(lib.kind.into()),
            root_path: Set(lib.root_path.clone()),
            created_at: Set(lib.created_at),
            updated_at: Set(lib.updated_at),
        };

        library::Entity::insert(active)
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to insert library '{}'", lib.name))?;

        Ok(())
    }

    pub async fn get_library(&self, id: Uuid) -> Result<Option<Library>> {
        let model = library::Entity::find_by_id(id)
            .one(&self.conn)
            .await
            .with_context(|| format!("failed to fetch library {id}"))?;

        Ok(model.map(into_domain))
    }

    pub async fn list_libraries(&self) -> Result<Vec<Library>> {
        let models = library::Entity::find()
            .order_by_asc(library::Column::Name)
            .all(&self.conn)
            .await
            .context("failed to list libraries")?;

        Ok(models.into_iter().map(into_domain).collect())
    }

    pub async fn update_library(&self, lib: &Library) -> Result<bool> {
        let Some(existing) = library::Entity::find_by_id(lib.id).one(&self.conn).await? else {
            return Ok(false);
        };

        let mut active: library::ActiveModel = existing.into();
        active.name = Set(lib.name.clone());
        active.kind = Set(lib.kind.into());
        active.root_path = Set(lib.root_path.clone());
        active.updated_at = Set(lib.updated_at);

        active
            .update(&self.conn)
            .await
            .with_context(|| format!("failed to update library {}", lib.id))?;

        Ok(true)
    }

    pub async fn delete_library(&self, id: Uuid) -> Result<bool> {
        let res = library::Entity::delete_by_id(id)
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to delete library {id}"))?;

        Ok(res.rows_affected > 0)
    }
}
