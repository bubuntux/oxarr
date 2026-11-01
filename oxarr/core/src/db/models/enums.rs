use oxarr_types::{EntityKind, MinimumAvailability, TrackedReleaseStatus};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "snake_case"
)]
pub enum DbMinimumAvailability {
    Announced,
    InCinemas,
    Released,
}

impl From<MinimumAvailability> for DbMinimumAvailability {
    fn from(availability: MinimumAvailability) -> Self {
        match availability {
            MinimumAvailability::Announced => Self::Announced,
            MinimumAvailability::InCinemas => Self::InCinemas,
            MinimumAvailability::Released => Self::Released,
        }
    }
}

impl From<DbMinimumAvailability> for MinimumAvailability {
    fn from(db: DbMinimumAvailability) -> Self {
        match db {
            DbMinimumAvailability::Announced => Self::Announced,
            DbMinimumAvailability::InCinemas => Self::InCinemas,
            DbMinimumAvailability::Released => Self::Released,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, EnumIter, DeriveActiveEnum)]
#[sea_orm(
    rs_type = "String",
    db_type = "String(StringLen::None)",
    rename_all = "snake_case"
)]
pub enum DbTrackedReleaseStatus {
    Grabbed,
    Imported,
    Failed,
    Blocklisted,
}

impl From<TrackedReleaseStatus> for DbTrackedReleaseStatus {
    fn from(status: TrackedReleaseStatus) -> Self {
        match status {
            TrackedReleaseStatus::Grabbed => Self::Grabbed,
            TrackedReleaseStatus::Imported => Self::Imported,
            TrackedReleaseStatus::Failed => Self::Failed,
            TrackedReleaseStatus::Blocklisted => Self::Blocklisted,
        }
    }
}

impl From<DbTrackedReleaseStatus> for TrackedReleaseStatus {
    fn from(db: DbTrackedReleaseStatus) -> Self {
        match db {
            DbTrackedReleaseStatus::Grabbed => Self::Grabbed,
            DbTrackedReleaseStatus::Imported => Self::Imported,
            DbTrackedReleaseStatus::Failed => Self::Failed,
            DbTrackedReleaseStatus::Blocklisted => Self::Blocklisted,
        }
    }
}
