use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "entity_assets")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub entity_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub asset_id: Uuid,
    pub position: i32,
    #[sea_orm(belongs_to, from = "entity_id", to = "id", on_delete = "Cascade")]
    pub entity: BelongsTo<super::entity::Entity>,
    #[sea_orm(belongs_to, from = "asset_id", to = "id", on_delete = "Cascade")]
    pub media_asset: BelongsTo<super::media_asset::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
