pub mod auth;
pub mod ui;

use std::io::IsTerminal;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{Form, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
use maud::Markup;
use oxarr_core::{
    new_id, Database, LibraryScanner, MetadataAggregator, TmdbProvider, TvMazeProvider,
};
use oxarr_types::{
    Entity, EntityDetails, EntityKind, Library, MediaAsset, MovieDetails, SeriesDetails, User,
};
use serde::Deserialize;
use tower_http::trace::TraceLayer;
use tower_sessions::session_store::ExpiredDeletion;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

use crate::auth::{
    AdminUser, AuthAccount, Credentials, CurrentUser, OxarrAuthSession, SeaOrmSessionStore,
};
use crate::ui::layout::LOGO_SVG;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) db: Database,
    pub(crate) scanner: Arc<LibraryScanner>,
    pub(crate) aggregator: Arc<MetadataAggregator>,
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
struct SetupForm {
    username: String,
    password: String,
    confirm_password: String,
}

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
    #[serde(default)]
    next: Option<String>,
}

#[derive(Deserialize)]
struct ChangePasswordForm {
    current_password: String,
    new_password: String,
    confirm_password: String,
}

#[derive(Deserialize)]
struct ResetPasswordForm {
    password: String,
}

#[derive(Deserialize)]
struct LoginQueryParams {
    #[serde(default)]
    next: Option<String>,
}

#[derive(Deserialize)]
struct SettingsQueryParams {
    #[serde(default)]
    notice: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Deserialize)]
struct SearchQueryParams {
    #[serde(default)]
    q: String,
}

pub(crate) fn build_app_with_store(state: AppState) -> (Router, SeaOrmSessionStore) {
    let (auth_layer, session_store) = auth::build_auth_layer(state.db.clone());

    let protected = Router::new()
        .route("/", get(|| async { Redirect::to("/libraries") }))
        .route("/libraries", get(list_libraries).post(create_library))
        .route("/libraries/{id}", get(view_library))
        .route("/libraries/{id}/edit", post(update_library))
        .route("/libraries/{id}/delete", post(delete_library))
        .route("/libraries/{id}/search", get(search_library_media))
        .route("/libraries/{id}/scan", post(scan_library))
        .route("/media/{id}", get(media_detail_page))
        .route("/account", get(account_page))
        .route("/account/password", post(change_own_password))
        .route("/settings", get(settings_page))
        .route("/settings/users", post(create_user))
        .route(
            "/settings/users/{id}/libraries",
            post(update_user_libraries),
        )
        .route(
            "/settings/users/{id}/password",
            post(admin_reset_user_password),
        )
        .route("/settings/users/{id}/delete", post(delete_user))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            auth::require_auth,
        ));

    let router = Router::new()
        .route("/logo.svg", get(logo_handler))
        .route("/favicon.ico", get(logo_handler))
        .route("/favicon.svg", get(logo_handler))
        .route("/setup", get(setup_page).post(setup_submit))
        .route("/login", get(login_page).post(login_submit))
        .route("/logout", post(logout_submit))
        .merge(protected)
        .layer(auth_layer)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    (router, session_store)
}

#[cfg(test)]
pub(crate) fn build_app(state: AppState) -> Router {
    build_app_with_store(state).0
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let use_ansi = should_use_ansi();

    tracing_subscriber::fmt()
        .compact()
        .with_target(false)
        .with_ansi(use_ansi)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("oxarr=debug,tower_http=debug,info")),
        )
        .init();

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| default_database_url());
    let db = Database::connect(&db_url).await?;
    let user_count = db.count_users().await.unwrap_or(0);

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

    let (app, session_store) = build_app_with_store(state);

    if let Err(err) = session_store.delete_expired().await {
        tracing::warn!(error = %err, "failed to prune expired sessions on startup");
    }
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Err(err) = session_store.delete_expired().await {
                tracing::warn!(error = %err, "failed to prune expired sessions");
            }
        }
    });

    let host: IpAddr = std::env::var("OXARR_HOST")
        .or_else(|_| std::env::var("HOST"))
        .unwrap_or_else(|_| "0.0.0.0".to_string())
        .parse()
        .unwrap_or_else(|_| IpAddr::from([0, 0, 0, 0]));

    let port: u16 = std::env::var("OXARR_PORT")
        .or_else(|_| std::env::var("PORT"))
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);

    let addr = SocketAddr::new(host, port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let bound_addr = listener.local_addr()?;

    eprintln!();
    eprintln!("  🦀 oxarr server ready");
    if bound_addr.ip().is_unspecified() {
        eprintln!("  ➜  Local:    http://localhost:{}/", bound_addr.port());
        eprintln!("  ➜  Network:  http://{bound_addr}/");
    } else {
        eprintln!("  ➜  Ready at: http://{bound_addr}/");
    }
    eprintln!("  ➜  Database: {db_url}");
    if user_count == 0 {
        eprintln!("  ➜  Setup:    create initial admin account at /setup");
    }
    eprintln!();

    tracing::info!("oxarr daemon running on http://{bound_addr}");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn setup_page(State(state): State<AppState>) -> Response {
    if state.db.count_users().await.unwrap_or(0) > 0 {
        return Redirect::to("/login").into_response();
    }
    ui::setup_page(None).into_response()
}

async fn setup_submit(
    State(state): State<AppState>,
    mut auth_session: OxarrAuthSession,
    Form(form): Form<SetupForm>,
) -> Response {
    if state.db.count_users().await.unwrap_or(0) > 0 {
        return Redirect::to("/login").into_response();
    }

    let username = match oxarr_core::auth::validate_username(&form.username) {
        Ok(u) => u,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                ui::setup_page(Some(&err.to_string())),
            )
                .into_response();
        }
    };

    if let Err(err) = oxarr_core::auth::validate_password(&form.password) {
        return (
            StatusCode::BAD_REQUEST,
            ui::setup_page(Some(&err.to_string())),
        )
            .into_response();
    }

    if form.password != form.confirm_password {
        return (
            StatusCode::BAD_REQUEST,
            ui::setup_page(Some("Passwords do not match")),
        )
            .into_response();
    }

    let Ok(password_hash) = auth::hash_password_blocking(form.password).await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            ui::setup_page(Some("Failed to hash password")),
        )
            .into_response();
    };

    match state
        .db
        .create_initial_admin(new_id(), &username, &password_hash, Utc::now())
        .await
    {
        Ok(Some(user)) => {
            let account = AuthAccount {
                user,
                password_hash,
            };
            if auth_session.login(&account).await.is_err() {
                return Redirect::to("/login").into_response();
            }
            Redirect::to("/libraries").into_response()
        }
        Ok(None) => Redirect::to("/login").into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            ui::setup_page(Some("Failed to create administrator account")),
        )
            .into_response(),
    }
}

