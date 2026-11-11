use sea_orm::entity::prelude::*;

use super::json_types::UuidList;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "media_assets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub path: String,
    pub size_bytes: i64,
    pub hash_xxh3: Option<String>,
    pub quality_score: Option<i32>,
    pub matched_classifier_ids_json: UuidList,
    pub media_info_json: Json,
    pub imported_at: DateTimeUtc,
    #[sea_orm(has_many, via = "entity_asset")]
    pub entities: HasMany<super::entity::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
