//! Erreurs de l'API : toujours `{"error": {"code", "message", "details"}}`.
//! Les apps se basent sur `code`, jamais sur `message`.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

use epikodi_domain::protocol::{ErrorBody, ErrorResponse};
use epikodi_domain::DomainError;
use epikodi_extensions::ExtensionError;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Clone, PartialEq)]
pub struct ApiError {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
    pub details: Option<Value>,
}

impl ApiError {
    pub fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Appareil non appairé ou session expirée.",
        )
    }

    pub fn localhost_only() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "localhost_only",
            "Route réservée à l'app TV (accès local).",
        )
    }

    pub fn profile_required() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "profile_required",
            "Choisissez d'abord un profil.",
        )
    }

    pub fn guest_forbidden() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "guest_forbidden",
            "Action impossible en mode invité.",
        )
    }

    pub fn admin_required() -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            "admin_required",
            "Action réservée à un compte non restreint.",
        )
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    pub fn not_found(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, code, message)
    }

    pub fn conflict(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }

    /// Champ invalide : `details.fields = {"champ": "raison"}`.
    pub fn validation(field: &str, reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "validation_failed",
            format!("Champ invalide : {field} ({reason})."),
        )
        .with_details(json!({ "fields": { field: reason } }))
    }

    pub fn invalid_query(field: &str, reason: impl Into<String>) -> Self {
        let reason = reason.into();
        Self::new(
            StatusCode::UNPROCESSABLE_ENTITY,
            "invalid_query",
            format!("Paramètre invalide : {field} ({reason})."),
        )
        .with_details(json!({ "fields": { field: reason } }))
    }

    pub fn internal(e: impl std::fmt::Display) -> Self {
        tracing::error!("erreur interne : {e}");
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Erreur interne du serveur.",
        )
    }
}

impl From<DomainError> for ApiError {
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::Validation { field, reason } => Self::validation(&field, reason),
            DomainError::NotFound { entity } => {
                let code = match entity {
                    "profil" => "profile_not_found",
                    "média" | "parent" => "media_not_found",
                    "source" => "source_not_found",
                    "extension" => "extension_not_found",
                    "appareil" => "device_not_found",
                    "liste de lecture" => "playlist_not_found",
                    _ => "not_found",
                };
                Self::not_found(code, format!("{} introuvable.", capitalize(entity)))
            }
            DomainError::Conflict { entity } => {
                let code = match entity {
                    "profil" => "profile_name_taken",
                    "source" => "source_exists",
                    "liste de lecture" => "playlist_name_taken",
                    _ => "conflict",
                };
                Self::conflict(code, format!("{} existe déjà.", capitalize(entity)))
            }
            DomainError::Rule(msg) => Self::new(StatusCode::CONFLICT, "rule_violation", msg),
            DomainError::Internal(msg) => Self::internal(msg),
        }
    }
}

impl From<ExtensionError> for ApiError {
    fn from(e: ExtensionError) -> Self {
        match e {
            ExtensionError::NotFound(_) => Self::not_found("extension_not_found", e.to_string()),
            ExtensionError::AlreadyInstalled(_) => {
                Self::conflict("extension_already_installed", e.to_string())
            }
            ExtensionError::NotNewer { .. } => Self::conflict("extension_not_newer", e.to_string()),
            ExtensionError::InvalidManifest(_) | ExtensionError::InvalidPackage(_) => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "extension_invalid",
                e.to_string(),
            ),
            ExtensionError::InvalidSetting(m) => Self::validation("settings", m),
            ExtensionError::PermissionDenied(_) => Self::forbidden(e.to_string()),
            ExtensionError::Domain(d) => d.into(),
            ExtensionError::Io(io) => Self::internal(io),
        }
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = ErrorResponse {
            error: ErrorBody {
                code: self.code.to_string(),
                message: self.message,
                details: self.details,
            },
        };
        (self.status, Json(body)).into_response()
    }
}

/// Corps JSON illisible → `validation_failed` (et pas le texte brut d'axum).
pub fn json_rejection(e: axum::extract::rejection::JsonRejection) -> ApiError {
    use axum::extract::rejection::JsonRejection::*;
    match e {
        MissingJsonContentType(_) => {
            Self_::validation("body", "Content-Type: application/json attendu")
        }
        JsonSyntaxError(_) => Self_::validation("body", "JSON invalide"),
        JsonDataError(d) => {
            let text = d.body_text();
            let field = text
                .split('`')
                .nth(1)
                .filter(|f| !f.contains(' '))
                .unwrap_or("body")
                .to_string();
            Self_::validation(&field, text)
        }
        other => Self_::validation("body", other.body_text()),
    }
}

type Self_ = ApiError;
