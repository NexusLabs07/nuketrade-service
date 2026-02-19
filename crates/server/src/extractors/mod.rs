use axum::{
    extract::{FromRequest, FromRequestParts, Json, Path, Query, Request},
    http::request::Parts,
};
use serde::de::DeserializeOwned;
use validator::Validate;

use crate::error::AppError;

pub struct ValidatedJson<T>(pub T);
pub struct ValidatedPath<T>(pub T);
pub struct ValidatedQuery<T>(pub T);

impl<S, T> FromRequest<S> for ValidatedJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state)
            .await
            .map_err(|e| AppError::parse("body", e.to_string()))?;

        value.validate()?;
        Ok(Self(value))
    }
}

impl<S, T> FromRequestParts<S> for ValidatedPath<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate + Send,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(value) = Path::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| AppError::parse("path", e.to_string()))?;

        value.validate()?;
        Ok(Self(value))
    }
}

impl<S, T> FromRequestParts<S> for ValidatedQuery<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate + Send,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Query(value) = Query::<T>::from_request_parts(parts, state)
            .await
            .map_err(|e| AppError::parse("query", e.to_string()))?;

        value.validate()?;
        Ok(Self(value))
    }
}
