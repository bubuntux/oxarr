use std::collections::HashSet;
use std::fmt;

use async_trait::async_trait;
use axum::extract::{FromRequestParts, Request, State};
use axum::http::request::Parts;
use axum::http::{HeaderMap, Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect, Response};
use axum_login::{
    AuthManagerLayer, AuthManagerLayerBuilder, AuthSession, AuthUser, AuthnBackend,
    AuthzBackend, UserId,
};
use oxarr_core::Database;
use oxarr_types::User;
use time::OffsetDateTime;
use tower_sessions::cookie::SameSite;
use tower_sessions::session::{Id, Record};
use tower_sessions::session_store::{self, ExpiredDeletion, SessionStore};
use tower_sessions::{Expiry, SessionManagerLayer};
use uuid::Uuid;

use crate::AppState;

pub const SESSION_COOKIE_NAME: &str = "oxarr_session";
pub const SESSION_INACTIVITY_DAYS: i64 = 30;

pub type OxarrAuthSession = AuthSession<OxarrAuthBackend>;

/// SeaORM-backed `tower-sessions` store persisting serialized session records in SQLite.
#[derive(Clone)]
pub struct SeaOrmSessionStore {
    db: Database,
}

impl fmt::Debug for SeaOrmSessionStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SeaOrmSessionStore").finish_non_exhaustive()
    }
}

impl SeaOrmSessionStore {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

#[async_trait]
impl SessionStore for SeaOrmSessionStore {
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        for _ in 0..8 {
            let data = serde_json::to_string(record)
                .map_err(|err| session_store::Error::Encode(err.to_string()))?;
            let expiry_unix = record.expiry_date.unix_timestamp();
            let inserted = self
                .db
                .insert_session(&record.id.to_string(), &data, expiry_unix)
                .await
                .map_err(|err| session_store::Error::Backend(err.to_string()))?;
            if inserted {
                return Ok(());
            }
            record.id = Id::default();
        }

        Err(session_store::Error::Backend(
            "failed to allocate unique session id".to_owned(),
        ))
    }

    async fn save(&self, record: &Record) -> session_store::Result<()> {
        let data = serde_json::to_string(record)
            .map_err(|err| session_store::Error::Encode(err.to_string()))?;
        let expiry_unix = record.expiry_date.unix_timestamp();
        self.db
            .upsert_session(&record.id.to_string(), &data, expiry_unix)
            .await
            .map_err(|err| session_store::Error::Backend(err.to_string()))
    }

    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let now_unix = OffsetDateTime::now_utc().unix_timestamp();
        let Some(raw) = self
            .db
            .get_session(&session_id.to_string(), now_unix)
            .await
            .map_err(|err| session_store::Error::Backend(err.to_string()))?
        else {
            return Ok(None);
        };

        let record: Record = serde_json::from_str(&raw)
            .map_err(|err| session_store::Error::Decode(err.to_string()))?;
        Ok(Some(record))
    }

    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        self.db
            .delete_session(&session_id.to_string())
            .await
            .map_err(|err| session_store::Error::Backend(err.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl ExpiredDeletion for SeaOrmSessionStore {
    async fn delete_expired(&self) -> session_store::Result<()> {
        let now_unix = OffsetDateTime::now_utc().unix_timestamp();
        self.db
            .delete_expired_sessions(now_unix)
            .await
            .map_err(|err| session_store::Error::Backend(err.to_string()))?;
        Ok(())
    }
}

/// Authenticated user record paired with its stored Argon2id hash for `axum-login`
/// session validation (`session_auth_hash`).
#[derive(Clone)]
pub struct AuthAccount {
    pub user: User,
    pub password_hash: String,
}

impl fmt::Debug for AuthAccount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthAccount")
            .field("user", &self.user)
            .field("password_hash", &"[REDACTED]")
            .finish()
    }
}

impl AuthUser for AuthAccount {
    type Id = Uuid;

    fn id(&self) -> Self::Id {
        self.user.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.password_hash.as_bytes()
    }
}

