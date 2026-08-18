use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use ic_agent::export::PrincipalError;
use std::env::VarError;
use thiserror::Error;
use types::{error::ApiError, ApiResult};
use utoipa::ToSchema;

use crate::services::error_wrappers::{
    AgentErrorDetail, ConfigErrorDetail, IOErrorData, IdentityErrorDetail,
    JwtErrorDetail, PrincipalErrorDetail, SerdeJsonErrorDetail, VarErrorDetail,
};

#[derive(Error, Debug, ToSchema)]
pub enum Error {
    #[error(transparent)]
    #[schema(value_type = IOErrorData)]
    IO(#[from] std::io::Error),
    #[error("failed to load config {0}")]
    #[schema(value_type = ConfigErrorDetail)]
    Config(#[from] config::ConfigError),
    #[error("{0}")]
    #[schema(value_type = IdentityErrorDetail)]
    Identity(#[from] identity::Error),
    #[error("failed to deserialize json {0}")]
    #[schema(value_type = SerdeJsonErrorDetail)]
    Deser(#[from] serde_json::Error),
    #[error("jwt {0}")]
    #[schema(value_type = JwtErrorDetail)]
    Jwt(#[from] jsonwebtoken::errors::Error),
    #[error("auth token missing")]
    AuthTokenMissing,
    #[error("auth token invalid")]
    AuthTokenInvalid,
    #[error("unknown error {0}")]
    Unknown(String),
    #[error("Environment variable error: {0}")]
    #[schema(value_type = VarErrorDetail)]
    EnvironmentVariable(#[from] VarError),
    #[error("Environment variable missing: {0}")]
    EnvironmentVariableMissing(String),
    #[error("failed to mark user sessin as registered")]
    UserAlreadyRegistered(String),
    #[error("failed to initialize backend admin ic agent")]
    BackendAdminIdentityInvalid(String),
    #[error("failed to parse principal {0}")]
    #[schema(value_type = PrincipalErrorDetail)]
    InvalidPrincipal(#[from] PrincipalError),
    #[error("failed to communicate with IC: {0}")]
    #[schema(value_type = AgentErrorDetail)]
    Agent(#[from] ic_agent::AgentError),
    #[error("failed to update session: {0}")]
    UpdateSession(String),
    #[error("swagger ui error {0}")]
    SwaggerUi(String),
    #[error("invalid username, must be 3-15 alphanumeric characters")]
    InvalidUsername,
    #[error("duplicate username")]
    DuplicateUsername,
    #[error("Invalid email")]
    InvalidEmail(String),
}

impl From<&Error> for ApiResult<()> {
    fn from(value: &Error) -> Self {
        let err = match value {
            Error::IO(_) | Error::Config(_) => {
                log::warn!("internal error {value}");
                ApiError::Unknown("internal error, reported".into())
            }
            Error::Identity(_) => {
                ApiError::InvalidSignature
            }
            Error::Deser(e) => {
                log::warn!("deserialization error {e}");
                ApiError::Deser
            }
            Error::Jwt(_) => {
                ApiError::Jwt
            }
            Error::AuthTokenMissing => {
                ApiError::AuthTokenMissing
            }
            Error::AuthTokenInvalid => {
                ApiError::AuthToken
            }
            Error::BackendAdminIdentityInvalid(e) => {
                log::error!("Backend admin identity invalid: {e}");
                ApiError::BackendAdminIdentityInvalid(e.clone())
            }
            Error::Unknown(e) => {
                log::error!("Unknown error: {e}");
                ApiError::Unknown(e.clone())
            }
            Error::EnvironmentVariable(_) => {
                log::error!("Environment variable error");
                ApiError::EnvironmentVariable
            }
            Error::EnvironmentVariableMissing(_) => {
                log::error!("Environment variable missing");
                ApiError::EnvironmentVariableMissing
            }
            Error::UserAlreadyRegistered(e) => {
                log::info!("User already registered: {e}");
                ApiError::UserAlreadyRegistered(e.clone())
            }
            Error::InvalidPrincipal(_) => {
                log::warn!("Invalid principal");
                ApiError::InvalidPrincipal
            }
            Error::Agent(e) => {
                log::warn!("agent error {e}");
                ApiError::Unknown(e.to_string())
            }
            Error::UpdateSession(e) => {
                log::warn!("update session error {e}");
                ApiError::UpdateSession(e.clone())
            }
            Error::SwaggerUi(e) => {
                log::warn!("swagger ui error {e}");
                ApiError::Unknown(format!("Swagger UI error: {}", e))
            }
            Error::InvalidUsername => {
                log::warn!("Invalid username");
                ApiError::InvalidUsername
            }
            Error::InvalidEmail(email) => {
                log::warn!("Invalid email: {email}");
                ApiError::InvalidEmail(email.clone())
            }
            Error::DuplicateUsername => {
                log::warn!("Duplicate username");
                ApiError::DuplicateUsername
            }
        };
        ApiResult::Err(err)
    }
}

// Implement IntoResponse for axum error handling
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let api_error = ApiResult::from(&self);
        let status_code = self.status_code();

        (status_code, Json(api_error)).into_response()
    }
}

impl Error {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Error::IO(_)
            | Error::Config(_)
            | Error::Deser(_)
            | Error::Unknown(_)
            | Error::BackendAdminIdentityInvalid(_)
            | Error::Agent(_)
            | Error::UpdateSession(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Identity(_)
            | Error::Jwt(_)
            | Error::AuthTokenInvalid
            | Error::AuthTokenMissing => StatusCode::UNAUTHORIZED,
            Error::EnvironmentVariable(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::EnvironmentVariableMissing(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::UserAlreadyRegistered(_)
            | Error::InvalidPrincipal(_)
            | Error::InvalidEmail(_)
            | Error::InvalidUsername => StatusCode::BAD_REQUEST,
            Error::SwaggerUi(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::DuplicateUsername => StatusCode::CONFLICT,
        }
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
