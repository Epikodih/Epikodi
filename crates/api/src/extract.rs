//! Extracteurs qui renvoient nos erreurs JSON (et pas le texte brut d'axum).

use axum::async_trait;
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use serde::de::DeserializeOwned;

use crate::error::{json_rejection, ApiError};

/// Corps JSON : erreur `validation_failed` s'il est invalide.
pub struct JsonBody<T>(pub T);

#[async_trait]
impl<T, S> FromRequest<S> for JsonBody<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        axum::Json::<T>::from_request(req, state)
            .await
            .map(|axum::Json(v)| JsonBody(v))
            .map_err(json_rejection)
    }
}

/// Paramètres d'URL : erreur `invalid_query` s'ils sont invalides.
pub struct QueryParams<T>(pub T);

#[async_trait]
impl<T, S> FromRequestParts<S> for QueryParams<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Query(v)| QueryParams(v))
            .map_err(|e| {
                let text = e.body_text();
                let field = text
                    .split('`')
                    .nth(1)
                    .filter(|f| !f.contains(' '))
                    .unwrap_or("query")
                    .to_string();
                ApiError::invalid_query(&field, text)
            })
    }
}

/// Identifiant dans le chemin (`/api/media/{id}`) : 404 s'il est invalide.
pub struct PathId(pub i64);

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for PathId {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let axum::extract::Path(raw): axum::extract::Path<String> =
            axum::extract::Path::from_request_parts(parts, state)
                .await
                .map_err(|_| ApiError::not_found("not_found", "Ressource introuvable."))?;
        raw.parse()
            .map(PathId)
            .map_err(|_| ApiError::not_found("not_found", "Identifiant invalide."))
    }
}
