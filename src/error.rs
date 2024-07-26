use std::io;

use failure::Fail;

#[derive(Debug, Fail)]
pub enum KvStoreError {
    #[fail(display = "{}", _0)]
    Io(#[cause] io::Error),
    #[fail(display = "{}", _0)]
    Serde(#[cause] serde_json::Error),
    #[fail(display = "key Not Found")]
    KeyNotFound,
    #[fail(display = "unknown command type")]
    UnKnowCommandType,
}

impl From<io::Error> for KvStoreError {
    fn from(value: io::Error) -> Self {
        KvStoreError::Io(value)
    }
}
impl From<serde_json::Error> for KvStoreError {
    fn from(value: serde_json::Error) -> Self {
        KvStoreError::Serde(value)
    }
}
pub type Result<T> = std::result::Result<T, KvStoreError>;
