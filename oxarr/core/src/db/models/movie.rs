use sea_orm::entity::prelude::*;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "movies")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub entity_id: Uuid,
    pub year: Option<i32>,
    pub in_cinemas_date: Option<Date>,
    pub digital_release_date: Option<Date>,
    pub physical_release_date: Option<Date>,
    pub runtime_minutes: Option<i32>,
    pub certification: Option<String>,
    pub studio: Option<String>,
    #[sea_orm(belongs_to, from = "entity_id", to = "id", on_delete = "Cascade")]
    pub entity: BelongsTo<super::entity::Entity>,
    #[sea_orm(has_many, via = "collection_movie")]
    pub collections: HasMany<super::collection::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
