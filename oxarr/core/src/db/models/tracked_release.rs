use sea_orm::entity::prelude::*;

use super::enums::DbTrackedReleaseStatus;
use super::json_types::UuidList;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "tracked_releases")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub entity_id: Uuid,
    pub indexer: String,
    pub guid: String,
    pub title: String,
    pub download_url: String,
    pub info_hash: Option<String>,
    pub size_bytes: i64,
    pub score: i32,
    pub matched_classifier_ids_json: UuidList,
    pub status: DbTrackedReleaseStatus,
    pub download_client_id: Option<String>,
    pub published_at: Option<DateTimeUtc>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    #[sea_orm(belongs_to, from = "entity_id", to = "id", on_delete = "Cascade")]
    pub entity: BelongsTo<super::entity::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