#[derive(Clone)]
pub struct Credentials {
    pub username: String,
    pub password: String,
}

impl fmt::Debug for Credentials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &self.username)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Permission {
    Admin,
}

#[derive(Debug)]
pub struct AuthBackendError(pub String);

impl fmt::Display for AuthBackendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AuthBackendError {}

impl From<anyhow::Error> for AuthBackendError {
    fn from(err: anyhow::Error) -> Self {
        Self(err.to_string())
    }
}

/// `axum-login` backend delegating user persistence and Argon2id verification to `oxarr-core`.
#[derive(Clone)]
pub struct OxarrAuthBackend {
    db: Database,
}

impl fmt::Debug for OxarrAuthBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OxarrAuthBackend").finish_non_exhaustive()
    }
}

impl OxarrAuthBackend {
    pub fn new(db: Database) -> Self {
        Self { db }
    }
}

impl AuthnBackend for OxarrAuthBackend {
    type User = AuthAccount;
    type Credentials = Credentials;
    type Error = AuthBackendError;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let username = creds.username.trim();
        let Some((user, password_hash)) = self.db.get_user_by_username(username).await? else {
            return Ok(None);
        };

        let valid = verify_password_blocking(creds.password, password_hash.clone()).await?;
        if !valid {
            return Ok(None);
        }

        Ok(Some(AuthAccount {
            user,
            password_hash,
        }))
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        let Some((user, password_hash)) = self.db.get_user_with_password_hash(*user_id).await?
        else {
            return Ok(None);
        };

        Ok(Some(AuthAccount {
            user,
            password_hash,
        }))
    }
}

impl AuthzBackend for OxarrAuthBackend {
    type Permission = Permission;

    async fn get_user_permissions(
        &self,
        user: &Self::User,
    ) -> Result<HashSet<Self::Permission>, Self::Error> {
        let mut perms = HashSet::new();
        if user.user.is_admin {
            perms.insert(Permission::Admin);
        }
        Ok(perms)
    }
}

/// Authenticated user extractor backed by `axum-login`'s [`AuthSession`].
#[derive(Clone, Debug)]
pub struct CurrentUser(pub User);

impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_session = OxarrAuthSession::from_request_parts(parts, state).await?;
        auth_session
            .user
            .map(|account| CurrentUser(account.user))
            .ok_or((StatusCode::UNAUTHORIZED, "Authentication required"))
    }
}

/// Authenticated administrator extractor backed by [`AuthzBackend::has_perm`].
#[derive(Clone, Debug)]
pub struct AdminUser(pub User);

impl<S: Send + Sync> FromRequestParts<S> for AdminUser {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let auth_session = OxarrAuthSession::from_request_parts(parts, state).await?;
        let Some(account) = auth_session.user else {
            return Err((StatusCode::UNAUTHORIZED, "Authentication required"));
        };
        let is_admin = auth_session
            .backend
            .has_perm(&account, Permission::Admin)
            .await
            .map_err(|_| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Authorization check failed",
                )
            })?;
        if !is_admin {
            return Err((StatusCode::FORBIDDEN, "Administrator access required"));
        }
        Ok(AdminUser(account.user))
    }
}

/// Builds the combined `tower-sessions` + `axum-login` layer and returns its underlying
/// [`SeaOrmSessionStore`] so callers can schedule expired-session cleanup.
pub fn build_auth_layer(
    db: Database,
) -> (
    AuthManagerLayer<OxarrAuthBackend, SeaOrmSessionStore>,
    SeaOrmSessionStore,
) {
    let store = SeaOrmSessionStore::new(db.clone());
    let session_layer = SessionManagerLayer::new(store.clone())
        .with_name(SESSION_COOKIE_NAME)
        .with_http_only(true)
        .with_same_site(SameSite::Lax)
        .with_secure(should_use_secure_cookie())
        .with_expiry(Expiry::OnInactivity(time::Duration::days(
            SESSION_INACTIVITY_DAYS,
        )));

    let auth_layer = AuthManagerLayerBuilder::new(OxarrAuthBackend::new(db), session_layer).build();
    (auth_layer, store)
}

