use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use oxarr_types::User;
use sea_orm::{
    ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};
use uuid::Uuid;

use super::models::{user, user_library};
use super::Database;

impl Database {
    /// Inserts a new user record.
    pub async fn insert_user(
        &self,
        id: Uuid,
        username: &str,
        password_hash: &str,
        is_admin: bool,
        now: DateTime<Utc>,
    ) -> Result<User> {
        let active = user::ActiveModel {
            id: Set(id),
            username: Set(username.to_owned()),
            password_hash: Set(password_hash.to_owned()),
            is_admin: Set(is_admin),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = user::Entity::insert(active)
            .exec_with_returning(&self.conn)
            .await
            .with_context(|| format!("failed to insert user '{username}'"))?;

        Ok(User {
            id: model.id,
            username: model.username,
            is_admin: model.is_admin,
            created_at: model.created_at,
            updated_at: model.updated_at,
        })
    }

    /// Fetches a user by username, returning `(User, password_hash)`.
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<(User, String)>> {
        let model = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.conn)
            .await
            .with_context(|| format!("failed to find user by username '{username}'"))?;

        Ok(model.map(|m| {
            let user = User {
                id: m.id,
                username: m.username,
                is_admin: m.is_admin,
                created_at: m.created_at,
                updated_at: m.updated_at,
            };
            (user, m.password_hash)
        }))
    }

    /// Lists all user accounts.
    pub async fn list_users(&self) -> Result<Vec<User>> {
        let models = user::Entity::find()
            .order_by_asc(user::Column::Username)
            .all(&self.conn)
            .await
            .context("failed to list users")?;

        Ok(models
            .into_iter()
            .map(|m| User {
                id: m.id,
                username: m.username,
                is_admin: m.is_admin,
                created_at: m.created_at,
                updated_at: m.updated_at,
            })
            .collect())
    }

    /// Grants a user access to a library.
    pub async fn grant_library_access(&self, user_id: Uuid, library_id: Uuid) -> Result<()> {
        let active = user_library::ActiveModel {
            user_id: Set(user_id),
            library_id: Set(library_id),
        };

        user_library::Entity::insert(active)
            .on_conflict_do_nothing()
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to grant user {user_id} access to library {library_id}"))?;

        Ok(())
    }

    /// Revokes a user's access to a library.
    pub async fn revoke_library_access(&self, user_id: Uuid, library_id: Uuid) -> Result<bool> {
        let res = user_library::Entity::delete_many()
            .filter(user_library::Column::UserId.eq(user_id))
            .filter(user_library::Column::LibraryId.eq(library_id))
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to revoke user {user_id} access to library {library_id}"))?;

        Ok(res.rows_affected > 0)
    }

    /// Lists library IDs accessible by a user.
    pub async fn list_user_libraries(&self, user_id: Uuid) -> Result<Vec<Uuid>> {
        let rows = user_library::Entity::find()
            .filter(user_library::Column::UserId.eq(user_id))
            .all(&self.conn)
            .await
            .with_context(|| format!("failed to list libraries for user {user_id}"))?;

        Ok(rows.into_iter().map(|r| r.library_id).collect())
    }

    /// Checks if a user can access a specific library (admins have universal access).
    pub async fn user_can_access_library(&self, user_id: Uuid, library_id: Uuid) -> Result<bool> {
        let Some(u) = user::Entity::find_by_id(user_id).one(&self.conn).await? else {
            return Ok(false);
        };

        if u.is_admin {
            return Ok(true);
        }

        let access = user_library::Entity::find()
            .filter(user_library::Column::UserId.eq(user_id))
            .filter(user_library::Column::LibraryId.eq(library_id))
            .one(&self.conn)
            .await?;

        Ok(access.is_some())
    }
}