async fn login_page(
    State(state): State<AppState>,
    auth_session: OxarrAuthSession,
    Query(query): Query<LoginQueryParams>,
) -> Response {
    if state.db.count_users().await.unwrap_or(0) == 0 {
        return Redirect::to("/setup").into_response();
    }

    let safe_next = query
        .next
        .as_deref()
        .filter(|n| auth::sanitize_next_url(Some(n)) == *n);

    if auth_session.user.is_some() {
        return Redirect::to(safe_next.unwrap_or("/libraries")).into_response();
    }

    ui::login_page(None, safe_next).into_response()
}

async fn login_submit(
    State(state): State<AppState>,
    mut auth_session: OxarrAuthSession,
    Form(form): Form<LoginForm>,
) -> Response {
    if state.db.count_users().await.unwrap_or(0) == 0 {
        return Redirect::to("/setup").into_response();
    }

    let safe_next = form
        .next
        .as_deref()
        .filter(|n| auth::sanitize_next_url(Some(n)) == *n);

    let creds = Credentials {
        username: form.username,
        password: form.password,
    };

    match auth_session.authenticate(creds).await {
        Ok(Some(account)) => {
            if auth_session.login(&account).await.is_err() {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ui::login_page(Some("Failed to establish session"), safe_next),
                )
                    .into_response();
            }
            Redirect::to(auth::sanitize_next_url(safe_next)).into_response()
        }
        Ok(None) => (
            StatusCode::UNAUTHORIZED,
            ui::login_page(Some("Invalid username or password"), safe_next),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            ui::login_page(Some("Authentication error"), safe_next),
        )
            .into_response(),
    }
}

async fn logout_submit(mut auth_session: OxarrAuthSession) -> Response {
    let _ = auth_session.logout().await;
    Redirect::to("/login").into_response()
}

async fn render_libraries(
    state: &AppState,
    user: &User,
    notice: Option<&str>,
    error: Option<&str>,
) -> Markup {
    let libs = state
        .db
        .list_accessible_libraries(user)
        .await
        .unwrap_or_default();
    ui::libraries_page(&libs, user, notice, error)
}

async fn list_libraries(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Query(query): Query<SettingsQueryParams>,
) -> Markup {
    render_libraries(
        &state,
        &user,
        query.notice.as_deref(),
        query.error.as_deref(),
    )
    .await
}

fn validate_library_form(form: &CreateLibraryForm) -> Result<(String, String, EntityKind), &'static str> {
    let name = form.name.trim();
    if name.is_empty() {
        return Err("Library name cannot be empty");
    }
    let root_path = form.root_path.trim();
    if root_path.is_empty() {
        return Err("Root folder path cannot be empty");
    }
    let kind = EntityKind::from_value(&form.kind)
        .filter(|k| EntityKind::LIBRARY_KINDS.contains(k))
        .ok_or("Invalid library media type")?;
    Ok((name.to_owned(), root_path.to_owned(), kind))
}

async fn create_library(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Form(form): Form<CreateLibraryForm>,
) -> Response {
    let (name, root_path, kind) = match validate_library_form(&form) {
        Ok(vals) => vals,
        Err(msg) => {
            return (
                StatusCode::BAD_REQUEST,
                render_libraries(&state, &admin, None, Some(msg)).await,
            )
                .into_response();
        }
    };

    let now = Utc::now();
    let lib = Library {
        id: new_id(),
        name,
        kind,
        root_path,
        created_at: now,
        updated_at: now,
    };
    if let Err(err) = state.db.insert_library(&lib).await {
        tracing::warn!(error = %err, "failed to create library");
        return (
            StatusCode::BAD_REQUEST,
            render_libraries(
                &state,
                &admin,
                None,
                Some("Could not create library (name or root path may already exist, or type is unsupported)"),
            )
            .await,
        )
            .into_response();
    }

    Redirect::to("/libraries?notice=Library+created").into_response()
}

