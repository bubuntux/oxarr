use oxarr_types::EntityKind;
use sea_orm::entity::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "snake_case"
)]
pub enum DbEntityKind {
    Movie,
    Collection,
}

impl From<EntityKind> for DbEntityKind {
    fn from(kind: EntityKind) -> Self {
        match kind {
            EntityKind::Movie => Self::Movie,
            EntityKind::Collection => Self::Collection,
        }
    }
}

impl From<DbEntityKind> for EntityKind {
    fn from(db: DbEntityKind) -> Self {
        match db {
            DbEntityKind::Movie => Self::Movie,
            DbEntityKind::Collection => Self::Collection,
        }
    }
}
