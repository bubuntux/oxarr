use oxarr_types::{ClassifierCondition, EntityMetadata, ProfileClassifierScore};
use sea_orm::FromJsonQueryResult;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(transparent)]
pub struct DbEntityMetadata(pub EntityMetadata);

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(transparent)]
pub struct ClassifierConditionList(pub Vec<ClassifierCondition>);

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(transparent)]
pub struct ProfileClassifierScoreList(pub Vec<ProfileClassifierScore>);

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, FromJsonQueryResult)]
#[serde(transparent)]
pub struct UuidList(pub Vec<Uuid>);
