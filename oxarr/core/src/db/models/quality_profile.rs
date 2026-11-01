use sea_orm::entity::prelude::*;

use super::json_types::ProfileClassifierScoreList;

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "quality_profiles")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub name: String,
    pub upgrade_allowed: bool,
    pub min_score: i32,
    pub cutoff_score: i32,
    pub classifier_scores_json: ProfileClassifierScoreList,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
    #[sea_orm(has_many)]
    pub entities: HasMany<super::entity::Entity>,
}

impl ActiveModelBehavior for ActiveModel {}