async fn update_library(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(id): Path<Uuid>,
    Form(form): Form<CreateLibraryForm>,
) -> Response {
    let Ok(Some(mut existing)) = state.db.get_library(id).await else {
        return (StatusCode::NOT_FOUND, "Library not found").into_response();
    };

    let (name, root_path, kind) = match validate_library_form(&form) {
        Ok(vals) => vals,
        Err(msg) => {
            return (
                StatusCode::BAD_REQUEST,
                render_libraries(&state, &admin, None, Some(msg)).await,
            )
                .into_response();
        }
    };

    existing.name = name;
    existing.root_path = root_path;
    existing.kind = kind;
    existing.updated_at = Utc::now();

    if let Err(err) = state.db.update_library(&existing).await {
        tracing::warn!(library_id = %id, error = %err, "failed to update library");
        return (
            StatusCode::BAD_REQUEST,
            render_libraries(
                &state,
                &admin,
                None,
                Some("Could not update library (name or root path may already exist)"),
            )
            .await,
        )
            .into_response();
    }

    Redirect::to("/libraries?notice=Library+updated").into_response()
}

async fn delete_library(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<Uuid>,
) -> Response {
    match state.db.delete_library(id).await {
        Ok(true) => Redirect::to("/libraries?notice=Library+deleted").into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "Library not found").into_response(),
        Err(err) => {
            tracing::error!(library_id = %id, error = %err, "failed to delete library");
            (StatusCode::INTERNAL_SERVER_ERROR, "Failed to delete library").into_response()
        }
    }
}

async fn view_library(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Response {
    let Ok(Some(lib)) = state.db.get_library(id).await else {
        return (StatusCode::NOT_FOUND, "Library not found").into_response();
    };

    if !state
        .db
        .user_can_access_library(user.id, id)
        .await
        .unwrap_or(false)
    {
        return (StatusCode::FORBIDDEN, "Access denied").into_response();
    }

    let entities = state
        .db
        .list_entities_in_library(id)
        .await
        .unwrap_or_default();

    ui::library_media_page(&lib, &entities, &user).into_response()
}

async fn search_library_media(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
    Query(params): Query<SearchQueryParams>,
) -> Response {
    if !state
        .db
        .user_can_access_library(user.id, id)
        .await
        .unwrap_or(false)
    {
        return (StatusCode::FORBIDDEN, "Access denied").into_response();
    }

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

    ui::media_cards(&filtered).into_response()
}

async fn media_detail_page(
    State(state): State<AppState>,
    CurrentUser(user): CurrentUser,
    Path(id): Path<Uuid>,
) -> Response {
    let Ok(Some(entity)) = state.db.get_entity(id).await else {
        return (StatusCode::NOT_FOUND, "Media title not found").into_response();
    };

    let allowed = match entity.library_id {
        Some(lib_id) => state
            .db
            .user_can_access_library(user.id, lib_id)
            .await
            .unwrap_or(false),
        None => user.is_admin,
    };
    if !allowed {
        return (StatusCode::FORBIDDEN, "Access denied").into_response();
    }

    let assets = state
        .db
        .list_entity_media_assets(id)
        .await
        .unwrap_or_default();

    ui::media_detail_page(&entity, &assets, &user).into_response()
}

async fn scan_library(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<Uuid>,
) -> Response {
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

async fn account_page(CurrentUser(user): CurrentUser) -> Markup {
    ui::account_page(&user, None, None)
}

async fn change_own_password(
    State(state): State<AppState>,
    mut auth_session: OxarrAuthSession,
    Form(form): Form<ChangePasswordForm>,
) -> Response {
    let Some(current_account) = auth_session.user.clone() else {
        return (StatusCode::UNAUTHORIZED, "Authentication required").into_response();
    };
    let user = current_account.user;

    if !auth::verify_password_blocking(form.current_password, current_account.password_hash)
        .await
        .unwrap_or(false)
    {
        return (
            StatusCode::BAD_REQUEST,
            ui::account_page(&user, None, Some("Current password is incorrect")),
        )
            .into_response();
    }

    if let Err(err) = oxarr_core::auth::validate_password(&form.new_password) {
        return (
            StatusCode::BAD_REQUEST,
            ui::account_page(&user, None, Some(&err.to_string())),
        )
            .into_response();
    }

    if form.new_password != form.confirm_password {
        return (
            StatusCode::BAD_REQUEST,
            ui::account_page(&user, None, Some("New passwords do not match")),
        )
            .into_response();
    }

    let Ok(new_hash) = auth::hash_password_blocking(form.new_password).await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            ui::account_page(&user, None, Some("Failed to hash new password")),
        )
            .into_response();
    };

    if !state
        .db
        .update_user_password(user.id, &new_hash, Utc::now())
        .await
        .unwrap_or(false)
    {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            ui::account_page(&user, None, Some("Failed to update password")),
        )
            .into_response();
    }

    // Rotate this session's ID and update its stored auth_hash; other sessions holding
    // the previous auth_hash are automatically invalidated by `axum-login`.
    let _ = auth_session.session.cycle_id().await;
    let updated_account = AuthAccount {
        user: user.clone(),
        password_hash: new_hash,
    };
    let _ = auth_session.login(&updated_account).await;

    ui::account_page(&user, Some("Password updated"), None).into_response()
}

