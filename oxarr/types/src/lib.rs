pub mod asset;
pub mod entity;
pub mod media;
pub mod provider;
pub mod quality;
pub mod release;

pub use asset::MediaAsset;
pub use entity::{Entity, EntityDetails, EntityKind, EntityMetadata, Library};
pub use media::{CollectionDetails, MinimumAvailability, MovieDetails};
pub use provider::{
    CollectionMetadata, CollectionSummary, MovieMetadata, ProviderMediaMetadata, SearchQuery,
    SearchResult,
};
pub use quality::{Classifier, ClassifierCondition, ProfileClassifierScore, QualityProfile};
pub use release::{ReleaseCandidate, TrackedRelease, TrackedReleaseStatus};

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn entity_kind_json_round_trip() {
        for kind in [EntityKind::Movie, EntityKind::Collection] {
            let encoded = serde_json::to_string(&kind).expect("serialize entity kind");
            let decoded: EntityKind =
                serde_json::from_str(&encoded).expect("deserialize entity kind");
            assert_eq!(decoded, kind);
        }
    }

    #[test]
    fn entity_details_json_round_trip() {
        let movie = EntityDetails::Movie(MovieDetails {
            year: Some(1999),
            in_cinemas_date: None,
            digital_release_date: None,
            physical_release_date: None,
            minimum_availability: MinimumAvailability::Released,
            runtime_minutes: Some(136),
            certification: Some("R".to_owned()),
            studio: Some("Warner Bros.".to_owned()),
        });

        let collection = EntityDetails::Collection(CollectionDetails::default());

        for details in [movie, collection] {
            let json = serde_json::to_string(&details).expect("serialize EntityDetails");
            let decoded: EntityDetails =
                serde_json::from_str(&json).expect("deserialize EntityDetails");
            assert_eq!(decoded, details);
        }
    }

    #[test]
    fn classifier_and_profile_json_round_trip() {
        let now = Utc::now();
        let classifier_2160p_id = Uuid::from_u128(1);
        let classifier_cam_id = Uuid::from_u128(2);

        let classifier_2160p = Classifier {
            id: classifier_2160p_id,
            name: "2160p".to_owned(),
            conditions: vec![
                ClassifierCondition::Regex {
                    pattern: r"\b(2160p|4k|uhd)\b".to_owned(),
                    negate: false,
                },
                ClassifierCondition::NumericProperty {
                    property: "size_bytes".to_owned(),
                    min: Some(1_000_000_000),
                    max: None,
                },
            ],
            match_all: true,
            created_at: now,
            updated_at: now,
        };

        let classifier_cam = Classifier {
            id: classifier_cam_id,
            name: "CAM/TS".to_owned(),
            conditions: vec![ClassifierCondition::Regex {
                pattern: r"\b(cam|camrip|telesync|ts)\b".to_owned(),
                negate: false,
            }],
            match_all: false,
            created_at: now,
            updated_at: now,
        };

        let profile = QualityProfile {
            id: Uuid::from_u128(3),
            name: "Ultra-HD".to_owned(),
            upgrade_allowed: true,
            min_score: 500,
            cutoff_score: 5000,
            classifier_scores: vec![
                ProfileClassifierScore {
                    classifier_id: classifier_2160p_id,
                    score: 3000,
                    reject: false,
                },
                ProfileClassifierScore {
                    classifier_id: classifier_cam_id,
                    score: 0,
                    reject: true,
                },
            ],
            created_at: now,
            updated_at: now,
        };

        for item in [&classifier_2160p, &classifier_cam] {
            let json = serde_json::to_string(item).expect("serialize classifier");
            let decoded: Classifier = serde_json::from_str(&json).expect("deserialize classifier");
            assert_eq!(&decoded, item);
        }

        let profile_json = serde_json::to_string(&profile).expect("serialize profile");
        let decoded_profile: QualityProfile =
            serde_json::from_str(&profile_json).expect("deserialize profile");
        assert_eq!(decoded_profile, profile);
    }
}
