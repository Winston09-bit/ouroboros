// =============================================================
// src/auth/clerk.rs – Clerk JWT middleware for Axum
// =============================================================

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tracing::{debug, warn};

// ─────────────────────────────────────────────────────────────
// Error types
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Missing Authorization header")]
    MissingToken,

    #[error("Invalid token format")]
    InvalidToken,

    #[error("Token has expired")]
    ExpiredToken,

    #[error("Insufficient permissions for this resource")]
    InsufficientPermissions,

    #[error("JWT decode error: {0}")]
    JwtDecodeError(#[from] jsonwebtoken::errors::Error),

    #[error("Environment variable not set: {0}")]
    EnvError(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AuthError::MissingToken => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::ExpiredToken => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::InsufficientPermissions => (StatusCode::FORBIDDEN, self.to_string()),
            AuthError::JwtDecodeError(_) => (StatusCode::UNAUTHORIZED, "Invalid token".into()),
            AuthError::EnvError(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Auth configuration error".into(),
            ),
        };

        (
            status,
            Json(json!({
                "error": message,
                "code": status.as_u16()
            })),
        )
            .into_response()
    }
}

// ─────────────────────────────────────────────────────────────
// JWT Claims – Clerk's token structure
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Clone)]
pub struct ClerkClaims {
    /// Subject = user_id
    pub sub: String,
    /// Issued at
    pub iat: i64,
    /// Expiry
    pub exp: i64,
    /// Clerk session ID
    pub sid: Option<String>,
    /// Active organization ID
    pub org_id: Option<String>,
    /// Organization role (e.g. "org:admin")
    pub org_role: Option<String>,
    /// Organization slug
    pub org_slug: Option<String>,
    /// Primary email address
    pub email: Option<String>,
    /// First name
    pub first_name: Option<String>,
    /// Last name
    pub last_name: Option<String>,
    /// Custom metadata set on the user in Clerk dashboard
    pub metadata: Option<serde_json::Value>,
}

// ─────────────────────────────────────────────────────────────
// Authenticated user – injected as Axum extension
// ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub email: String,
    pub org_id: Option<String>,
    pub org_role: Option<String>,
    pub roles: Vec<String>,
    pub session_id: Option<String>,
}

impl AuthenticatedUser {
    /// Check if the user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }

    /// Check if the user is an org admin
    pub fn is_org_admin(&self) -> bool {
        self.org_role
            .as_deref()
            .map(|r| r == "org:admin")
            .unwrap_or(false)
    }

    /// Check if the user belongs to the specified org
    pub fn belongs_to_org(&self, org_id: &str) -> bool {
        self.org_id.as_deref() == Some(org_id)
    }
}

impl From<ClerkClaims> for AuthenticatedUser {
    fn from(claims: ClerkClaims) -> Self {
        // Build roles list from org_role and any custom metadata roles
        let mut roles: Vec<String> = Vec::new();
        if let Some(ref role) = claims.org_role {
            roles.push(role.clone());
        }
        if let Some(ref meta) = claims.metadata {
            if let Some(extra_roles) = meta.get("roles").and_then(|v| v.as_array()) {
                roles.extend(extra_roles.iter().filter_map(|r| r.as_str().map(String::from)));
            }
        }

        AuthenticatedUser {
            user_id: claims.sub,
            email: claims.email.unwrap_or_default(),
            org_id: claims.org_id,
            org_role: claims.org_role,
            roles,
            session_id: claims.sid,
        }
    }
}

// ─────────────────────────────────────────────────────────────
// ClerkAuth – main auth state
// ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ClerkAuth {
    /// Clerk publishable key (PK_*)
    publishable_key: String,
    /// Clerk secret key (SK_*) – used for JWKS endpoint auth
    secret_key: String,
    /// PEM-encoded JWT verification key (from Clerk dashboard > API Keys > JWT key)
    jwt_decoding_key: DecodingKey,
    /// JWT validation settings
    validation: Validation,
}

