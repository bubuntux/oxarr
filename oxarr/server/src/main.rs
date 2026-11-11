use std::net::SocketAddr;
use std::sync::Arc;

use askama::Template;
use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
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

fn render<T: Template>(template: T) -> Response {
    match template.render() {
        Ok(html) => Html(html).into_response(),
        Err(err) => {
            tracing::error!(error = %err, "template rendering error");
            (StatusCode::INTERNAL_SERVER_ERROR, "Template error").into_response()
        }
    }
}

fn format_size(bytes: u64) -> String {
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    if bytes as f64 >= GIB {
        format!("{:.2} GB", (bytes as f64) / GIB)
    } else {
        format!("{:.1} MB", (bytes as f64) / MIB)
    }
}

fn extract_streams(media_info: &serde_json::Value) -> Vec<StreamViewModel> {
    let Some(streams_arr) = media_info.get("streams").and_then(|v| v.as_array()) else {
        return Vec::new();
    };

    streams_arr
        .iter()
        .enumerate()
        .map(|(idx, stream)| {
            let stream_type = stream
                .get("codec_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let codec = stream
                .get("codec_name")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let mut details = Vec::new();
            if stream_type == "video" {
                if let (Some(w), Some(h)) = (
                    stream.get("width").and_then(|v| v.as_u64()),
                    stream.get("height").and_then(|v| v.as_u64()),
                ) {
                    details.push(format!("{w}x{h}"));
                }
                if let Some(r_fps) = stream.get("r_frame_rate").and_then(|v| v.as_str()) {
                    details.push(format!("{r_fps} fps"));
                }
                if let Some(pix) = stream.get("pix_fmt").and_then(|v| v.as_str()) {
                    details.push(pix.to_string());
                }
            } else if stream_type == "audio" {
                if let Some(channels) = stream.get("channels").and_then(|v| v.as_u64()) {
                    let layout = match channels {
                        8 => "7.1 surround".to_owned(),
                        6 => "5.1 surround".to_owned(),
                        2 => "Stereo".to_owned(),
                        1 => "Mono".to_owned(),
                        n => format!("{n} channels"),
                    };
                    details.push(layout);
                }
                if let Some(hz) = stream.get("sample_rate").and_then(|v| v.as_str()) {
                    details.push(format!("{hz} Hz"));
                }
            }

            if let Some(lang) = stream
                .get("tags")
                .and_then(|t| t.get("language"))
                .and_then(|l| l.as_str())
            {
                details.push(format!("[{lang}]"));
            }

            if let Some(disp) = stream.get("disposition") {
                if disp.get("forced").and_then(|v| v.as_i64()) == Some(1) {
                    details.push("Forced".to_owned());
                }
                if disp.get("default").and_then(|v| v.as_i64()) == Some(1) {
                    details.push("Default".to_owned());
                }
            }

            StreamViewModel {
                index: stream
                    .get("index")
                    .and_then(|v| v.as_u64())
                    .map(|i| i as usize)
                    .unwrap_or(idx),
                stream_type,
                codec,
                details: details.join(" • "),
            }
        })
        .collect()
}

#[derive(Template)]
#[template(path = "libraries.html")]
struct LibrariesTemplate {
    current_nav: &'static str,
    libraries: Vec<LibraryViewModel>,
}

struct LibraryViewModel {
    id: Uuid,
    name: String,
    kind: &'static str,
    root_path: String,
}

#[derive(Template)]
#[template(path = "media_grid.html")]
struct MediaGridTemplate {
    current_nav: &'static str,
    library: Library,
    items: Vec<MediaItemViewModel>,
}

struct MediaItemViewModel {
    id: Uuid,
    title: String,
    kind: &'static str,
    year_display: String,
    monitored: bool,
    metadata: oxarr_types::EntityMetadata,
}

#[derive(Template)]
#[template(path = "media_detail.html")]
struct MediaDetailTemplate {
    current_nav: &'static str,
    entity: Entity,
    kind_label: &'static str,
    year_display: String,
    runtime_display: String,
    assets: Vec<MediaAssetViewModel>,
}

#[allow(dead_code)]
struct MediaAssetViewModel {
    id: Uuid,
    path: String,
    size_formatted: String,
    hash_xxh3: Option<String>,
    imported_at: String,
    streams: Vec<StreamViewModel>,
}

struct StreamViewModel {
    index: usize,
    stream_type: String,
    codec: String,
    details: String,
}

#[derive(Template)]
#[template(path = "settings.html")]
struct SettingsTemplate {
    current_nav: &'static str,
    users: Vec<UserViewModel>,
}

struct UserViewModel {
    username: String,
    is_admin: bool,
    created_at: String,
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

async fn list_libraries(State(state): State<AppState>) -> Response {
    let libs = state.db.list_libraries().await.unwrap_or_default();
    let view_models = libs
        .into_iter()
        .map(|l| {
            let kind = match l.kind {
                EntityKind::Movie => "Movie",
                EntityKind::Collection => "Collection",
                EntityKind::Series => "Series",
            };
            LibraryViewModel {
                id: l.id,
                name: l.name,
                kind,
                root_path: l.root_path,
            }
        })
        .collect();

    render(LibrariesTemplate {
        current_nav: "libraries",
        libraries: view_models,
    })
}

async fn create_library(
    State(state): State<AppState>,
    Form(form): Form<CreateLibraryForm>,
) -> Response {
    let kind = match form.kind.as_str() {
        "series" => EntityKind::Series,
        _ => EntityKind::Movie,
    };
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

async fn view_library(State(state): State<AppState>, Path(id): Path<Uuid>) -> Response {
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
    let items = entities
        .into_iter()
        .map(|e| {
            let year_display = match &e.details {
                EntityDetails::Movie(m) => m.year.map(|y| y.to_string()).unwrap_or_default(),
                EntityDetails::Series(s) => s.year.map(|y| y.to_string()).unwrap_or_default(),
                _ => "".to_owned(),
            };
            let kind = match e.kind {
                EntityKind::Movie => "Movie",
                EntityKind::Collection => "Collection",
                EntityKind::Series => "Series",
            };
            MediaItemViewModel {
                id: e.id,
                title: e.title,
                kind,
                year_display,
                monitored: e.monitored,
                metadata: e.metadata,
            }
        })
        .collect();

    render(MediaGridTemplate {
        current_nav: "libraries",
        library: lib,
        items,
    })
}

async fn media_detail_page(State(state): State<AppState>, Path(id): Path<Uuid>) -> Response {
    let Ok(Some(entity)) = state.db.get_entity(id).await else {
        return (StatusCode::NOT_FOUND, "Media title not found").into_response();
    };

    let kind_label = match entity.kind {
        EntityKind::Movie => "Movie",
        EntityKind::Collection => "Collection",
        EntityKind::Series => "Series",
    };

    let (year_display, runtime_display) = match &entity.details {
        EntityDetails::Movie(m) => (
            m.year.map(|y| y.to_string()).unwrap_or_default(),
            m.runtime_minutes.map(|r| format!("{r}m")).unwrap_or_default(),
        ),
        EntityDetails::Series(s) => (
            s.year.map(|y| y.to_string()).unwrap_or_default(),
            s.status.clone().unwrap_or_default(),
        ),
        EntityDetails::Collection(_) => ("".to_owned(), "".to_owned()),
    };

    let db_assets = state
        .db
        .list_entity_media_assets(id)
        .await
        .unwrap_or_default();
    let assets = db_assets
        .into_iter()
        .map(|a| {
            let streams = extract_streams(&a.media_info);
            MediaAssetViewModel {
                id: a.id,
                path: a.path,
                size_formatted: format_size(a.size_bytes),
                hash_xxh3: a.hash_xxh3,
                imported_at: a.imported_at.format("%Y-%m-%d %H:%M").to_string(),
                streams,
            }
        })
        .collect();

    render(MediaDetailTemplate {
        current_nav: "libraries",
        entity,
        kind_label,
        year_display,
        runtime_display,
        assets,
    })
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

async fn settings_page(State(state): State<AppState>) -> Response {
    let db_users = state.db.list_users().await.unwrap_or_default();
    let users = db_users
        .into_iter()
        .map(|u| UserViewModel {
            username: u.username,
            is_admin: u.is_admin,
            created_at: u.created_at.format("%Y-%m-%d").to_string(),
        })
        .collect();

    render(SettingsTemplate {
        current_nav: "settings",
        users,
    })
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
