use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use actix_web::{HttpResponse, HttpResponseBuilder, ResponseError};
use serde::Serialize;
use std::fmt;

use super::codes::{GeneralErrorCode, ServerErrorCode};

#[derive(Debug)]
pub enum ErrorCause {
    Database,
    Internal,
}

#[derive(Debug, Serialize)]
pub struct RequestError {
    err_code: GeneralErrorCode,
    err_desc: String,
}

#[derive(Debug)]
pub enum RouteError {
    ServerError(ErrorCause, ServerErrorCode),
    InvalidRequest(ServerErrorCode, String),
}

impl RequestError {
    pub fn new(code: GeneralErrorCode, description: String) -> Self {
        Self {
            err_code: code,
            err_desc: description,
        }
    }
}

impl fmt::Display for RouteError {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        unimplemented!()
    }
}

impl ResponseError for RouteError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::ServerError(..) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvalidRequest(code, _) => code.response_code().status_code(),
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        let mut response = HttpResponseBuilder::new(self.status_code());
        match self {
            Self::ServerError(cause, server_err_code) => {
                log::error!("{cause:?} error: {:?}", server_err_code);
                if let Some(extra) = server_err_code.extra_info() {
                    log::error!("Extra info: {extra}");
                }

                let response_code = server_err_code.response_code();
                let description = response_code.description().to_string();
                response.json(RequestError::new(response_code, description))
            }
            Self::InvalidRequest(code, description) => {
                log::error!("{:?} error: {}", code, description);
                if let Some(extra) = code.extra_info() {
                    log::error!("Extra info: {extra}");
                }

                response.json(RequestError::new(code.response_code(), description.clone()))
            }
        }
    }
}

// to delete '$into_type:path' you need to use proc macros and further manipulation of the AST
macro_rules! error_from {
    (transform $from:path, $into_type:path, |$err_name:ident| $blk:block) => {
        impl From<$from> for $into_type {
            fn from($err_name: $from) -> Self {
                $blk
            }
        }
    };
    (transform_io $from:path, $into_type:path) => {
        impl From<$from> for $into_type {
            fn from(err: $from) -> Self {
                std::io::Error::from(err).into()
            }
        }
    };
}

error_from! { transform_io rand_core::Error, RouteError }
error_from! { transform std::io::Error, RouteError, |value| {
    RouteError::ServerError(
        ErrorCause::Internal,
        ServerErrorCode::External(value.to_string())
    )
} }
error_from! { transform tokio_postgres::Error, RouteError, |value| {
    RouteError::ServerError(
        ErrorCause::Database,
        ServerErrorCode::External(value.to_string())
    )
} }

error_from! { transform deadpool_postgres::PoolError, RouteError, |value| {
    RouteError::ServerError(
        ErrorCause::Database,
        ServerErrorCode::External(value.to_string())
    )
} }

error_from! { transform jsonwebtoken::errors::Error, RouteError, |value| {
    RouteError::ServerError(
        ErrorCause::Internal,
        ServerErrorCode::JWTAccident(value)
    )
} }
