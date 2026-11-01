use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "collections")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub entity_id: Uuid,
    #[sea_orm(belongs_to, from = "entity_id", to = "id", on_delete = "Cascade")]
    pub entity: BelongsTo<super::entity::Entity>,
    #[sea_orm(has_many, via = "collection_movie")]
    pub movies: HasMany<super::movie::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
