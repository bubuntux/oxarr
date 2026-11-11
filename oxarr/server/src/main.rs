use std::net::SocketAddr;
use std::sync::Arc;

use askama::Template;
use axum::extract::{Form, Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
use oxarr_core::{new_id, Database, LibraryScanner};
use oxarr_types::{Entity, EntityDetails, EntityKind, Library, MovieDetails};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db: Database,
    scanner: Arc<LibraryScanner>,
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
    title: String,
    kind: &'static str,
    year_display: String,
    monitored: bool,
    metadata: oxarr_types::EntityMetadata,
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

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://./oxarr.db?mode=rwc".to_owned());
    let db = Database::connect(&db_url).await?;

    let state = AppState {
        db,
        scanner: Arc::new(LibraryScanner::default()),
    };

    let app = Router::new()
        .route("/", get(|| async { Redirect::to("/libraries") }))
        .route("/libraries", get(list_libraries).post(create_library))
        .route("/libraries/{id}", get(view_library))
        .route("/libraries/{id}/scan", post(scan_library))
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
        "collection" => EntityKind::Collection,
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

async fn view_library(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    let lib = state.db.get_library(id).await.ok().flatten().unwrap_or_else(|| Library {
        id,
        name: "Library".to_owned(),
        kind: EntityKind::Movie,
        root_path: "".to_owned(),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    });

    let entities = state.db.list_entities_in_library(id).await.unwrap_or_default();
    let items = entities
        .into_iter()
        .map(|e| {
            let year_display = match &e.details {
                EntityDetails::Movie(m) => m.year.map(|y| y.to_string()).unwrap_or_default(),
                _ => "".to_owned(),
            };
            let kind = match e.kind {
                EntityKind::Movie => "Movie",
                EntityKind::Collection => "Collection",
            };
            MediaItemViewModel {
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

async fn scan_library(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Response {
    let Ok(Some(lib)) = state.db.get_library(id).await else {
        return Html("<button class='btn'>Scan Failed (Not Found)</button>".to_owned()).into_response();
    };

    let mut stream = state.scanner.scan_directory(&lib.root_path);
    let db = state.db.clone();

    tokio::spawn(async move {
        while let Some(scanned) = stream.recv().await {
            let entity_id = new_id();
            let entity = Entity {
                id: entity_id,
                kind: EntityKind::Movie,
                library_id: Some(id),
                title: scanned.parsed.title,
                original_title: None,
                original_language: None,
                monitored: true,
                external_ids: std::collections::BTreeMap::new(),
                metadata: oxarr_types::EntityMetadata::default(),
                details: EntityDetails::Movie(MovieDetails {
                    year: scanned.parsed.year,
                    ..Default::default()
                }),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            };

            let _ = db.insert_entity(&entity).await;
        }
    });

    Html("<span class='badge' style='color: var(--success); border-color: var(--success);'>Scanning in background...</span>".to_owned()).into_response()
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
        .insert_user(new_id(), &form.username, &password_hash, form.is_admin, Utc::now())
        .await;

    Redirect::to("/settings").into_response()
}
