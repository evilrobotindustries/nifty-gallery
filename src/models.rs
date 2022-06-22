use crate::{uri, Address};
use chrono::{DateTime, Utc};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use workers::metadata::Metadata;
use workers::{ParseError, Url};

#[derive(Clone, Deserialize, Serialize)]
pub struct Collection {
    pub address: Option<Address>,
    pub name: String,
    pub base_uri: Option<Url>,
    pub start_token: u32,
    pub tokens: IndexMap<u32, Token>,
}

impl Collection {
    pub(crate) fn add(&mut self, token: u32, mut metadata: Metadata) {
        // Parse urls
        metadata.image = uri::parse(&metadata.image).map_or(metadata.image, |url| url.to_string());
        if let Some(animation_url) = &metadata.animation_url {
            metadata.animation_url = uri::parse(&animation_url)
                .map_or(metadata.animation_url, |url| Some(url.to_string()));
        }

        let url = self
            .base_uri
            .as_ref()
            .expect("expected a base uri")
            .join(&token.to_string())
            .expect("expected a valid url");
        self.tokens.insert(
            token,
            Token {
                url,
                id: Some(token),
                metadata: Some(metadata),
                last_viewed: None,
            },
        );
    }
}

#[derive(Clone, Deserialize, Debug, Serialize)]
pub struct Token {
    pub url: Url,
    pub id: Option<u32>,

    pub metadata: Option<Metadata>,
    pub last_viewed: Option<DateTime<Utc>>,
}

impl Token {
    pub fn create(uri: String, id: Option<u32>) -> Result<Token, ParseError> {
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