impl ClerkAuth {
    /// Construct from environment variables.
    ///
    /// Required env vars:
    /// - `CLERK_SECRET_KEY`      — sk_live_* or sk_test_*
    /// - `CLERK_PUBLISHABLE_KEY` — pk_live_* or pk_test_*
    /// - `CLERK_JWT_KEY`         — PEM public key from Clerk dashboard
    pub fn new() -> Result<Self, AuthError> {
        let secret_key = std::env::var("CLERK_SECRET_KEY")
            .map_err(|_| AuthError::EnvError("CLERK_SECRET_KEY".into()))?;

        let publishable_key = std::env::var("CLERK_PUBLISHABLE_KEY")
            .map_err(|_| AuthError::EnvError("CLERK_PUBLISHABLE_KEY".into()))?;

        let jwt_key_pem = std::env::var("CLERK_JWT_KEY")
            .map_err(|_| AuthError::EnvError("CLERK_JWT_KEY".into()))?;

        // Clerk exports keys as PEM; decode accordingly
        let jwt_decoding_key = DecodingKey::from_rsa_pem(jwt_key_pem.as_bytes())
            .map_err(|e| {
                warn!("Failed to parse CLERK_JWT_KEY as RSA PEM: {}", e);
                AuthError::InvalidToken
            })?;

        let mut validation = Validation::new(Algorithm::RS256);
        // Clerk tokens don't set 'aud' by default; disable audience check
        validation.validate_aud = false;

        Ok(Self {
            publishable_key,
            secret_key,
            jwt_decoding_key,
            validation,
        })
    }

    /// Extract and validate a JWT, returning typed claims.
    pub fn verify_token(&self, token: &str) -> Result<ClerkClaims, AuthError> {
        let token_data =
            decode::<ClerkClaims>(token, &self.jwt_decoding_key, &self.validation)?;
        Ok(token_data.claims)
    }

    /// Extract user_id (sub) from token without full validation.
    /// Useful for logging. Prefer `verify_token` for auth decisions.
    pub fn extract_user_id(token: &str) -> Result<String, AuthError> {
        // Decode header first to get algorithm
        let _header = decode_header(token).map_err(|_| AuthError::InvalidToken)?;

        // Decode without validation just to read sub claim
        let mut insecure = Validation::new(Algorithm::RS256);
        insecure.insecure_disable_signature_validation();
        insecure.validate_exp = false;
        insecure.validate_aud = false;

        // We still need a key object even for insecure decode
        let dummy_key = DecodingKey::from_secret(b"dummy");
        let data = decode::<ClerkClaims>(token, &dummy_key, &insecure)
            .map_err(|_| AuthError::InvalidToken)?;

        Ok(data.claims.sub)
    }

    /// Verify that a user (identified by user_id) belongs to the given org.
    /// This checks the claim embedded in the JWT — for real-time membership
    /// verification you would additionally query Clerk's BAPI.
    pub fn verify_org_access(claims: &ClerkClaims, org_id: &str) -> bool {
        claims.org_id.as_deref() == Some(org_id)
    }
}

// ─────────────────────────────────────────────────────────────
// Helper – extract bearer token from headers
// ─────────────────────────────────────────────────────────────

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}

// ─────────────────────────────────────────────────────────────
// Axum middleware
// ─────────────────────────────────────────────────────────────

