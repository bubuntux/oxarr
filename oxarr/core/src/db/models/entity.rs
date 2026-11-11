use sea_orm::entity::prelude::*;

use super::enums::DbEntityKind;
use super::json_types::DbEntityMetadata;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "entities")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub library_id: Option<Uuid>,
    pub kind: DbEntityKind,
    pub title: String,
    pub original_title: Option<String>,
    pub original_language: Option<String>,
    pub monitored: bool,
    pub metadata_json: DbEntityMetadata,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    #[sea_orm(belongs_to, from = "library_id", to = "id", on_delete = "Restrict")]
    pub library: BelongsTo<Option<super::library::Entity>>,
    #[sea_orm(has_one)]
    pub movie: HasOne<super::movie::Entity>,
    #[sea_orm(has_one)]
    pub collection: HasOne<super::collection::Entity>,
    #[sea_orm(has_many)]
    pub external_ids: HasMany<super::entity_external_id::Entity>,
    #[sea_orm(has_many, via = "entity_asset")]
    pub media_assets: HasMany<super::media_asset::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
