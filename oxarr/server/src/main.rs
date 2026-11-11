pub mod ui;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Form, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
use maud::Markup;
use oxarr_core::{
    new_id, Database, LibraryScanner, MetadataAggregator, TmdbProvider, TvMazeProvider,
};
use oxarr_types::{
    Entity, EntityDetails, EntityKind, Library, MediaAsset, MovieDetails, SeriesDetails,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db: Database,
    scanner: Arc<LibraryScanner>,
    aggregator: Arc<MetadataAggregator>,
}

#[derive(Deserialize)]
struct CreateLibraryForm {
    name: String,
    root_path: String,
    kind: String,
}

#[derive(Deserialize)]
struct CreateUserForm {
    username: String,
    password: String,
    #[serde(default)]
    is_admin: bool,
}

#[derive(Deserialize)]
struct SearchQueryParams {
    #[serde(default)]
    q: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://./oxarr.db?mode=rwc".to_owned());
    let db = Database::connect(&db_url).await?;

    let mut aggregator = MetadataAggregator::new();
    let tmdb_key = std::env::var("TMDB_API_KEY").ok();
    aggregator.add_provider(Arc::new(TmdbProvider::new(tmdb_key)));
    aggregator.add_provider(Arc::new(TvMazeProvider::new()));
    let aggregator = Arc::new(aggregator);

    let state = AppState {
        db,
        scanner: Arc::new(LibraryScanner::default()),
        aggregator,
    };

    let app = Router::new()
        .route("/", get(|| async { Redirect::to("/libraries") }))
        .route("/libraries", get(list_libraries).post(create_library))
        .route("/libraries/{id}", get(view_library))
        .route("/libraries/{id}/search", get(search_library_media))
        .route("/libraries/{id}/scan", post(scan_library))
        .route("/media/{id}", get(media_detail_page))
        .route("/settings", get(settings_page))
        .route("/settings/users", post(create_user))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("🦀 oxarr daemon running on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn list_libraries(State(state): State<AppState>) -> Markup {
    let libs = state.db.list_libraries().await.unwrap_or_default();
    ui::libraries_page(&libs)
}

async fn create_library(
    State(state): State<AppState>,
    Form(form): Form<CreateLibraryForm>,
) -> Response {
    let kind = EntityKind::from_value(&form.kind).unwrap_or(EntityKind::Movie);
    let lib = Library {
        id: new_id(),
        name: form.name,
        kind,
        root_path: form.root_path,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };
    let _ = state.db.insert_library(&lib).await;
    Redirect::to("/libraries").into_response()
}

async fn view_library(State(state): State<AppState>, Path(id): Path<Uuid>) -> Markup {
    let lib = state
        .db
        .get_library(id)
        .await
        .ok()
        .flatten()
        .unwrap_or_else(|| Library {
            id,
            name: "Library".to_owned(),
            kind: EntityKind::Movie,
            root_path: "".to_owned(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        });

    let entities = state
        .db
        .list_entities_in_library(id)
        .await
        .unwrap_or_default();

    ui::library_media_page(&lib, &entities)
}

async fn search_library_media(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<SearchQueryParams>,
) -> Markup {
    let entities = state
        .db
        .list_entities_in_library(id)
        .await
        .unwrap_or_default();

    let query = params.q.trim().to_lowercase();
    let filtered: Vec<Entity> = if query.is_empty() {
        entities
    } else {
        entities
            .into_iter()
            .filter(|e| e.title.to_lowercase().contains(&query))
            .collect()
    };

    ui::media_cards(&filtered)
}

async fn media_detail_page(State(state): State<AppState>, Path(id): Path<Uuid>) -> Response {
    let Ok(Some(entity)) = state.db.get_entity(id).await else {
        return (StatusCode::NOT_FOUND, "Media title not found").into_response();
    };

    let assets = state
        .db
        .list_entity_media_assets(id)
        .await
        .unwrap_or_default();

    ui::media_detail_page(&entity, &assets).into_response()
}

async fn scan_library(State(state): State<AppState>, Path(id): Path<Uuid>) -> Response {
    let Ok(Some(lib)) = state.db.get_library(id).await else {
        return Html("<button class='btn'>Scan Failed (Not Found)</button>".to_owned())
            .into_response();
    };

    let mut stream = state.scanner.scan_directory(&lib.root_path);
    let db = state.db.clone();
    let aggregator = state.aggregator.clone();
    let lib_kind = lib.kind;

    tokio::spawn(async move {
        while let Some(scanned) = stream.recv().await {
            let entity_id = new_id();

            let mut metadata = oxarr_types::EntityMetadata::default();
            let mut external_ids = std::collections::BTreeMap::new();
            if let Ok(Some(agg)) = aggregator
                .resolve_and_aggregate(&scanned.parsed.title, scanned.parsed.year, lib_kind)
                .await
            {
                metadata.overview = agg.overview().map(str::to_owned);
                metadata.poster_url = agg.poster_url().map(str::to_owned);
                metadata.backdrop_url = agg.backdrop_url().map(str::to_owned);
                metadata.genres = agg.genres().to_vec();
                external_ids = agg.external_ids().clone();
            }

            let details = match lib_kind {
                EntityKind::Movie => EntityDetails::Movie(MovieDetails {
                    year: scanned.parsed.year,
                    ..Default::default()
                }),
                EntityKind::Series => EntityDetails::Series(SeriesDetails {
                    year: scanned.parsed.year,
                    ..Default::default()
                }),
                EntityKind::Collection => {
                    EntityDetails::Collection(oxarr_types::CollectionDetails::default())
                }
            };

            let entity = Entity {
                id: entity_id,
                kind: lib_kind,
                library_id: Some(id),
                title: scanned.parsed.title,
                original_title: None,
                original_language: None,
                monitored: true,
                external_ids,
                metadata,
                details,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            if db.insert_entity(&entity).await.is_ok() {
                let asset = MediaAsset {
                    id: new_id(),
                    entity_ids: vec![entity_id],
                    path: scanned.path.to_string_lossy().to_string(),
                    size_bytes: scanned.size_bytes,
                    hash_xxh3: scanned.hash_xxh3,
                    matched_classifier_ids: Vec::new(),
                    quality_score: None,
                    media_info: serde_json::to_value(&scanned.probe).unwrap_or_default(),
                    imported_at: Utc::now(),
                };
                let _ = db.insert_media_asset(&asset).await;
            }
        }
    });

    Html(
        "<span class='badge' style='color: var(--success); border-color: var(--success);'>Scanning in background...</span>"
            .to_owned(),
    )
    .into_response()
}

async fn settings_page(State(state): State<AppState>) -> Markup {
    let users = state.db.list_users().await.unwrap_or_default();
    ui::settings_page(&users)
}

async fn create_user(
    State(state): State<AppState>,
    Form(form): Form<CreateUserForm>,
) -> Response {
    let password_hash = format!("sha256_{}", form.password);
    let _ = state
        .db
        .insert_user(
            new_id(),
            &form.username,
            &password_hash,
            form.is_admin,
            Utc::now(),
        )
        .await;

    Redirect::to("/settings").into_response()
}
