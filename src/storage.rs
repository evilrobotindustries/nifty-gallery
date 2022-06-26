use crate::{models, Address, Route};
use gloo_storage::{LocalStorage, Storage};
use indexmap::IndexSet;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use workers::etherscan::TypeExtensions;

pub trait Get<I, T> {
    fn get(id: I) -> T;
}

pub trait All<T> {
    fn get() -> T;
}

impl Get<&Address, Option<models::Collection>> for Collection {
    fn get(id: &Address) -> Option<models::Collection> {
        LocalStorage::get(format!(
            "{}:{}",
            Self::COLLECTION,
            TypeExtensions::format(id)
        ))
        .ok()
    }
}

impl Get<&str, Option<crate::models::Collection>> for Collection {
    fn get(id: &str) -> Option<crate::models::Collection> {
        LocalStorage::get(format!("{}:{id}", Self::COLLECTION)).ok()
    }
}

impl All<Vec<models::Collection>> for Collection {
    fn get() -> Vec<models::Collection> {
        let collections: HashSet<String> =
            LocalStorage::get(Self::COLLECTIONS).unwrap_or_else(|_| HashSet::new());
        collections
            .iter()
            .filter_map(|id| {
                <Collection as Get<&str, Option<models::Collection>>>::get(id.as_str())
            })
            .collect()
    }
}

pub struct Collection {}

impl Collection {
    const COLLECTION: &'static str = "C";
    const COLLECTIONS: &'static str = "CS";

    pub fn contains(collection: &crate::models::Collection) -> bool {
        let collection: gloo_storage::Result<models::Collection> =
            LocalStorage::get(format!("{}:{}", Self::COLLECTION, collection.id()));
        collection.is_ok()
    }

    pub fn store(collection: crate::models::Collection) {
        // Store individual item
        let id = collection.id();
        if let Err(e) = LocalStorage::set(format!("{}:{id}", Self::COLLECTION), collection.clone())
        {
            log::error!("An error occurred whilst storing the collection: {:?}", e)
        }

        // Add to list
        let mut collections: HashSet<String> =
            LocalStorage::get(Self::COLLECTIONS).unwrap_or_else(|_| HashSet::new());
        collections.insert(id);
        if let Err(e) = LocalStorage::set(Self::COLLECTIONS, collections) {
            log::error!("An error occurred whilst storing the collection: {:?}", e)
        }
    }
}

pub struct RecentlyViewed {}

impl RecentlyViewed {
    const STORAGE_KEY: &'static str = "RV";
    const MAX_ITEMS: usize = 10;

    fn data() -> gloo_storage::Result<IndexSet<RecentlyViewedItem>> {
        LocalStorage::get(Self::STORAGE_KEY)
    }

    pub fn insert(item: RecentlyViewedItem) {
        let mut data = Self::data().unwrap_or(IndexSet::new());
        while data.len() >= Self::MAX_ITEMS {
            // Remove the oldest items
            data.shift_remove_index(0);
        }
        data.insert(item);
        if let Err(e) = LocalStorage::set(Self::STORAGE_KEY, data) {
            log::error!("an error occurred whilst storing the item: {:?}", e)
        }
    }

    pub fn values() -> Option<IndexSet<RecentlyViewedItem>> {
        Self::data().ok()
    }
}

#[derive(Eq, Hash, PartialEq, Deserialize, Serialize)]
pub struct RecentlyViewedItem {
    pub name: String,
    pub image: String,
    pub route: Route,
}

pub struct Token {}

impl Token {
    const TOKEN: &'static str = "T";
    const COLLECTION_TOKENS: &'static str = "CT";

    pub fn all(collection: &str) -> Vec<models::Token> {
        Token::collection(collection)
            .iter()
            .map(|token| Token::get(collection, *token))
            .filter(|t| t.is_some())
            .map(|t| t.unwrap())
            .collect()
    }

    pub fn get(collection: &str, token: u32) -> Option<models::Token> {
        LocalStorage::get(format!("{}:{collection}:{token}", Self::TOKEN)).ok()
    }

    pub fn store(collection: &str, token: models::Token) {
        let id = token.id;
        if let Err(e) = LocalStorage::set(format!("{}:{collection}:{}", Self::TOKEN, id), token) {
            log::error!("An error occurred whilst storing the token: {:?}", e)
        }

        // Add to collection
        let mut collection_tokens = Token::collection(collection);
        collection_tokens.insert(id);
        if let Err(e) = LocalStorage::set(
            format!("{}:{collection}", Self::COLLECTION_TOKENS),
            collection_tokens,
        ) {
            log::error!(
                "An error occurred whilst storing the collection tokens: {:?}",
                e
            )
        }
    }

    fn collection(collection: &str) -> HashSet<u32> {
        LocalStorage::get(format!("{}:{collection}", Self::COLLECTION_TOKENS))
            .unwrap_or_else(|_| HashSet::new())
    }
}
