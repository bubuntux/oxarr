use std::collections::HashSet;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use oxarr_types::{Library, User};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, Set, TransactionTrait,
};
use uuid::Uuid;

use super::libraries;
use super::models::{library, user, user_library};
use super::Database;

fn user_from_model(model: user::Model) -> User {
    User {
        id: model.id,
        username: model.username,
        is_admin: model.is_admin,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

impl Database {
    /// Returns the total number of registered user accounts.
    pub async fn count_users(&self) -> Result<u64> {
        user::Entity::find()
            .count(&self.conn)
            .await
            .context("failed to count users")
    }

    /// Returns the number of administrator accounts.
    pub async fn count_admins(&self) -> Result<u64> {
        user::Entity::find()
            .filter(user::Column::IsAdmin.eq(true))
            .count(&self.conn)
            .await
            .context("failed to count administrator users")
    }

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

        Ok(user_from_model(model))
    }

    /// Atomically creates the first administrator account if no users exist yet.
    /// Returns `Ok(None)` when at least one user is already registered.
    pub async fn create_initial_admin(
        &self,
        id: Uuid,
        username: &str,
        password_hash: &str,
        now: DateTime<Utc>,
    ) -> Result<Option<User>> {
        let tx = self.conn.begin().await?;
        let existing_count = user::Entity::find()
            .count(&tx)
            .await
            .context("failed to check existing users")?;

        if existing_count > 0 {
            return Ok(None);
        }

        let active = user::ActiveModel {
            id: Set(id),
            username: Set(username.to_owned()),
            password_hash: Set(password_hash.to_owned()),
            is_admin: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
        };

        let model = user::Entity::insert(active)
            .exec_with_returning(&tx)
            .await
            .with_context(|| format!("failed to insert initial admin '{username}'"))?;

        tx.commit().await?;
        Ok(Some(user_from_model(model)))
    }

    /// Fetches a user by ID.
    pub async fn get_user(&self, id: Uuid) -> Result<Option<User>> {
        let model = user::Entity::find_by_id(id)
            .one(&self.conn)
            .await
            .with_context(|| format!("failed to fetch user {id}"))?;

        Ok(model.map(user_from_model))
    }

    /// Fetches a user by ID alongside their stored password hash.
    pub async fn get_user_with_password_hash(&self, id: Uuid) -> Result<Option<(User, String)>> {
        let model = user::Entity::find_by_id(id)
            .one(&self.conn)
            .await
            .with_context(|| format!("failed to fetch user {id}"))?;

        Ok(model.map(|m| {
            let password_hash = m.password_hash.clone();
            (user_from_model(m), password_hash)
        }))
    }

    /// Fetches a user by username, returning `(User, password_hash)`.
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<(User, String)>> {
        let model = user::Entity::find()
            .filter(user::Column::Username.eq(username))
            .one(&self.conn)
            .await
            .with_context(|| format!("failed to find user by username '{username}'"))?;

        Ok(model.map(|m| {
            let password_hash = m.password_hash.clone();
            (user_from_model(m), password_hash)
        }))
    }

    /// Lists all user accounts ordered by username.
    pub async fn list_users(&self) -> Result<Vec<User>> {
        let models = user::Entity::find()
            .order_by_asc(user::Column::Username)
            .all(&self.conn)
            .await
            .context("failed to list users")?;

        Ok(models.into_iter().map(user_from_model).collect())
    }

    /// Replaces the stored password hash for a user. Returns `Ok(false)` if not found.
    pub async fn update_user_password(
        &self,
        id: Uuid,
        password_hash: &str,
        updated_at: DateTime<Utc>,
    ) -> Result<bool> {
        let Some(existing) = user::Entity::find_by_id(id)
            .one(&self.conn)
            .await
            .with_context(|| format!("failed to look up user {id} for password update"))?
        else {
            return Ok(false);
        };

        let mut active: user::ActiveModel = existing.into();
        active.password_hash = Set(password_hash.to_owned());
        active.updated_at = Set(updated_at);
        active
            .update(&self.conn)
            .await
            .with_context(|| format!("failed to update password for user {id}"))?;

        Ok(true)
    }

    /// Deletes a user account (cascading library grants and sessions).
    pub async fn delete_user(&self, id: Uuid) -> Result<bool> {
        let res = user::Entity::delete_by_id(id)
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to delete user {id}"))?;

        Ok(res.rows_affected > 0)
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
            .with_context(|| {
                format!("failed to grant user {user_id} access to library {library_id}")
            })?;

        Ok(())
    }

    /// Revokes a user's access to a library.
    pub async fn revoke_library_access(&self, user_id: Uuid, library_id: Uuid) -> Result<bool> {
        let res = user_library::Entity::delete_many()
            .filter(user_library::Column::UserId.eq(user_id))
            .filter(user_library::Column::LibraryId.eq(library_id))
            .exec(&self.conn)
            .await
            .with_context(|| {
                format!("failed to revoke user {user_id} access to library {library_id}")
            })?;

        Ok(res.rows_affected > 0)
    }

    /// Atomically replaces the set of libraries accessible by a user.
    pub async fn set_user_libraries(&self, user_id: Uuid, library_ids: &[Uuid]) -> Result<()> {
        let tx = self.conn.begin().await?;

        user_library::Entity::delete_many()
            .filter(user_library::Column::UserId.eq(user_id))
            .exec(&tx)
            .await
            .with_context(|| format!("failed to clear libraries for user {user_id}"))?;

        let mut seen = HashSet::new();
        let links: Vec<user_library::ActiveModel> = library_ids
            .iter()
            .copied()
            .filter(|lib_id| seen.insert(*lib_id))
            .map(|lib_id| user_library::ActiveModel {
                user_id: Set(user_id),
                library_id: Set(lib_id),
            })
            .collect();

        if !links.is_empty() {
            user_library::Entity::insert_many(links)
                .exec(&tx)
                .await
                .with_context(|| format!("failed to assign libraries to user {user_id}"))?;
        }

        tx.commit().await?;
        Ok(())
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

    /// Lists all libraries accessible by the given user (all libraries for admins).
    pub async fn list_accessible_libraries(&self, user: &User) -> Result<Vec<Library>> {
        if user.is_admin {
            return self.list_libraries().await;
        }

        let models = library::Entity::find()
            .filter(
                library::Column::Id.in_subquery(
                    user_library::Entity::find()
                        .select_only()
                        .column(user_library::Column::LibraryId)
                        .filter(user_library::Column::UserId.eq(user.id))
                        .into_query(),
                ),
            )
            .order_by_asc(library::Column::Name)
            .all(&self.conn)
            .await
            .with_context(|| format!("failed to list accessible libraries for user {}", user.id))?;

        Ok(models.into_iter().map(libraries::into_domain).collect())
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
