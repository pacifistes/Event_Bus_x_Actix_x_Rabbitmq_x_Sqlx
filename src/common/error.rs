use actix_web::{HttpResponse, ResponseError};
use derive_more::Display;
use serde::Serialize;

#[allow(dead_code)]
#[derive(Debug, Display, Serialize)]
pub enum AppError {
    #[display("Not found: {}", message)]
    NotFound { message: String },
    #[display("Forbidden: {}", message)]
    Forbidden { message: String },
    #[display("Unauthorized: {}", message)]
    Unauthorized { message: String },
    #[display("Internal server error: {}", message)]
    InternalServerError { message: String },
    #[display("Invalid request parameters: {}", message)]
    BadRequest { message: String },
}

#[allow(dead_code)]
pub type AppResult<T> = std::result::Result<T, AppError>;

#[macro_export]
macro_rules! internal_error {
    ($target:ty : $($other:path), *) => {
        $(
            impl From<$other> for $target {
                fn from(other: $other) -> Self {
                    Self::InternalServerError { message: other.to_string() }
                }
            }
        )*
    }
}

internal_error!(
    AppError: std::io::Error, sqlx::Error, actix_web::error::Error
);

#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    message: String,
    error_type: String,
}

impl ResponseError for AppError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self {
            AppError::NotFound { .. } => actix_web::http::StatusCode::NOT_FOUND,
            AppError::Forbidden { .. } => actix_web::http::StatusCode::FORBIDDEN,
            AppError::Unauthorized { .. } => actix_web::http::StatusCode::UNAUTHORIZED,
            AppError::InternalServerError { .. } => {
                actix_web::http::StatusCode::INTERNAL_SERVER_ERROR
            }
            AppError::BadRequest { .. } => actix_web::http::StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let status_code = self.status_code();
        let error_response = ErrorResponse {
            code: status_code.as_u16(),
            message: self.to_string(),
            error_type: format!("{:?}", self),
        };

        HttpResponse::build(status_code).json(error_response)
    }
}

#[allow(dead_code)]
impl AppError {
    pub fn not_found(message: impl Into<String>) -> Self {
        AppError::NotFound {
            message: message.into(),
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        AppError::Forbidden {
            message: message.into(),
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        AppError::Unauthorized {
            message: message.into(),
        }
    }

    pub fn internal_server_error(message: impl Into<String>) -> Self {
        AppError::InternalServerError {
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        AppError::BadRequest {
            message: message.into(),
        }
    }
}