async fn render_settings(
    state: &AppState,
    admin: &User,
    notice: Option<&str>,
    error: Option<&str>,
) -> Markup {
    let users = state.db.list_users().await.unwrap_or_default();
    let libraries = state.db.list_libraries().await.unwrap_or_default();

    let mut users_with_libs = Vec::with_capacity(users.len());
    for u in users {
        let libs = if u.is_admin {
            Vec::new()
        } else {
            state.db.list_user_libraries(u.id).await.unwrap_or_default()
        };
        users_with_libs.push((u, libs));
    }

    ui::settings_page(&users_with_libs, &libraries, admin, notice, error)
}

async fn settings_page(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Query(query): Query<SettingsQueryParams>,
) -> Markup {
    render_settings(
        &state,
        &admin,
        query.notice.as_deref(),
        query.error.as_deref(),
    )
    .await
}

async fn create_user(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Form(form): Form<CreateUserForm>,
) -> Response {
    let username = match oxarr_core::auth::validate_username(&form.username) {
        Ok(u) => u,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                render_settings(&state, &admin, None, Some(&err.to_string())).await,
            )
                .into_response();
        }
    };

    if let Err(err) = oxarr_core::auth::validate_password(&form.password) {
        return (
            StatusCode::BAD_REQUEST,
            render_settings(&state, &admin, None, Some(&err.to_string())).await,
        )
            .into_response();
    }

    let Ok(password_hash) = auth::hash_password_blocking(form.password).await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            render_settings(&state, &admin, None, Some("Failed to hash password")).await,
        )
            .into_response();
    };

    if state
        .db
        .insert_user(
            new_id(),
            &username,
            &password_hash,
            form.is_admin,
            Utc::now(),
        )
        .await
        .is_err()
    {
        return (
            StatusCode::BAD_REQUEST,
            render_settings(
                &state,
                &admin,
                None,
                Some("Could not create user (username may already exist)"),
            )
            .await,
        )
            .into_response();
    }

    Redirect::to("/settings?notice=User+created").into_response()
}

async fn update_user_libraries(
    State(state): State<AppState>,
    AdminUser(_admin): AdminUser,
    Path(id): Path<Uuid>,
    body: String,
) -> Response {
    let Ok(library_ids) = auth::parse_library_ids_form(&body) else {
        return (StatusCode::BAD_REQUEST, "Invalid library ID").into_response();
    };

    let Ok(Some(_)) = state.db.get_user(id).await else {
        return (StatusCode::NOT_FOUND, "User not found").into_response();
    };

    if state.db.set_user_libraries(id, &library_ids).await.is_err() {
        return (StatusCode::BAD_REQUEST, "Failed to update library access").into_response();
    }

    Redirect::to("/settings?notice=Library+access+updated").into_response()
}

async fn admin_reset_user_password(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(id): Path<Uuid>,
    Form(form): Form<ResetPasswordForm>,
) -> Response {
    if let Err(err) = oxarr_core::auth::validate_password(&form.password) {
        return (
            StatusCode::BAD_REQUEST,
            render_settings(&state, &admin, None, Some(&err.to_string())).await,
        )
            .into_response();
    }

    let Ok(password_hash) = auth::hash_password_blocking(form.password).await else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            render_settings(&state, &admin, None, Some("Failed to hash password")).await,
        )
            .into_response();
    };

    match state
        .db
        .update_user_password(id, &password_hash, Utc::now())
        .await
    {
        Ok(true) => Redirect::to("/settings?notice=Password+updated").into_response(),
        Ok(false) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to update password",
        )
            .into_response(),
    }
}

async fn delete_user(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    Path(id): Path<Uuid>,
) -> Response {
    if id == admin.id {
        return (
            StatusCode::BAD_REQUEST,
            render_settings(
                &state,
                &admin,
                None,
                Some("You cannot delete your own account"),
            )
            .await,
        )
            .into_response();
    }

    let Ok(Some(target)) = state.db.get_user(id).await else {
        return (StatusCode::NOT_FOUND, "User not found").into_response();
    };

    if target.is_admin && state.db.count_admins().await.unwrap_or(0) <= 1 {
        return (
            StatusCode::BAD_REQUEST,
            render_settings(
                &state,
                &admin,
                None,
                Some("Cannot delete the last administrator account"),
            )
            .await,
        )
            .into_response();
    }

    let _ = state.db.delete_user(id).await;
    Redirect::to("/settings?notice=User+deleted").into_response()
}

async fn logo_handler() -> impl IntoResponse {
    (
        [
            (header::CONTENT_TYPE, "image/svg+xml"),
            (header::CACHE_CONTROL, "public, max-age=604800, immutable"),
        ],
        LOGO_SVG,
    )
}

