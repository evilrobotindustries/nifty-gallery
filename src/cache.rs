use gloo_console::error;
use gloo_storage::errors::StorageError;
use gloo_storage::{LocalStorage, Storage};
use itertools::Itertools;
use std::collections::HashMap;

pub struct Collection {}

impl Collection {
    const STORAGE_KEY: &'static str = "Collections:Viewed";

    fn cache() -> gloo_storage::Result<HashMap<String, crate::models::Collection>> {
        LocalStorage::get(Collection::STORAGE_KEY)
    }

    fn clear() {
        LocalStorage::delete(Collection::STORAGE_KEY)
    }

    pub fn get(key: &str) -> Option<crate::models::Collection> {
        match Collection::cache() {
            Ok(mut cache) => {
                return cache.remove(key);
            }
            Err(e) => {
                if !matches!(e, StorageError::KeyNotFound(_)) {
                    Collection::clear();
                    error!(format!("{:?}", e))
                }
                None
            }
        }
    }

    pub fn insert(key: String, value: crate::models::Collection) {
        let mut cache = Collection::cache().unwrap_or(HashMap::new());
        cache.insert(key, value);
        if let Err(e) = LocalStorage::set(Collection::STORAGE_KEY, cache) {
            error!(format!(
                "An error occurred whilst caching the collection: {:?}",
                e
            ))
        }
    }

    pub fn items() -> Option<HashMap<String, crate::models::Collection>> {
        Collection::cache().map_or(None, |cache| Some(cache))
    }

    pub fn values() -> Option<Vec<crate::models::Collection>> {
        Collection::cache().map_or(None, |cache| Some(cache.into_values().collect()))
    }
}

pub struct Token {}

impl Token {
    const STORAGE_KEY: &'static str = "Tokens:Viewed";
    const CACHE_SIZE: usize = 10;

    fn cache() -> gloo_storage::Result<HashMap<String, crate::models::Token>> {
        LocalStorage::get(Token::STORAGE_KEY)
    }

    fn clear() {
        LocalStorage::delete(Token::STORAGE_KEY)
    }

    pub fn get(key: &str) -> Option<crate::models::Token> {
        match Token::cache() {
            Ok(mut cache) => {
                return cache.remove(key);
            }
            Err(e) => {
                if !matches!(e, StorageError::KeyNotFound(_)) {
                    Token::clear();
                    error!(format!("{:?}", e))
                }
                None
            }
        }
    }

    pub fn insert(key: String, value: crate::models::Token) {
        let mut cache = Token::cache().unwrap_or(HashMap::new());
        if cache.len() >= Token::CACHE_SIZE {
            let expired: Vec<String> = cache
                .iter()
                .sorted_by_key(|(_, value)| value.last_viewed.unwrap_or(chrono::offset::Utc::now()))
                .take(cache.len() - Token::CACHE_SIZE + 1)
                .map(|(key, _)| key.clone())
                .collect();
            for key in expired {
                cache.remove(&key);
            }
        }
        cache.insert(key, value);
        if let Err(e) = LocalStorage::set(Token::STORAGE_KEY, cache) {
            error!(format!(
                "An error occurred whilst caching the token: {:?}",
                e
            ))
        }
    }

    pub fn values() -> Option<Vec<crate::models::Token>> {
        Token::cache().map_or(None, |cache| Some(cache.into_values().collect()))
    }
}
