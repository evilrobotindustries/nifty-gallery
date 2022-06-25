use crate::Address;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use workers::etherscan::TypeExtensions;
use workers::metadata::Metadata;
use workers::{ParseError, Url};

#[derive(Clone, Deserialize, Serialize)]
pub enum Collection {
    /// Collection is sourced from a smart contract address
    Contract {
        address: Address,
        name: String,
        base_uri: Option<Url>,
        start_token: u32,
        total_supply: Option<u32>,
        last_viewed: Option<DateTime<Utc>>,
    },
    /// Collection is sourced from url
    Url {
        url: String,
        base_uri: Option<Url>,
        start_token: u32,
        total_supply: Option<u32>,
        last_viewed: Option<DateTime<Utc>>,
    },
}

impl Collection {
    pub(crate) fn new(
        address: &str,
        name: &str,
        base_uri: &str,
        total_supply: Option<u32>,
    ) -> Collection {
        Collection::Contract {
            address: Address::from_str(address)
                .expect(&format!("unable to parse {address} as an address")),
            name: name.to_string(),
            base_uri: Some(
                Url::from_str(base_uri)
                    .expect(&format!("unable to parse {base_uri} as a url").to_string()),
            ),
            start_token: 0,
            total_supply,
            last_viewed: None,
        }
    }

    pub(crate) fn set_base_uri(&mut self, value: Url) {
        match self {
            Collection::Contract { base_uri, .. } => *base_uri = Some(value),
            Collection::Url { base_uri, .. } => *base_uri = Some(value),
        }
    }

    pub(crate) fn set_last_viewed(&mut self) {
        match self {
            Collection::Contract { last_viewed, .. } => {
                *last_viewed = Some(chrono::offset::Utc::now())
            }
            Collection::Url { last_viewed, .. } => *last_viewed = Some(chrono::offset::Utc::now()),
        }
    }

    pub(crate) fn increment_start_token(&mut self, increment: u32) {
        match self {
            Collection::Contract { start_token, .. } => *start_token += increment,
            Collection::Url { start_token, .. } => *start_token += increment,
        }
    }

    pub(crate) fn set_total_supply(&mut self, value: u32) {
        match self {
            Collection::Contract { total_supply, .. } => *total_supply = Some(value),
            Collection::Url { total_supply, .. } => *total_supply = Some(value),
        }
    }

    pub(crate) fn id(&self) -> String {
        match self {
            Collection::Contract { address, .. } => TypeExtensions::format(address),
            Collection::Url { url, .. } => url.clone(),
        }
    }

    pub(crate) fn last_viewed(&self) -> &Option<DateTime<Utc>> {
        match self {
            Collection::Contract { last_viewed, .. } => last_viewed,
            Collection::Url { last_viewed, .. } => last_viewed,
        }
    }

    pub(crate) fn name(&self) -> Option<&str> {
        match self {
            Collection::Contract { name, .. } => Some(name.as_str()),
            Collection::Url { .. } => None,
        }
    }

    pub(crate) fn start_token(&self) -> &u32 {
        match self {
            Collection::Contract { start_token, .. } => start_token,
            Collection::Url { start_token, .. } => start_token,
        }
    }

    pub(crate) fn total_supply(&self) -> &Option<u32> {
        match self {
            Collection::Contract { total_supply, .. } => total_supply,
            Collection::Url { total_supply, .. } => total_supply,
        }
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
