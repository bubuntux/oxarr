use anyhow::{Context, Result};
use oxarr_types::{Entity, EntityDetails};
use sea_orm::sea_query::{OnConflict, SelectStatement};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, QueryTrait, Set};
use uuid::Uuid;

use super::entities::hydrate_entities;
use super::models::{collection_movie, entity};
use super::Database;

/// Subquery selecting the entity IDs of every movie that belongs to `collection_id`.
fn movie_ids_in_collection(collection_id: Uuid) -> SelectStatement {
    collection_movie::Entity::find()
        .select_only()
        .column(collection_movie::Column::MovieId)
        .filter(collection_movie::Column::CollectionId.eq(collection_id))
        .into_query()
}

/// Subquery selecting the entity IDs of every collection that contains `movie_id`.
pub(super) fn collection_ids_of_movie(movie_id: Uuid) -> SelectStatement {
    collection_movie::Entity::find()
        .select_only()
        .column(collection_movie::Column::CollectionId)
        .filter(collection_movie::Column::MovieId.eq(movie_id))
        .into_query()
}

impl Database {
    /// Links a movie to a movie collection. Idempotent if the movie is already in the collection;
    /// fails with a foreign-key error if either ID does not reference a valid `Collection` / `Movie`.
    pub async fn add_movie_to_collection(&self, collection_id: Uuid, movie_id: Uuid) -> Result<()> {
        let active = collection_movie::ActiveModel {
            collection_id: Set(collection_id),
            movie_id: Set(movie_id),
        };

        collection_movie::Entity::insert(active)
            .on_conflict(
                OnConflict::columns([
                    collection_movie::Column::CollectionId,
                    collection_movie::Column::MovieId,
                ])
                .do_nothing()
                .to_owned(),
            )
            .exec_without_returning(&self.conn)
            .await
            .with_context(|| {
                format!("failed to add movie {movie_id} to collection {collection_id}")
            })?;

        Ok(())
    }

    pub async fn remove_movie_from_collection(
        &self,
        collection_id: Uuid,
        movie_id: Uuid,
    ) -> Result<bool> {
        let res = collection_movie::Entity::delete_by_id((collection_id, movie_id))
            .exec(&self.conn)
            .await
            .with_context(|| {
                format!("failed to remove movie {movie_id} from collection {collection_id}")
            })?;

        Ok(res.rows_affected > 0)
    }

    pub async fn list_collection_movies(&self, collection_id: Uuid) -> Result<Vec<Entity>> {
        let base_models = entity::Entity::find()
            .filter(entity::Column::Id.in_subquery(movie_ids_in_collection(collection_id)))
            .all(&self.conn)
            .await
            .with_context(|| format!("failed to list movies in collection {collection_id}"))?;

        let mut movies = hydrate_entities(&self.conn, base_models).await?;
        movies.sort_by(|a, b| chronological_movie_key(a).cmp(&chronological_movie_key(b)));

        Ok(movies)
    }

    pub async fn list_movie_collections(&self, movie_id: Uuid) -> Result<Vec<Entity>> {
        let base_models = entity::Entity::find()
            .filter(entity::Column::Id.in_subquery(collection_ids_of_movie(movie_id)))
            .order_by_asc(entity::Column::Title)
            .all(&self.conn)
            .await
            .with_context(|| format!("failed to list collections for movie {movie_id}"))?;

        hydrate_entities(&self.conn, base_models).await
    }
}

fn chronological_movie_key(entity: &Entity) -> (bool, u16, Option<chrono::NaiveDate>, &str) {
    match &entity.details {
        EntityDetails::Movie(m) => (
            m.year.is_none(),
            m.year.unwrap_or(u16::MAX),
            m.in_cinemas_date,
            entity.title.as_str(),
        ),
        EntityDetails::Collection(_) => (true, u16::MAX, None, entity.title.as_str()),
    }
}
