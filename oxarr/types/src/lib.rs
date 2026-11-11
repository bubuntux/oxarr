pub mod asset;
pub mod entity;
pub mod media;
pub mod provider;
pub mod quality;
pub mod user;

pub use asset::MediaAsset;
pub use entity::{Entity, EntityDetails, EntityKind, EntityMetadata, Library};
pub use media::{CollectionDetails, MovieDetails};
pub use provider::{
    CollectionMetadata, CollectionSummary, MovieMetadata, ProviderMediaMetadata, SearchQuery,
    SearchResult,
};
pub use quality::{Classifier, ClassifierCondition};
pub use user::User;

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
    fn classifier_json_round_trip() {
        let classifier = Classifier {
            id: Uuid::nil(),
            name: "Remux 2160p".to_owned(),
            conditions: vec![
                ClassifierCondition::TextProperty {
                    property: "source".to_owned(),
                    pattern: "remux".to_owned(),
                    negate: false,
                },
                ClassifierCondition::NumericProperty {
                    property: "height".to_owned(),
                    min: Some(2160),
                    max: None,
                },
            ],
            match_all: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let json = serde_json::to_string(&classifier).expect("serialize Classifier");
        let decoded: Classifier = serde_json::from_str(&json).expect("deserialize Classifier");
        assert_eq!(decoded, classifier);
    }
}
