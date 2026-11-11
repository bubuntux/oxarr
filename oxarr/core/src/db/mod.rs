mod assets;
mod classifiers;
mod collections;
mod entities;
mod libraries;
pub mod models;
mod users;

use std::str::FromStr;
use std::time::Duration;

use anyhow::{Context, Result};
use oxarr_migration::{Migrator, MigratorTrait};
use sea_orm::sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous,
};
use sea_orm::{DatabaseConnection, SqlxSqliteConnector};
use uuid::Uuid;

#[derive(Clone)]
pub struct Database {
    conn: DatabaseConnection,
}

impl Database {
    /// Opens a file-backed SQLite pool in WAL mode and runs embedded SeaORM migrations.
    pub async fn connect(database_url: &str) -> Result<Self> {
        let connect_options = base_connect_options(database_url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal);

        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .connect_with(connect_options)
            .await
            .with_context(|| format!("failed to connect to SQLite database '{database_url}'"))?;

        let conn = SqlxSqliteConnector::from_sqlx_sqlite_pool(pool);
        let db = Self { conn };
        db.run_migrations().await?;
        Ok(db)
    }

    /// Opens a single-connection in-memory SQLite database and runs embedded SeaORM migrations.
    pub async fn connect_in_memory() -> Result<Self> {
        let connect_options = base_connect_options("sqlite::memory:")?;

        // Keep a single non-expiring connection so the `:memory:` database persists for the pool's lifetime.
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .idle_timeout(None)
            .max_lifetime(None)
            .connect_with(connect_options)
            .await
            .context("failed to open in-memory SQLite database")?;

        let conn = SqlxSqliteConnector::from_sqlx_sqlite_pool(pool);
        let db = Self { conn };
        db.run_migrations().await?;
        Ok(db)
    }

    pub fn conn(&self) -> &DatabaseConnection {
        &self.conn
    }

    pub async fn run_migrations(&self) -> Result<()> {
        Migrator::up(&self.conn, None)
            .await
            .context("failed to run embedded SeaORM migrations")?;
        Ok(())
    }
}

fn base_connect_options(database_url: &str) -> Result<SqliteConnectOptions> {
    SqliteConnectOptions::from_str(database_url)
        .with_context(|| format!("invalid SQLite connection URL '{database_url}'"))
        .map(|opts| {
            opts.foreign_keys(true)
                .busy_timeout(Duration::from_secs(5))
                .pragma("recursive_triggers", "ON")
                .pragma("temp_store", "MEMORY")
                .pragma("cache_size", "-16384")
        })
}

