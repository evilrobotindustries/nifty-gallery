use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use workers::metadata::Metadata;
use workers::{ParseError, Url};

#[derive(Deserialize, Serialize)]
pub struct Collection {
    pub name: String,
    pub address: Option<String>,
    pub start_token: u8,
    pub base_uri: Option<String>,
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct Token {
    pub url: Url,
    pub id: Option<usize>,

    pub metadata: Option<Metadata>,
    pub last_viewed: Option<DateTime<Utc>>,
}

impl Token {
    pub fn create(uri: String, id: Option<usize>) -> Result<Token, ParseError> {
        let mut url = Url::parse(&uri)?;
        if let Some(id) = id {
            url = url.join(&id.to_string())?;
        }
        Ok(Token {
            url,
            id,
            metadata: None,
            last_viewed: None,
        })
    }
}
