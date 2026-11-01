use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "collection_movies")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub collection_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub movie_id: Uuid,
    #[sea_orm(
        belongs_to,
        from = "collection_id",
        to = "entity_id",
        on_delete = "Cascade"
    )]
    pub collection: BelongsTo<super::collection::Entity>,
    #[sea_orm(belongs_to, from = "movie_id", to = "entity_id", on_delete = "Cascade")]
    pub movie: BelongsTo<super::movie::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
