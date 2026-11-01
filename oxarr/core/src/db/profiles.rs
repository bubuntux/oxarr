use anyhow::{Context, Result};
use oxarr_types::QualityProfile;
use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, Set};
use uuid::Uuid;

use super::models::json_types::ProfileClassifierScoreList;
use super::models::quality_profile;
use super::Database;

fn into_domain(model: quality_profile::Model) -> QualityProfile {
    QualityProfile {
        id: model.id,
        name: model.name,
        upgrade_allowed: model.upgrade_allowed,
        min_score: model.min_score,
        cutoff_score: model.cutoff_score,
        classifier_scores: model.classifier_scores_json.0,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

impl Database {
    pub async fn insert_quality_profile(&self, profile: &QualityProfile) -> Result<()> {
        let active = quality_profile::ActiveModel {
            id: Set(profile.id),
            name: Set(profile.name.clone()),
            upgrade_allowed: Set(profile.upgrade_allowed),
            min_score: Set(profile.min_score),
            cutoff_score: Set(profile.cutoff_score),
            classifier_scores_json: Set(ProfileClassifierScoreList(
                profile.classifier_scores.clone(),
            )),
            created_at: Set(profile.created_at),
            updated_at: Set(profile.updated_at),
        };

        quality_profile::Entity::insert(active)
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to insert quality profile '{}'", profile.name))?;

        Ok(())
    }

    pub async fn get_quality_profile(&self, id: Uuid) -> Result<Option<QualityProfile>> {
        let model = quality_profile::Entity::find_by_id(id)
            .one(&self.conn)
            .await
            .with_context(|| format!("failed to fetch quality profile {id}"))?;

        Ok(model.map(into_domain))
    }

    pub async fn list_quality_profiles(&self) -> Result<Vec<QualityProfile>> {
        let models = quality_profile::Entity::find()
            .order_by_asc(quality_profile::Column::Name)
            .all(&self.conn)
            .await
            .context("failed to list quality profiles")?;

        Ok(models.into_iter().map(into_domain).collect())
    }

    pub async fn update_quality_profile(&self, profile: &QualityProfile) -> Result<bool> {
        let Some(existing) = quality_profile::Entity::find_by_id(profile.id)
            .one(&self.conn)
            .await?
        else {
            return Ok(false);
        };

        let mut active: quality_profile::ActiveModel = existing.into();
        active.name = Set(profile.name.clone());
        active.upgrade_allowed = Set(profile.upgrade_allowed);
        active.min_score = Set(profile.min_score);
        active.cutoff_score = Set(profile.cutoff_score);
        active.classifier_scores_json = Set(ProfileClassifierScoreList(
            profile.classifier_scores.clone(),
        ));
        active.updated_at = Set(profile.updated_at);

        active
            .update(&self.conn)
            .await
            .with_context(|| format!("failed to update quality profile {}", profile.id))?;

        Ok(true)
    }

    pub async fn delete_quality_profile(&self, id: Uuid) -> Result<bool> {
        let res = quality_profile::Entity::delete_by_id(id)
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to delete quality profile {id}"))?;

        Ok(res.rows_affected > 0)
    }
}
