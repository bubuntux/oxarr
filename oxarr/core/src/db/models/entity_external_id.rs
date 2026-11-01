use sea_orm::entity::prelude::*;

use super::enums::DbEntityKind;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "entity_external_ids")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub entity_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub provider: String,
    pub entity_kind: DbEntityKind,
    pub external_id: String,
    /// Composite key keeps `entity_kind` from drifting away from `entities.kind`.
    #[sea_orm(
        belongs_to,
        from = "(entity_id, entity_kind)",
        to = "(id, kind)",
        on_delete = "Cascade"
    )]
    pub entity: BelongsTo<super::entity::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
