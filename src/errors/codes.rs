use std::borrow::Cow;

use actix_web::http::StatusCode;
use serde::{Serialize, Serializer};

use super::InternalError;

#[derive(Debug)]
pub enum GeneralErrorCode {
    FetchLatestRelease,
    NotFoundPlatform,

    NicknameEmpty,
    NicknameToolong,
    NicknameForbiddenCharacters,

    AuthenticationInvalidToken,
    InvalidToken,
    InvalidId,

    // error due to an error in the server
    Internal,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum ServerErrorCode {
    InvalidSha256(usize, String),
    WrongChecksum(String),
    NoReleaseFound,
    InvalidVersion,
    NotFoundPlatform(String),

    NicknameEmpty,
    NicknameToolong,
    NicknameForbiddenCharacters,

    AuthenticationInvalidToken(String),
    EmptyToken,
    InvalidToken(Option<String>),
    InvalidId,
    TokenGenerationFailed,
    JWTAccident(jsonwebtoken::errors::Error),

    // error due to an external error of the source code of the api
    External(String),
    // error due to the source code of the api
    Internal,
}

impl GeneralErrorCode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::FetchLatestRelease => "fetch_latest_release",
            Self::NotFoundPlatform => "not_found_platform",

            Self::NicknameEmpty => "nickname_empty",
            Self::NicknameToolong => "nickname_toolong",
            Self::NicknameForbiddenCharacters => "nickname_forbidden_characters",

            Self::AuthenticationInvalidToken => "authentication_invalid_token",
            Self::InvalidToken => "invalid_token",
            Self::InvalidId => "invalid_id",

            Self::Internal => "api_internal",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::FetchLatestRelease => "An internal error occured during the fetching of the latest release, please retry later",
            Self::NotFoundPlatform => "The given platform has no associated release",

            Self::NicknameEmpty => "The given nickname is empty",
            Self::NicknameToolong => "The given nickname is too long (shorten it)",
            Self::NicknameForbiddenCharacters => {
                "The given nickname has an invalid character inside, please change it"
            }
            Self::AuthenticationInvalidToken => "The given authentication token is invalid",
            Self::InvalidToken => "The given token is invalid",
            Self::InvalidId => "The given id has never been attributed to anyone",

            Self::Internal => "An internal error occured on the server, please retry later",
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::FetchLatestRelease | Self::NotFoundPlatform => StatusCode::NOT_FOUND,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}

impl Serialize for GeneralErrorCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.as_str().serialize(serializer)
    }
}

impl ServerErrorCode {
    pub fn response_code(&self) -> GeneralErrorCode {
        match self {
            Self::InvalidSha256(..)
            | Self::WrongChecksum(_)
            | Self::NoReleaseFound
            | Self::InvalidVersion => GeneralErrorCode::FetchLatestRelease,
            Self::NotFoundPlatform(_) => GeneralErrorCode::NotFoundPlatform,

            Self::NicknameEmpty => GeneralErrorCode::NicknameEmpty,
            Self::NicknameToolong => GeneralErrorCode::NicknameToolong,
            Self::NicknameForbiddenCharacters => GeneralErrorCode::NicknameForbiddenCharacters,

            Self::AuthenticationInvalidToken(_) => GeneralErrorCode::AuthenticationInvalidToken,
            Self::EmptyToken | Self::InvalidToken(_) => GeneralErrorCode::InvalidToken,
            Self::InvalidId => GeneralErrorCode::InvalidId,

            Self::TokenGenerationFailed
            | Self::JWTAccident(_)
            | Self::External(_)
            | Self::Internal => GeneralErrorCode::Internal,
        }
    }

    pub fn extra_info(&self) -> Option<Cow<str>> {
        match self {
            Self::InvalidSha256(parts, assets) => Some(Cow::Owned(format!(
                "The SHA256 file of {assets} has {parts} parts, please fix it!"
            ))),
            Self::WrongChecksum(assets) => Some(Cow::Owned(format!(
                "The SHA256 file of {assets} has an wrong checksum, please fix it!"
            ))),
            Self::NotFoundPlatform(platform) => Some(Cow::Owned(format!(
                "Someone is trying to play with the {platform} platform"
            ))),
            Self::JWTAccident(error) => Some(Cow::Owned(error.to_string())),
            Self::AuthenticationInvalidToken(token) => Some(Cow::Owned(format!(
                "The authentication token '...{}' is invalid",
                &token[token.len() - 6..token.len()]
            ))),
            Self::InvalidToken(extra) => extra.as_deref().map(|token| {
                Cow::Owned(format!(
                    "The token '...{}' is invalid",
                    &token[token.len() - 6..token.len()]
                ))
            }),
            Self::External(info) => Some(Cow::Borrowed(info)),

            _ => None,
        }
    }
}

impl From<InternalError> for ServerErrorCode {
    fn from(value: InternalError) -> Self {
        match value {
            InternalError::InvalidSha256(parts, assets) => Self::InvalidSha256(parts, assets),
            InternalError::WrongChecksum(assets) => Self::WrongChecksum(assets),
            InternalError::NoReleaseFound => Self::NoReleaseFound,
            InternalError::InvalidVersion => Self::InvalidVersion,

            InternalError::SystemTimeError => {
                Self::External("A problem occured with the time on the system".to_string())
            }

            InternalError::External(err) => Self::External(err.to_string()),
        }
    }
}