/// Generates a new time-ordered `UUIDv7` identifier for catalog records.
pub fn new_id() -> Uuid {
    Uuid::now_v7()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::{NaiveDate, SubsecRound, Utc};
    use oxarr_types::{
        Classifier, ClassifierCondition, CollectionDetails, Entity, EntityDetails, EntityKind,
        EntityMetadata, Library, MediaAsset, MovieDetails,
    };
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    use serde_json::json;

    use super::*;

    fn sample_timestamp() -> chrono::DateTime<Utc> {
        Utc::now().trunc_subsecs(3)
    }

    fn base_entity(
        id: Uuid,
        kind: EntityKind,
        title: &str,
        details: EntityDetails,
        now: chrono::DateTime<Utc>,
    ) -> Entity {
        Entity {
            id,
            kind,
            library_id: None,
            title: title.to_owned(),
            original_title: None,
            original_language: None,
            monitored: true,
            external_ids: BTreeMap::new(),
            metadata: EntityMetadata::default(),
            details,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn file_backed_connect_enables_wal_foreign_keys_and_recursive_triggers() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let db_path = temp_dir.path().join("oxarr.db");
        let db_url = format!("sqlite://{}", db_path.display());

        let db = Database::connect(&db_url).await.expect("connect file db");

        let journal_row = db
            .conn()
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                "PRAGMA journal_mode;",
            ))
            .await
            .unwrap()
            .unwrap();
        let journal_mode: String = journal_row.try_get_by_index(0).unwrap();
        assert_eq!(journal_mode.to_lowercase(), "wal");

        let fk_row = db
            .conn()
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                "PRAGMA foreign_keys;",
            ))
            .await
            .unwrap()
            .unwrap();
        let foreign_keys: i64 = fk_row.try_get_by_index(0).unwrap();
        assert_eq!(foreign_keys, 1);

        let rt_row = db
            .conn()
            .query_one_raw(Statement::from_string(
                DatabaseBackend::Sqlite,
                "PRAGMA recursive_triggers;",
            ))
            .await
            .unwrap()
            .unwrap();
        let recursive_triggers: i64 = rt_row.try_get_by_index(0).unwrap();
        assert_eq!(recursive_triggers, 1);
    }

    #[tokio::test]
    async fn movies_collections_and_asset_cascades() {
        let db = Database::connect_in_memory()
            .await
            .expect("connect memory db");
        let now = sample_timestamp();

        let classifier_2160p = Classifier {
            id: new_id(),
            name: "2160p".to_owned(),
            conditions: vec![ClassifierCondition::Regex {
                pattern: r"\b(2160p|4k|uhd)\b".to_owned(),
                negate: false,
            }],
            match_all: true,
            created_at: now,
            updated_at: now,
        };
        db.insert_classifier(&classifier_2160p).await.unwrap();

        let movie_lib = Library {
            id: new_id(),
            name: "Movies".to_owned(),
            kind: EntityKind::Movie,
            root_path: "/media/movies".to_owned(),
            created_at: now,
            updated_at: now,
        };
        db.insert_library(&movie_lib).await.unwrap();

        let collection_id = new_id();
        let mut matrix_collection = base_entity(
            collection_id,
            EntityKind::Collection,
            "The Matrix Collection",
            EntityDetails::Collection(CollectionDetails {}),
            now,
        );
        matrix_collection.library_id = Some(movie_lib.id);
        matrix_collection.external_ids = BTreeMap::from([("tmdb".to_owned(), "2344".to_owned())]);

        let matrix_1_id = new_id();
        let mut matrix_1 = base_entity(
            matrix_1_id,
            EntityKind::Movie,
            "The Matrix",
            EntityDetails::Movie(MovieDetails {
                year: Some(1999),
                in_cinemas_date: NaiveDate::from_ymd_opt(1999, 3, 31),
                digital_release_date: None,
                physical_release_date: NaiveDate::from_ymd_opt(1999, 9, 21),
                runtime_minutes: Some(136),
                certification: Some("R".to_owned()),
                studio: Some("Warner Bros.".to_owned()),
            }),
            now,
        );
        matrix_1.library_id = Some(movie_lib.id);
        matrix_1.original_language = Some("en".to_owned());
        matrix_1.metadata.genres = vec!["Sci-Fi".to_owned(), "Action".to_owned()];
        matrix_1.external_ids = BTreeMap::from([
            ("tmdb".to_owned(), "603".to_owned()),
            ("imdb".to_owned(), "tt0133093".to_owned()),
        ]);

        let matrix_2_id = new_id();
        let mut matrix_2 = base_entity(
            matrix_2_id,
            EntityKind::Movie,
            "The Matrix Reloaded",
            EntityDetails::Movie(MovieDetails {
                year: Some(2003),
                in_cinemas_date: NaiveDate::from_ymd_opt(2003, 5, 15),
                ..MovieDetails::default()
            }),
            now,
        );
        matrix_2.library_id = Some(movie_lib.id);

        let standalone_id = new_id();
        let mut standalone = base_entity(
            standalone_id,
            EntityKind::Movie,
            "Inception",
            EntityDetails::Movie(MovieDetails {
                year: Some(2010),
                ..MovieDetails::default()
            }),
            now,
        );
        standalone.library_id = Some(movie_lib.id);

        db.insert_entities_batch(&[
            matrix_collection.clone(),
            matrix_2.clone(),
            matrix_1.clone(),
            standalone,
        ])
        .await
        .expect("insert collection and movies");

        assert_eq!(
            db.get_entity_by_external_id(EntityKind::Movie, "imdb", "tt0133093")
                .await
                .unwrap()
                .map(|e| e.id),
            Some(matrix_1_id)
        );

        // Typed FK check on collection_movies: adding a Collection as a movie_id must fail
        assert!(db
            .add_movie_to_collection(collection_id, collection_id)
            .await
            .is_err());

        db.add_movie_to_collection(collection_id, matrix_2_id)
            .await
            .unwrap();
        db.add_movie_to_collection(collection_id, matrix_1_id)
            .await
            .unwrap();
        db.add_movie_to_collection(collection_id, matrix_1_id)
            .await
            .unwrap();

        let col_movies = db.list_collection_movies(collection_id).await.unwrap();
        assert_eq!(
            col_movies.iter().map(|m| m.id).collect::<Vec<_>>(),
            vec![matrix_1_id, matrix_2_id],
            "collection movies must sort chronologically"
        );
        assert_eq!(
            db.list_movie_collections(matrix_1_id).await.unwrap(),
            vec![matrix_collection]
        );

        // Deleting a Collection preserves its Movies
        assert!(db.delete_entity(collection_id).await.unwrap());
        assert!(db.get_entity(matrix_1_id).await.unwrap().is_some());

        // M:N MediaAsset: shared across two entities survives deleting the first, and auto-cleans on the last
        let asset = MediaAsset {
            id: new_id(),
            entity_ids: vec![matrix_1_id, matrix_2_id],
            path: "/media/movies/The Matrix Double Feature.mkv".to_owned(),
            size_bytes: 42_000_000_000,
            hash_xxh3: Some("0123456789abcdef".to_owned()),
            matched_classifier_ids: vec![classifier_2160p.id],
            quality_score: Some(5500),
            media_info: json!({ "duration_secs": 16440, "container": "mkv", "video_codec": "hevc" }),
            imported_at: now,
        };
        db.insert_media_asset(&asset).await.unwrap();
        assert_eq!(
            db.get_media_asset(asset.id).await.unwrap(),
            Some(asset.clone())
        );

        // Deleting matrix_1 keeps shared asset alive for matrix_2
        assert!(db.delete_entity(matrix_1_id).await.unwrap());
        let surviving_asset = db.get_media_asset(asset.id).await.unwrap().unwrap();
        assert_eq!(surviving_asset.entity_ids, vec![matrix_2_id]);

        // Deleting matrix_2 removes the last link and triggers orphan cleanup of the asset
        assert!(db.delete_entity(matrix_2_id).await.unwrap());
        assert!(db.get_media_asset(asset.id).await.unwrap().is_none());

        // Migration rollback (`down`) also executes cleanly
        Migrator::down(db.conn(), None)
            .await
            .expect("migrator down");
    }

    #[tokio::test]
    async fn enforces_scoped_external_ids() {
        let db = Database::connect_in_memory()
            .await
            .expect("connect memory db");
        let now = sample_timestamp();

        let mut movie_1 = base_entity(
            new_id(),
            EntityKind::Movie,
            "The Matrix",
            EntityDetails::Movie(MovieDetails {
                year: Some(1999),
                ..MovieDetails::default()
            }),
            now,
        );
        movie_1.external_ids = BTreeMap::from([("tmdb".to_owned(), "603".to_owned())]);
        db.insert_entity(&movie_1)
            .await
            .expect("insert first movie");

        // Same TMDB integer ID ("603") on Collection must succeed (separate namespace per entity_kind)
        let mut collection_603 = base_entity(
            new_id(),
            EntityKind::Collection,
            "Collection 603",
            EntityDetails::Collection(CollectionDetails {}),
            now,
        );
        collection_603.external_ids = BTreeMap::from([("tmdb".to_owned(), "603".to_owned())]);
        db.insert_entity(&collection_603)
            .await
            .expect("collection tmdb:603 coexists with movie tmdb:603");

        assert_eq!(
            db.find_entities_by_external_id("tmdb", "603")
                .await
                .unwrap()
                .len(),
            2
        );

        // Duplicate (entity_kind = movie, provider = tmdb, external_id = 603) must fail and roll back
        let mut duplicate_movie_ext = movie_1;
        duplicate_movie_ext.id = new_id();
        duplicate_movie_ext.title = "The Matrix Duplicate".to_owned();
        assert!(db.insert_entity(&duplicate_movie_ext).await.is_err());
        assert!(db
            .get_entity(duplicate_movie_ext.id)
            .await
            .expect("rolled back entity lookup")
            .is_none());
    }

    #[tokio::test]
    async fn user_management_and_library_access_control() {
        let db = Database::connect_in_memory()
            .await
            .expect("connect memory db");
        let now = sample_timestamp();

        let admin = db
            .insert_user(new_id(), "admin", "argon2_hash_admin", true, now)
            .await
            .unwrap();
        let user = db
            .insert_user(new_id(), "alice", "argon2_hash_alice", false, now)
            .await
            .unwrap();

        let lib_movies = Library {
            id: new_id(),
            name: "Movies".to_owned(),
            kind: EntityKind::Movie,
            root_path: "/media/movies".to_owned(),
            created_at: now,
            updated_at: now,
        };
        let lib_kids = Library {
            id: new_id(),
            name: "Kids".to_owned(),
            kind: EntityKind::Movie,
            root_path: "/media/kids".to_owned(),
            created_at: now,
            updated_at: now,
        };
        db.insert_library(&lib_movies).await.unwrap();
        db.insert_library(&lib_kids).await.unwrap();

        // Admin has universal access
        assert!(db.user_can_access_library(admin.id, lib_movies.id).await.unwrap());
        assert!(db.user_can_access_library(admin.id, lib_kids.id).await.unwrap());

        // Regular user has no access by default
        assert!(!db.user_can_access_library(user.id, lib_movies.id).await.unwrap());
        assert!(!db.user_can_access_library(user.id, lib_kids.id).await.unwrap());

        // Grant access to Kids only
        db.grant_library_access(user.id, lib_kids.id).await.unwrap();
        assert!(!db.user_can_access_library(user.id, lib_movies.id).await.unwrap());
        assert!(db.user_can_access_library(user.id, lib_kids.id).await.unwrap());

        assert_eq!(db.list_user_libraries(user.id).await.unwrap(), vec![lib_kids.id]);

        // Revoke access
        assert!(db.revoke_library_access(user.id, lib_kids.id).await.unwrap());
        assert!(!db.user_can_access_library(user.id, lib_kids.id).await.unwrap());
    }
}
