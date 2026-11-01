use sea_orm::entity::prelude::*;

use super::enums::DbEntityKind;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "libraries")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub name: String,
    pub kind: DbEntityKind,
    #[sea_orm(unique)]
    pub root_path: String,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    #[sea_orm(has_many)]
    pub entities: HasMany<super::entity::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
