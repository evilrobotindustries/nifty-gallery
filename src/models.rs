use crate::Address;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use workers::etherscan::TypeExtensions;
use workers::metadata::Metadata;
use workers::Url;

#[derive(Clone, Deserialize, Serialize)]
pub enum Collection {
    /// Collection is sourced from a smart contract address
    #[serde(rename = "c")]
    Contract {
        #[serde(rename = "a")]
        address: Address,
        #[serde(rename = "n")]
        name: String,
        #[serde(rename = "bu")]
        base_uri: Option<Url>,
        #[serde(rename = "st")]
        start_token: u32,
        #[serde(rename = "ts")]
        total_supply: Option<u32>,
        #[serde(rename = "lv")]
        last_viewed: Option<DateTime<Utc>>,
    },
    /// Collection is sourced from url
    #[serde(rename = "u")]
    Url {
        #[serde(rename = "i")]
        id: String,
        #[serde(rename = "bu")]
        base_uri: Option<Url>,
        #[serde(rename = "st")]
        start_token: u32,
        #[serde(rename = "ts")]
        total_supply: Option<u32>,
        #[serde(rename = "lv")]
        last_viewed: Option<DateTime<Utc>>,
    },
}

impl Collection {
    pub fn new(address: &str, name: &str, base_uri: &str, total_supply: Option<u32>) -> Collection {
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

    pub fn set_base_uri(&mut self, value: Url) {
        match self {
            Collection::Contract { base_uri, .. } => *base_uri = Some(value),
            Collection::Url { base_uri, .. } => *base_uri = Some(value),
        }
    }

    pub fn set_last_viewed(&mut self) {
        match self {
            Collection::Contract { last_viewed, .. } => {
                *last_viewed = Some(chrono::offset::Utc::now())
            }
            Collection::Url { last_viewed, .. } => *last_viewed = Some(chrono::offset::Utc::now()),
        }
    }

    pub fn increment_start_token(&mut self, increment: u32) {
        match self {
            Collection::Contract { start_token, .. } => *start_token += increment,
            Collection::Url { start_token, .. } => *start_token += increment,
        }
    }

    pub fn set_total_supply(&mut self, value: u32) {
        match self {
            Collection::Contract { total_supply, .. } => *total_supply = Some(value),
            Collection::Url { total_supply, .. } => *total_supply = Some(value),
        }
    }

    pub fn base_uri(&self) -> &Option<Url> {
        match self {
            Collection::Contract { base_uri, .. } => base_uri,
            Collection::Url { base_uri, .. } => base_uri,
        }
    }

    pub fn id(&self) -> String {
        match self {
            Collection::Contract { address, .. } => TypeExtensions::format(address),
            Collection::Url { id, .. } => id.clone(),
        }
    }

    pub fn last_viewed(&self) -> &Option<DateTime<Utc>> {
        match self {
            Collection::Contract { last_viewed, .. } => last_viewed,
            Collection::Url { last_viewed, .. } => last_viewed,
        }
    }

    pub fn name(&self) -> Option<&str> {
        match self {
            Collection::Contract { name, .. } => Some(name.as_str()),
            Collection::Url { base_uri, .. } => base_uri.as_ref().map(|u| u.as_str()),
        }
    }

    pub fn start_token(&self) -> &u32 {
        match self {
            Collection::Contract { start_token, .. } => start_token,
            Collection::Url { start_token, .. } => start_token,
        }
    }

    pub fn total_supply(&self) -> &Option<u32> {
        match self {
            Collection::Contract { total_supply, .. } => total_supply,
            Collection::Url { total_supply, .. } => total_supply,
        }
    }

    pub(crate) fn url(&self, token: u32) -> Option<String> {
        self.base_uri().as_ref().map(|base_uri| {
            base_uri
                .join(token.to_string().as_str())
                .expect("unable to create token metadata request url")
                .to_string()
        })
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Token {
    #[serde(rename = "i")]
    pub id: u32,
    #[serde(rename = "m")]
    pub metadata: Option<Metadata>,
    #[serde(rename = "lv")]
    pub last_viewed: Option<DateTime<Utc>>,
}

impl Token {
    pub fn new(id: u32, metadata: Metadata) -> Self {
        Self {
            id,
            metadata: Some(metadata),
            last_viewed: None,
        }
    }
}
