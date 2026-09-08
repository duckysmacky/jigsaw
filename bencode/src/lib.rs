pub mod types;
pub mod parser;
pub mod util;

use thiserror::Error;

/// Public-oriented utility error type to use when extracting values from bencode dictionaries
#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("Field '{0}' is of wrong type.")]
    WrongType(String),
    #[error("Missing required key '{0}'.")]
    RequiredKeyMissing(String),
}