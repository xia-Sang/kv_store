use serde::{Deserialize, Serialize};
#[derive(Debug, Deserialize, Serialize)]
pub enum Command {
    Set(String, String),
    Rm(String),
}