/// Determines whether session cookies carry the `Secure` flag (`OXARR_COOKIE_SECURE=1|true`).
/// Defaults to `false` so local LAN HTTP access works out of the box.
pub fn should_use_secure_cookie() -> bool {
    std::env::var("OXARR_COOKIE_SECURE")
        .ok()
        .is_some_and(|val| val == "1" || val.eq_ignore_ascii_case("true"))
}

pub async fn hash_password_blocking(password: String) -> anyhow::Result<String> {
    tokio::task::spawn_blocking(move || oxarr_core::auth::hash_password(&password)).await?
}

pub async fn verify_password_blocking(password: String, hash: String) -> anyhow::Result<bool> {
    Ok(
        tokio::task::spawn_blocking(move || oxarr_core::auth::verify_password(&password, &hash))
            .await?,
    )
}

/// Restricts post-login redirects to local relative paths to prevent open redirects.
pub fn sanitize_next_url(next: Option<&str>) -> &str {
    match next {
        Some(path)
            if path.starts_with('/')
                && !path.starts_with("//")
                && !path.contains('\r')
                && !path.contains('\n') =>
        {
            path
        }
        _ => "/libraries",
    }
}

pub fn is_htmx_request(headers: &HeaderMap) -> bool {
    headers.get("hx-request").and_then(|v| v.to_str().ok()) == Some("true")
}

/// Redirects unauthenticated callers, using `HX-Redirect` for htmx partial requests so
/// full-page navigation happens instead of swapping the login page into a DOM fragment.
pub fn redirect_unauthenticated(headers: &HeaderMap, target: &str) -> Response {
    if is_htmx_request(headers) {
        (StatusCode::OK, [("HX-Redirect", target)], "").into_response()
    } else {
        Redirect::to(target).into_response()
    }
}

/// Axum middleware enforcing an active `axum-login` session on protected routes,
/// redirecting to `/setup` on first run or `/login` otherwise.
pub(crate) async fn require_auth(
    State(state): State<AppState>,
    auth_session: OxarrAuthSession,
    req: Request,
    next: Next,
) -> Response {
    if auth_session.user.is_some() {
        return next.run(req).await;
    }

    let has_users = state.db.count_users().await.unwrap_or(0) > 0;
    if !has_users {
        return redirect_unauthenticated(req.headers(), "/setup");
    }

    let target = if !is_htmx_request(req.headers()) && req.method() == Method::GET {
        let path_and_query = req
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/libraries");
        if path_and_query == "/" || path_and_query == "/libraries" {
            "/login".to_owned()
        } else {
            format!("/login?next={}", encode_query_component(path_and_query))
        }
    } else {
        "/login".to_owned()
    };

    redirect_unauthenticated(req.headers(), &target)
}

/// Parses repeated `library_ids=<uuid>` checkbox pairs from an `x-www-form-urlencoded` body.
/// Raw body parsing is used because `serde_urlencoded` rejects repeated keys into `Vec<Uuid>`.
pub fn parse_library_ids_form(body: &str) -> Result<Vec<Uuid>, uuid::Error> {
    let mut ids = Vec::new();
    for pair in body.split('&') {
        if pair.is_empty() {
            continue;
        }
        let Some((key, raw_val)) = pair.split_once('=') else {
            continue;
        };
        if key == "library_ids" {
            let trimmed = raw_val.trim();
            if !trimmed.is_empty() {
                ids.push(Uuid::parse_str(trimmed)?);
            }
        }
    }
    Ok(ids)
}

pub fn encode_query_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            }
            _ => {
                out.push('%');
                out.push(hex_nibble(byte >> 4));
                out.push(hex_nibble(byte & 0x0F));
            }
        }
    }
    out
}

fn hex_nibble(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'A' + (n - 10)) as char,
        _ => '0',
    }
}
