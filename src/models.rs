use crate::metadata::Metadata;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Collection {
    pub name: String,
    pub address: Option<String>,
    pub start_token: u8,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct Token {
    pub uri: String,
    pub id: Option<usize>,

    pub metadata: Option<Metadata>,
    pub last_viewed: Option<DateTime<Utc>>,
}

impl Token {
    pub fn create(uri: String, id: Option<usize>) -> Token {
        Token {
            uri,
            id,
            metadata: None,
            last_viewed: None,
        }
    }

    pub(crate) fn url(&self) -> String {
        self.id
            .map_or(self.uri.clone(), |id| format!("{}{id}", self.uri))
    }
}
