use anyhow::{Context, Result};
use oxarr_types::Classifier;
use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, Set};
use uuid::Uuid;

use super::models::classifier;
use super::models::json_types::ClassifierConditionList;
use super::Database;

fn into_domain(model: classifier::Model) -> Classifier {
    Classifier {
        id: model.id,
        name: model.name,
        conditions: model.conditions_json.0,
        match_all: model.match_all,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

impl Database {
    pub async fn insert_classifier(&self, classifier: &Classifier) -> Result<()> {
        let active = classifier::ActiveModel {
            id: Set(classifier.id),
            name: Set(classifier.name.clone()),
            match_all: Set(classifier.match_all),
            conditions_json: Set(ClassifierConditionList(classifier.conditions.clone())),
            created_at: Set(classifier.created_at),
            updated_at: Set(classifier.updated_at),
        };

        classifier::Entity::insert(active)
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to insert classifier '{}'", classifier.name))?;

        Ok(())
    }

    pub async fn get_classifier(&self, id: Uuid) -> Result<Option<Classifier>> {
        let model = classifier::Entity::find_by_id(id)
            .one(&self.conn)
            .await
            .with_context(|| format!("failed to fetch classifier {id}"))?;

        Ok(model.map(into_domain))
    }

    pub async fn list_classifiers(&self) -> Result<Vec<Classifier>> {
        let models = classifier::Entity::find()
            .order_by_asc(classifier::Column::Name)
            .all(&self.conn)
            .await
            .context("failed to list classifiers")?;

        Ok(models.into_iter().map(into_domain).collect())
    }

    pub async fn update_classifier(&self, classifier: &Classifier) -> Result<bool> {
        let Some(existing) = classifier::Entity::find_by_id(classifier.id)
            .one(&self.conn)
            .await?
        else {
            return Ok(false);
        };

        let mut active: classifier::ActiveModel = existing.into();
        active.name = Set(classifier.name.clone());
        active.match_all = Set(classifier.match_all);
        active.conditions_json = Set(ClassifierConditionList(classifier.conditions.clone()));
        active.updated_at = Set(classifier.updated_at);

        active
            .update(&self.conn)
            .await
            .with_context(|| format!("failed to update classifier {}", classifier.id))?;

        Ok(true)
    }

    pub async fn delete_classifier(&self, id: Uuid) -> Result<bool> {
        let res = classifier::Entity::delete_by_id(id)
            .exec(&self.conn)
            .await
            .with_context(|| format!("failed to delete classifier {id}"))?;

        Ok(res.rows_affected > 0)
    }
}