/// Axum middleware that verifies the Clerk JWT and injects
/// `AuthenticatedUser` as a request extension.
///
/// Usage:
/// ```rust
/// let clerk = Arc::new(ClerkAuth::new().expect("Clerk auth init failed"));
/// let app = Router::new()
///     .route("/protected", get(handler))
///     .layer(axum::middleware::from_fn_with_state(
///         clerk.clone(),
///         clerk_auth_middleware,
///     ));
/// ```
pub async fn clerk_auth_middleware(
    State(auth): State<Arc<ClerkAuth>>,
    mut req: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let token = extract_bearer_token(req.headers())
        .ok_or(AuthError::MissingToken)?;

    debug!("Verifying Clerk JWT...");

    let claims = auth.verify_token(token).map_err(|e| {
        warn!("JWT verification failed: {:?}", e);
        match e {
            AuthError::JwtDecodeError(ref inner) => {
                use jsonwebtoken::errors::ErrorKind;
                match inner.kind() {
                    ErrorKind::ExpiredSignature => AuthError::ExpiredToken,
                    _ => AuthError::InvalidToken,
                }
            }
            other => other,
        }
    })?;

    debug!("Authenticated user: {}", claims.sub);

    let user: AuthenticatedUser = claims.into();
    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}

/// Middleware variant that additionally requires the user to belong
/// to a specific organisation.
pub async fn require_org_middleware(
    State((auth, required_org_id)): State<(Arc<ClerkAuth>, String)>,
    mut req: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let token = extract_bearer_token(req.headers())
        .ok_or(AuthError::MissingToken)?;

    let claims = auth.verify_token(token).map_err(|e| {
        warn!("JWT verification failed: {:?}", e);
        AuthError::InvalidToken
    })?;

    if !ClerkAuth::verify_org_access(&claims, &required_org_id) {
        warn!(
            "User {} denied: not a member of org {}",
            claims.sub, required_org_id
        );
        return Err(AuthError::InsufficientPermissions);
    }

    let user: AuthenticatedUser = claims.into();
    req.extensions_mut().insert(user);

    Ok(next.run(req).await)
}

// ─────────────────────────────────────────────────────────────
// Axum extractor – pulls AuthenticatedUser from extensions
// ─────────────────────────────────────────────────────────────

use axum::extract::FromRequestParts;
use axum::http::request::Parts;

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<AuthenticatedUser>()
            .cloned()
            .ok_or(AuthError::MissingToken)
    }
}

// ─────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_error_missing_token() {
        let err = AuthError::MissingToken;
        assert_eq!(err.to_string(), "Missing Authorization header");
    }

    #[test]
    fn test_auth_error_expired_token() {
        let err = AuthError::ExpiredToken;
        assert_eq!(err.to_string(), "Token has expired");
    }

    #[test]
    fn test_authenticated_user_has_role() {
        let user = AuthenticatedUser {
            user_id: "user_123".into(),
            email: "test@example.com".into(),
            org_id: Some("org_456".into()),
            org_role: Some("org:admin".into()),
            roles: vec!["org:admin".into(), "financial:read".into()],
            session_id: None,
        };

        assert!(user.has_role("org:admin"));
        assert!(user.has_role("financial:read"));
        assert!(!user.has_role("superadmin"));
        assert!(user.is_org_admin());
        assert!(user.belongs_to_org("org_456"));
        assert!(!user.belongs_to_org("org_999"));
    }

    #[test]
    fn test_verify_org_access_match() {
        let claims = ClerkClaims {
            sub: "user_abc".into(),
            iat: 0,
            exp: 9999999999,
            sid: None,
            org_id: Some("org_target".into()),
            org_role: None,
            org_slug: None,
            email: None,
            first_name: None,
            last_name: None,
            metadata: None,
        };

        assert!(ClerkAuth::verify_org_access(&claims, "org_target"));
        assert!(!ClerkAuth::verify_org_access(&claims, "org_other"));
    }

    #[test]
    fn test_verify_org_access_no_org() {
        let claims = ClerkClaims {
            sub: "user_abc".into(),
            iat: 0,
            exp: 9999999999,
            sid: None,
            org_id: None,
            org_role: None,
            org_slug: None,
            email: None,
            first_name: None,
            last_name: None,
            metadata: None,
        };

        assert!(!ClerkAuth::verify_org_access(&claims, "org_target"));
    }
}
