use anyhow::{Context, Result};
use sea_orm::sea_query::OnConflict;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, Set, TryInsertResult};

use super::models::session;
use super::Database;

impl Database {
    /// Inserts a new session record if its ID is unused. Returns `Ok(false)` on ID collision.
    pub async fn insert_session(&self, id: &str, data: &str, expiry_unix: i64) -> Result<bool> {
        let active = session::ActiveModel {
            id: Set(id.to_owned()),
            data: Set(data.to_owned()),
            expiry_date: Set(expiry_unix),
        };

        let res = session::Entity::insert(active)
            .on_conflict_do_nothing()
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to insert session '{id}'"))?;

        Ok(matches!(res, TryInsertResult::Inserted(_)))
    }

    /// Inserts or updates a session record by ID.
    pub async fn upsert_session(&self, id: &str, data: &str, expiry_unix: i64) -> Result<()> {
        let active = session::ActiveModel {
            id: Set(id.to_owned()),
            data: Set(data.to_owned()),
            expiry_date: Set(expiry_unix),
        };

        session::Entity::insert(active)
            .on_conflict(
                OnConflict::column(session::Column::Id)
                    .update_columns([session::Column::Data, session::Column::ExpiryDate])
                    .to_owned(),
            )
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to upsert session '{id}'"))?;

        Ok(())
    }

    /// Fetches serialized session data for `id` if it has not expired at `now_unix`.
    pub async fn get_session(&self, id: &str, now_unix: i64) -> Result<Option<String>> {
        let Some(model) = session::Entity::find_by_id(id.to_owned())
            .one(&self.conn)
            .await
            .with_context(|| format!("failed to fetch session '{id}'"))?
        else {
            return Ok(None);
        };

        if model.expiry_date <= now_unix {
            return Ok(None);
        }

        Ok(Some(model.data))
    }

    /// Deletes a session record by ID. Returns `Ok(true)` if a row was removed.
    pub async fn delete_session(&self, id: &str) -> Result<bool> {
        let res = session::Entity::delete_by_id(id.to_owned())
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to delete session '{id}'"))?;

        Ok(res.rows_affected > 0)
    }

    /// Removes all sessions whose `expiry_date` is at or before `now_unix`.
    pub async fn delete_expired_sessions(&self, now_unix: i64) -> Result<u64> {
        let res = session::Entity::delete_many()
            .filter(session::Column::ExpiryDate.lte(now_unix))
            .exec(&self.conn)
            .await
            .context("failed to prune expired sessions")?;

        Ok(res.rows_affected)
    }
}