fn default_database_url() -> String {
    default_database_url_from(
        std::env::var("OXARR_DATA_DIR").ok().as_deref(),
        std::env::var("XDG_DATA_HOME").ok().as_deref(),
        std::env::var("HOME").ok().as_deref(),
    )
}

fn default_database_url_from(
    oxarr_data_dir: Option<&str>,
    xdg_data_home: Option<&str>,
    home: Option<&str>,
) -> String {
    let base_dir = if let Some(dir) = oxarr_data_dir.filter(|s| !s.trim().is_empty()) {
        std::path::PathBuf::from(dir)
    } else if let Some(xdg) = xdg_data_home.filter(|s| !s.trim().is_empty()) {
        std::path::PathBuf::from(xdg).join("oxarr")
    } else if let Some(home_dir) = home.filter(|s| !s.trim().is_empty()) {
        std::path::PathBuf::from(home_dir)
            .join(".local")
            .join("share")
            .join("oxarr")
    } else {
        std::path::PathBuf::from(".oxarr")
    };

    format!("sqlite://{}?mode=rwc", base_dir.join("oxarr.db").display())
}

fn should_use_ansi() -> bool {
    if std::env::var("NO_COLOR").is_ok() {
        return false;
    }
    if let Ok(force) = std::env::var("CLICOLOR_FORCE") {
        return force != "0";
    }
    if let Ok(clicolor) = std::env::var("CLICOLOR") {
        if clicolor == "0" {
            return false;
        }
    }
    std::io::stderr().is_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::{header, Method, Request};
    use tower::ServiceExt;

    async fn test_app() -> (Router, Database) {
        let db = Database::connect_in_memory()
            .await
            .expect("connect in-memory sqlite");
        let state = AppState {
            db: db.clone(),
            scanner: Arc::new(LibraryScanner::default()),
            aggregator: Arc::new(MetadataAggregator::new()),
        };
        (build_app(state), db)
    }

    async fn send(app: &Router, req: Request<Body>) -> Response {
        app.clone().oneshot(req).await.expect("service call")
    }

    async fn body_string(resp: Response) -> String {
        let bytes = to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .expect("read body");
        String::from_utf8(bytes.to_vec()).expect("utf8 body")
    }

    fn extract_cookie_pair(resp: &Response) -> String {
        for raw_val in resp.headers().get_all(header::SET_COOKIE) {
            if let Ok(raw) = raw_val.to_str() {
                if raw.starts_with("oxarr_session=") {
                    return raw.split(';').next().unwrap().trim().to_owned();
                }
            }
        }
        panic!("oxarr_session Set-Cookie header missing");
    }

    fn post_form(uri: &str, form_body: &str, cookie: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded");
        if let Some(c) = cookie {
            builder = builder.header(header::COOKIE, c);
        }
        builder.body(Body::from(form_body.to_owned())).unwrap()
    }

    fn get_req(uri: &str, cookie: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().method(Method::GET).uri(uri);
        if let Some(c) = cookie {
            builder = builder.header(header::COOKIE, c);
        }
        builder.body(Body::empty()).unwrap()
    }

    async fn bootstrap_admin(app: &Router) -> String {
        let resp = send(
            app,
            post_form(
                "/setup",
                "username=admin&password=adminpassword1&confirm_password=adminpassword1",
                None,
            ),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        assert_eq!(resp.headers().get(header::LOCATION).unwrap(), "/libraries");
        extract_cookie_pair(&resp)
    }

    #[test]
    fn resolves_default_database_url_precedence() {
        assert_eq!(
            default_database_url_from(Some("/srv/oxarr"), Some("/xdg"), Some("/home/u")),
            "sqlite:///srv/oxarr/oxarr.db?mode=rwc"
        );
        assert_eq!(
            default_database_url_from(None, Some("/xdg"), Some("/home/u")),
            "sqlite:///xdg/oxarr/oxarr.db?mode=rwc"
        );
        assert_eq!(
            default_database_url_from(None, Some(""), Some("/home/u")),
            "sqlite:///home/u/.local/share/oxarr/oxarr.db?mode=rwc"
        );
        assert_eq!(
            default_database_url_from(None, None, None),
            "sqlite://.oxarr/oxarr.db?mode=rwc"
        );
    }

    #[tokio::test]
    async fn test_logo_and_favicon_endpoint() {
        let resp = logo_handler().await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "image/svg+xml"
        );
        assert_eq!(
            resp.headers().get(header::CACHE_CONTROL).unwrap(),
            "public, max-age=604800, immutable"
        );
        assert!(LOGO_SVG.contains("<svg"));
        assert!(LOGO_SVG.contains("</svg>"));

        let (app, _) = test_app().await;
        let unauth_logo = send(&app, get_req("/logo.svg", None)).await;
        assert_eq!(unauth_logo.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn first_run_setup_flow_and_lockout() {
        let (app, db) = test_app().await;

        let libs_resp = send(&app, get_req("/libraries", None)).await;
        assert_eq!(libs_resp.status(), StatusCode::SEE_OTHER);
        assert_eq!(libs_resp.headers().get(header::LOCATION).unwrap(), "/setup");

        let login_resp = send(&app, get_req("/login", None)).await;
        assert_eq!(login_resp.status(), StatusCode::SEE_OTHER);
        assert_eq!(login_resp.headers().get(header::LOCATION).unwrap(), "/setup");

        // Short password rejected with 400
        let short_resp = send(
            &app,
            post_form(
                "/setup",
                "username=admin&password=short&confirm_password=short",
                None,
            ),
        )
        .await;
        assert_eq!(short_resp.status(), StatusCode::BAD_REQUEST);

        // Mismatched confirm rejected with 400
        let mismatch_resp = send(
            &app,
            post_form(
                "/setup",
                "username=admin&password=adminpassword1&confirm_password=adminpassword2",
                None,
            ),
        )
        .await;
        assert_eq!(mismatch_resp.status(), StatusCode::BAD_REQUEST);

        // Successful setup sets HttpOnly cookie and creates admin
        let resp = send(
            &app,
            post_form(
                "/setup",
                "username=admin&password=adminpassword1&confirm_password=adminpassword1",
                None,
            ),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::SEE_OTHER);
        assert_eq!(resp.headers().get(header::LOCATION).unwrap(), "/libraries");
        let set_cookie = resp
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(set_cookie.contains("oxarr_session="));
        assert!(set_cookie.contains("HttpOnly"));
        assert!(set_cookie.contains("SameSite=Lax"));
        assert_eq!(db.count_users().await.unwrap(), 1);

        // Subsequent setup requests redirect to /login
        let setup_again_get = send(&app, get_req("/setup", None)).await;
        assert_eq!(setup_again_get.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            setup_again_get.headers().get(header::LOCATION).unwrap(),
            "/login"
        );

        let setup_again_post = send(
            &app,
            post_form(
                "/setup",
                "username=evil&password=adminpassword1&confirm_password=adminpassword1",
                None,
            ),
        )
        .await;
        assert_eq!(setup_again_post.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            setup_again_post.headers().get(header::LOCATION).unwrap(),
            "/login"
        );
        assert_eq!(db.count_users().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn login_logout_and_htmx_redirect() {
        let (app, _) = test_app().await;
        let admin_cookie = bootstrap_admin(&app).await;

        // Authenticated request succeeds
        let ok_resp = send(&app, get_req("/libraries", Some(&admin_cookie))).await;
        assert_eq!(ok_resp.status(), StatusCode::OK);

        // Logout clears cookie and flushes session from SeaORM store
        let logout_resp = send(&app, post_form("/logout", "", Some(&admin_cookie))).await;
        assert_eq!(logout_resp.status(), StatusCode::SEE_OTHER);
        assert_eq!(logout_resp.headers().get(header::LOCATION).unwrap(), "/login");
        let cleared = logout_resp
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(cleared.contains("oxarr_session=;"));

        // Old cookie is now rejected
        let after_logout = send(&app, get_req("/libraries", Some(&admin_cookie))).await;
        assert_eq!(after_logout.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            after_logout.headers().get(header::LOCATION).unwrap(),
            "/login"
        );

        // Unauthenticated htmx request returns HX-Redirect
        let htmx_req = Request::builder()
            .method(Method::GET)
            .uri("/libraries")
            .header("HX-Request", "true")
            .body(Body::empty())
            .unwrap();
        let htmx_resp = send(&app, htmx_req).await;
        assert_eq!(htmx_resp.status(), StatusCode::OK);
        assert_eq!(htmx_resp.headers().get("HX-Redirect").unwrap(), "/login");

        // Wrong password rejected with 401
        let bad_login = send(
            &app,
            post_form("/login", "username=admin&password=wrongpassword", None),
        )
        .await;
        assert_eq!(bad_login.status(), StatusCode::UNAUTHORIZED);

        // Valid login succeeds and honors next redirect
        let good_login = send(
            &app,
            post_form(
                "/login",
                "username=admin&password=adminpassword1&next=%2Fsettings",
                None,
            ),
        )
        .await;
        assert_eq!(good_login.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            good_login.headers().get(header::LOCATION).unwrap(),
            "/settings"
        );
        let new_cookie = extract_cookie_pair(&good_login);
        let settings_resp = send(&app, get_req("/settings", Some(&new_cookie))).await;
        assert_eq!(settings_resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn rbac_library_grants_and_admin_endpoints() {
        let (app, db) = test_app().await;
        let admin_cookie = bootstrap_admin(&app).await;

        // Create two libraries as admin
        let create_a = send(
            &app,
            post_form(
                "/libraries",
                "name=Movies&root_path=%2Fmedia%2Fmovies&kind=movie",
                Some(&admin_cookie),
            ),
        )
        .await;
        assert_eq!(create_a.status(), StatusCode::SEE_OTHER);

        let create_b = send(
            &app,
            post_form(
                "/libraries",
                "name=Private&root_path=%2Fmedia%2Fprivate&kind=movie",
                Some(&admin_cookie),
            ),
        )
        .await;
        assert_eq!(create_b.status(), StatusCode::SEE_OTHER);

        let libs = db.list_libraries().await.unwrap();
        let movies_lib = libs.iter().find(|l| l.name == "Movies").unwrap().clone();
        let private_lib = libs.iter().find(|l| l.name == "Private").unwrap().clone();

        // Admin creates standard user alice
        let create_alice = send(
            &app,
            post_form(
                "/settings/users",
                "username=alice&password=alicepassword1",
                Some(&admin_cookie),
            ),
        )
        .await;
        assert_eq!(create_alice.status(), StatusCode::SEE_OTHER);

        let (alice, _) = db.get_user_by_username("alice").await.unwrap().unwrap();
        assert!(!alice.is_admin);

        // Grant alice access to Movies (and verify repeated library_ids form parsing too)
        let grant_both = send(
            &app,
            post_form(
                &format!("/settings/users/{}/libraries", alice.id),
                &format!("library_ids={}&library_ids={}", movies_lib.id, private_lib.id),
                Some(&admin_cookie),
            ),
        )
        .await;
        assert_eq!(grant_both.status(), StatusCode::SEE_OTHER);
        assert_eq!(db.list_user_libraries(alice.id).await.unwrap().len(), 2);

        // Narrow alice access to only Movies
        let grant_movies = send(
            &app,
            post_form(
                &format!("/settings/users/{}/libraries", alice.id),
                &format!("library_ids={}", movies_lib.id),
                Some(&admin_cookie),
            ),
        )
        .await;
        assert_eq!(grant_movies.status(), StatusCode::SEE_OTHER);

        // Log in as alice
        let alice_login = send(
            &app,
            post_form("/login", "username=alice&password=alicepassword1", None),
        )
        .await;
        assert_eq!(alice_login.status(), StatusCode::SEE_OTHER);
        let alice_cookie = extract_cookie_pair(&alice_login);

        // Alice sees only Movies on /libraries
        let alice_libs_resp = send(&app, get_req("/libraries", Some(&alice_cookie))).await;
        assert_eq!(alice_libs_resp.status(), StatusCode::OK);
        let html = body_string(alice_libs_resp).await;
        assert!(html.contains("Movies"));
        assert!(!html.contains("Private"));
        assert!(!html.contains("+ Add Library"));

        // Alice can view granted library, denied on ungranted library
        let allowed_view = send(
            &app,
            get_req(&format!("/libraries/{}", movies_lib.id), Some(&alice_cookie)),
        )
        .await;
        assert_eq!(allowed_view.status(), StatusCode::OK);

        let denied_view = send(
            &app,
            get_req(
                &format!("/libraries/{}", private_lib.id),
                Some(&alice_cookie),
            ),
        )
        .await;
        assert_eq!(denied_view.status(), StatusCode::FORBIDDEN);

        // Alice forbidden from admin routes
        assert_eq!(
            send(&app, get_req("/settings", Some(&alice_cookie)))
                .await
                .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(
                &app,
                post_form(
                    "/libraries",
                    "name=Nope&root_path=%2Ftmp%2Fnope&kind=movie",
                    Some(&alice_cookie),
                ),
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(
                &app,
                post_form(
                    &format!("/libraries/{}/scan", movies_lib.id),
                    "",
                    Some(&alice_cookie),
                ),
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );

        // Admin resets alice's password -> alice's existing session is auto-invalidated
        // via `axum-login`'s `session_auth_hash()` comparison.
        let reset_pw = send(
            &app,
            post_form(
                &format!("/settings/users/{}/password", alice.id),
                "password=alicepassword2",
                Some(&admin_cookie),
            ),
        )
        .await;
        assert_eq!(reset_pw.status(), StatusCode::SEE_OTHER);

        let alice_after_reset = send(&app, get_req("/libraries", Some(&alice_cookie))).await;
        assert_eq!(alice_after_reset.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            alice_after_reset.headers().get(header::LOCATION).unwrap(),
            "/login"
        );

        // Empty library_ids clears access cleanly
        let clear_access = send(
            &app,
            post_form(
                &format!("/settings/users/{}/libraries", alice.id),
                "",
                Some(&admin_cookie),
            ),
        )
        .await;
        assert_eq!(clear_access.status(), StatusCode::SEE_OTHER);
        assert!(db.list_user_libraries(alice.id).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn password_change_invalidates_other_sessions_and_self_delete_guard() {
        let (app, db) = test_app().await;
        let admin_cookie = bootstrap_admin(&app).await;
        let (admin, _) = db.get_user_by_username("admin").await.unwrap().unwrap();

        // Second independent browser session for admin
        let second_login = send(
            &app,
            post_form("/login", "username=admin&password=adminpassword1", None),
        )
        .await;
        assert_eq!(second_login.status(), StatusCode::SEE_OTHER);
        let second_cookie = extract_cookie_pair(&second_login);

        // Admin cannot delete themselves
        let self_delete = send(
            &app,
            post_form(
                &format!("/settings/users/{}/delete", admin.id),
                "",
                Some(&admin_cookie),
            ),
        )
        .await;
        assert_eq!(self_delete.status(), StatusCode::BAD_REQUEST);
        assert_eq!(db.count_users().await.unwrap(), 1);

        // Admin changes own password via /account/password
        let change_pw = send(
            &app,
            post_form(
                "/account/password",
                "current_password=adminpassword1&new_password=newpassword99&confirm_password=newpassword99",
                Some(&admin_cookie),
            ),
        )
        .await;
        assert_eq!(change_pw.status(), StatusCode::OK);
        let rotated_cookie = extract_cookie_pair(&change_pw);

        // Old session ID and the second independent session are both rejected
        let old_session_check = send(&app, get_req("/libraries", Some(&admin_cookie))).await;
        assert_eq!(old_session_check.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            old_session_check.headers().get(header::LOCATION).unwrap(),
            "/login"
        );

        let second_session_check = send(&app, get_req("/libraries", Some(&second_cookie))).await;
        assert_eq!(second_session_check.status(), StatusCode::SEE_OTHER);
        assert_eq!(
            second_session_check
                .headers()
                .get(header::LOCATION)
                .unwrap(),
            "/login"
        );

        // Rotated session works
        assert_eq!(
            send(&app, get_req("/libraries", Some(&rotated_cookie)))
                .await
                .status(),
            StatusCode::OK
        );

        // Old password fails login, new password succeeds
        assert_eq!(
            send(
                &app,
                post_form("/login", "username=admin&password=adminpassword1", None),
            )
            .await
            .status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            send(
                &app,
                post_form("/login", "username=admin&password=newpassword99", None),
            )
            .await
            .status(),
            StatusCode::SEE_OTHER
        );
    }

    #[tokio::test]
    async fn library_edit_and_delete_permissions_and_cascade() {
        let (app, db) = test_app().await;
        let admin_cookie = bootstrap_admin(&app).await;

        let create_resp = send(
            &app,
            post_form(
                "/libraries",
                "name=Movies&root_path=%2Fmedia%2Fmovies&kind=movie",
                Some(&admin_cookie),
            ),
        )
        .await;
        assert_eq!(create_resp.status(), StatusCode::SEE_OTHER);

        let lib = db.list_libraries().await.unwrap().pop().unwrap();

        // Create standard user alice and verify she cannot edit or delete libraries
        assert_eq!(
            send(
                &app,
                post_form(
                    "/settings/users",
                    "username=alice&password=alicepassword1",
                    Some(&admin_cookie),
                ),
            )
            .await
            .status(),
            StatusCode::SEE_OTHER
        );
        let alice_login = send(
            &app,
            post_form("/login", "username=alice&password=alicepassword1", None),
        )
        .await;
        let alice_cookie = extract_cookie_pair(&alice_login);

        assert_eq!(
            send(
                &app,
                post_form(
                    &format!("/libraries/{}/edit", lib.id),
                    "name=Hacked&root_path=%2Ftmp&kind=movie",
                    Some(&alice_cookie),
                ),
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            send(
                &app,
                post_form(
                    &format!("/libraries/{}/delete", lib.id),
                    "",
                    Some(&alice_cookie),
                ),
            )
            .await
            .status(),
            StatusCode::FORBIDDEN
        );

        // Admin edits library name and path
        let edit_resp = send(
            &app,
            post_form(
                &format!("/libraries/{}/edit", lib.id),
                "name=4K+Movies&root_path=%2Fmedia%2F4k&kind=movie",
                Some(&admin_cookie),
            ),
        )
        .await;
        assert_eq!(edit_resp.status(), StatusCode::SEE_OTHER);
        let updated = db.get_library(lib.id).await.unwrap().unwrap();
        assert_eq!(updated.name, "4K Movies");
        assert_eq!(updated.root_path, "/media/4k");

        // Populate library with a catalog entity and media asset, then delete library
        let now = Utc::now();
        let entity = Entity {
            id: new_id(),
            kind: EntityKind::Movie,
            library_id: Some(lib.id),
            title: "Arrival".to_owned(),
            original_title: None,
            original_language: None,
            monitored: true,
            external_ids: std::collections::BTreeMap::new(),
            metadata: oxarr_types::EntityMetadata::default(),
            details: EntityDetails::Movie(MovieDetails {
                year: Some(2016),
                ..Default::default()
            }),
            created_at: now,
            updated_at: now,
        };
        db.insert_entity(&entity).await.unwrap();

        let asset = MediaAsset {
            id: new_id(),
            entity_ids: vec![entity.id],
            path: "/media/4k/Arrival.2016.mkv".to_owned(),
            size_bytes: 5_000_000,
            hash_xxh3: None,
            matched_classifier_ids: Vec::new(),
            quality_score: None,
            media_info: serde_json::json!({}),
            imported_at: now,
        };
        db.insert_media_asset(&asset).await.unwrap();

        let delete_resp = send(
            &app,
            post_form(
                &format!("/libraries/{}/delete", lib.id),
                "",
                Some(&admin_cookie),
            ),
        )
        .await;
        assert_eq!(delete_resp.status(), StatusCode::SEE_OTHER);
        assert!(db.get_library(lib.id).await.unwrap().is_none());
        assert!(db.get_entity(entity.id).await.unwrap().is_none());
        assert!(db.get_media_asset(asset.id).await.unwrap().is_none());
    }
}
