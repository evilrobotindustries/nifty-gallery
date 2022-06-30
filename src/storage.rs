use crate::{models, Address, Route};
use gloo_storage::errors::StorageError;
use gloo_storage::{LocalStorage, Storage};
use indexmap::IndexSet;
use jsonm::packer::PackerError;
use jsonm::unpacker::UnpackerError;
use serde::de::Error;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Debug;
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

    pub fn get() -> Option<IndexSet<RecentlyViewedItem>> {
        MemoizedLocalStorage::get(Self::STORAGE_KEY).ok()
    }

    pub fn store(item: RecentlyViewedItem) {
        let mut items = Self::get().unwrap_or(IndexSet::new());
        while items.len() >= Self::MAX_ITEMS {
            // Remove the oldest items
            items.shift_remove_index(0);
        }
        if items.contains(&item) {
            items.remove(&item);
        }
        items.insert(item);
        if let Err(e) = MemoizedLocalStorage::set(Self::STORAGE_KEY, items) {
            log::error!("an error occurred whilst storing the item: {:?}", e)
        }
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

    fn get_page(collection: &str, page: usize) -> BTreeMap<u32, models::Token> {
        MemoizedLocalStorage::get(format!("{}:{collection}:{page}", Self::TOKEN)).unwrap_or_else(
            |e| {
                log::error!("An error occurred whilst fetching the token(s): {:?}", e);
                BTreeMap::new()
            },
        )
    }

    pub fn page(collection: &str, page: usize) -> Vec<models::Token> {
        Self::get_page(collection, page)
            .into_iter()
            .map(|(_, token)| token)
            .collect()
    }

    fn collection(collection: &str) -> BTreeMap<usize, usize> {
        LocalStorage::get(format!("{}:{collection}", Self::COLLECTION_TOKENS))
            .unwrap_or_else(|_| BTreeMap::new())
    }

    pub fn get(collection: &str, page: usize, token: u32) -> Option<models::Token> {
        Self::get_page(collection, page).remove(&token)
    }

    pub fn store(collection: &str, page: usize, token: models::Token) -> usize {
        // Load page and add token
        let mut tokens = Self::get_page(collection, page);
        tokens.insert(token.id, token);
        let page_count = tokens.len();
        if let Err(e) =
            MemoizedLocalStorage::set(format!("{}:{collection}:{page}", Self::TOKEN), tokens)
        {
            log::error!("An error occurred whilst storing the token: {:?}", e)
        }

        // Update collection totals
        let mut collection_tokens = Token::collection(collection);
        collection_tokens.insert(page, page_count);
        let total = collection_tokens.values().sum();
        if let Err(e) = LocalStorage::set(
            format!("{}:{collection}", Self::COLLECTION_TOKENS),
            collection_tokens,
        ) {
            log::error!(
                "An error occurred whilst storing the collection tokens: {:?}",
                e
            )
        }
        total
    }
}

/// Uses memoization to reduce local storage usage through JSON compression.
struct MemoizedLocalStorage;

impl MemoizedLocalStorage {
    fn pack<T>(value: T) -> gloo_storage::Result<String>
    where
        T: Serialize,
    {
        let mut packer = jsonm::packer::Packer::new();
        let options = jsonm::packer::PackOptions::new();
        let unpacked = json!(value);
        log::trace!("packing {unpacked}");
        let packed = packer.pack(&unpacked, &options).map_err(|e| match e {
            PackerError { .. } => gloo_storage::errors::StorageError::SerdeError(
                serde_json::Error::custom(e.to_string()),
            ),
        })?;
        log::trace!("packed {packed}");
        let packed = serde_json::to_string(&packed)?;
        log::trace!("packed string output: {packed}");
        Ok(packed)
    }

    fn unpack<T>(value: String) -> gloo_storage::Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        log::trace!("unpack string input: {value}");
        let mut unpacker = jsonm::unpacker::Unpacker::new();
        let packed = serde_json::from_str(&value)?;
        log::trace!("unpacking value from string: {packed}");
        let unpacked = unpacker.unpack(&packed).map_err(|e| match e {
            UnpackerError { cause, .. } => {
                gloo_storage::errors::StorageError::SerdeError(serde_json::Error::custom(cause))
            }
        })?;
        log::trace!("unpacked: {unpacked:?}");
        let item = serde_json::from_value(unpacked)?;
        Ok(item)
    }
}

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

impl gloo_storage::Storage for MemoizedLocalStorage {
    fn raw() -> web_sys::Storage {
        gloo_storage::LocalStorage::raw()
    }

    fn get<T>(key: impl AsRef<str>) -> gloo_storage::Result<T>
    where
        T: for<'de> Deserialize<'de>,
    {
        let key = key.as_ref();
        let item: String = Self::raw()
            .get_item(key)
            .expect_throw("unreachable: get_item does not throw an exception")
            .ok_or_else(|| gloo_storage::errors::StorageError::KeyNotFound(key.to_string()))?;

        MemoizedLocalStorage::unpack(item)
    }

    fn get_all<T>() -> gloo_storage::Result<T>
    where
        T: for<'a> Deserialize<'a>,
    {
        let local_storage = Self::raw();
        let length = Self::length();
        let mut map = serde_json::Map::with_capacity(length as usize);
        for index in 0..length {
            let key = local_storage
                .key(index)
                .map_err(js_to_error)?
                .unwrap_throw();
            let value: serde_json::Value = Self::get(&key)?;
            map.insert(key, value);
        }
        Ok(serde_json::from_value(serde_json::Value::Object(map))?)
    }

    fn set<T>(key: impl AsRef<str>, value: T) -> gloo_storage::Result<()>
    where
        T: Serialize,
    {
        let key = key.as_ref();
        let value = MemoizedLocalStorage::pack(value)?;
        Self::raw().set_item(key, &value).map_err(js_to_error)?;
        Ok(())
    }

    fn delete(key: impl AsRef<str>) {
        gloo_storage::LocalStorage::delete(key)
    }

    fn clear() {
        gloo_storage::LocalStorage::clear()
    }

    fn length() -> u32 {
        gloo_storage::LocalStorage::length()
    }
}

fn js_to_error(js_value: wasm_bindgen::JsValue) -> gloo_storage::errors::StorageError {
    match js_value.dyn_into::<js_sys::Error>() {
        Ok(error) => {
            gloo_storage::errors::StorageError::JsError(gloo_utils::errors::JsError::from(error))
        }
        Err(_) => unreachable!("JsValue passed is not an Error type - this is a bug"),
    }
}

#[cfg(test)]
mod tests {
    use jsonm::packer::{PackOptions, Packer};
    use jsonm::unpacker::Unpacker;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    #[derive(Debug, Deserialize, Serialize)]
    struct Person {
        name: String,
        age: u8,
        address: String,
    }

    impl Person {
        fn new(name: &str, age: u8, address: &str) -> Person {
            Person {
                name: name.to_string(),
                age,
                address: address.to_string(),
            }
        }
    }

    #[derive(Debug, Deserialize, Serialize)]
    struct Record {
        person: Person,
        tag: Option<String>,
    }

    impl Record {
        fn new(person: Person) -> Record {
            Record { person, tag: None }
        }
    }

    #[test]
    // fails as "tag":null becomes "tag": "name 2" through packing to string and then unpacking
    fn test() {
        let records: BTreeMap<u32, Record> = BTreeMap::from([
            (1, Record::new(Person::new("name", 18, "address 1"))),
            (2, Record::new(Person::new("name 2", 60, "address 2"))),
            (3, Record::new(Person::new("name 3", 32, "address 3"))),
            (4, Record::new(Person::new("name 4", 9, "address 2"))),
        ]);
        let serialised = serde_json::to_string(&records).unwrap();

        let packed = pack(records);
        let unpacked: BTreeMap<u32, Record> = unpack(packed);

        assert_eq!(serialised, serde_json::to_string(&unpacked).unwrap())
    }

    fn pack<T: Serialize>(value: T) -> String {
        let mut packer = Packer::new();
        let options = PackOptions::new();
        let value = serde_json::value::to_value(value).unwrap();
        println!("pack input: {value}");
        let packed = packer.pack(&value, &options).unwrap();
        println!("packed: {packed}");
        packed.to_string()
    }

    fn unpack<T: for<'de> Deserialize<'de>>(value: String) -> T {
        println!("unpack input: {value}");
        let packed = serde_json::from_str(&value).unwrap();
        println!("unpacked value: {packed}");
        let mut unpacker = Unpacker::new();
        let unpacked = unpacker.unpack(&packed).unwrap();
        println!("unpacked: {unpacked}");
        let unpacked = serde_json::from_value(unpacked).unwrap();
        unpacked
    }
}

// #[test]
// fn person_passes() {
//     let people = BTreeMap::from([
//         (1u32, Person::new("name", 18, "address 1")),
//         (2, Person::new("name 2", 60, "address 2")),
//         (3, Person::new("name 3", 32, "address 3")),
//         (4, Person::new("name 4", 9, "address 2")),
//     ]);
//     let serialised = serde_json::to_string(&people).unwrap();
//
//     let packed = pack(people);
//     let unpacked: BTreeMap<u32, Person> = unpack(packed);
//
//     assert_eq!(serialised, serde_json::to_string(&unpacked).unwrap())
// }
//
// #[test]
// fn vec() {
//     let people = vec![
//         Person::new("name", 18, "address 1"),
//         Person::new("name 2", 60, "address 2"),
//         Person::new("name 3", 32, "address 3"),
//         Person::new("name 4", 9, "address 2"),
//     ];
//
//     let serialised = serde_json::to_string(&people).unwrap();
//
//     let packed = pack(people);
//     let unpacked: Vec<Person> = unpack(packed);
//
//     assert_eq!(serialised, serde_json::to_string(&unpacked).unwrap())
// }
//
// #[test]
// fn memoizes_map() {
//     let mut test: BTreeMap<u32, Person> = BTreeMap::new();
//     test.insert(1, Person::new("name", 18, "address 1"));
//     test.insert(2, Person::new("name 2", 60, "address 2"));
//     test.insert(3, Person::new("name 3", 32, "address 3"));
//     test.insert(4, Person::new("name 4", 9, "address 2"));
//     println!("{test:?}");
//
//     let packed = MemoizedLocalStorage::pack(test).unwrap();
//     println!("{}", serde_json::to_string(&packed).unwrap());
//
//     let unpacked: BTreeMap<u32, Person> = MemoizedLocalStorage::unpack(packed).unwrap();
//     println!("{unpacked:?}");
// }
//
// #[test]
// fn memoizes_token() {
//     let tokens = BTreeMap::from([(
//         1u32,
//         Token::new(
//             1,
//             Metadata {
//                 name: Some("Some token".to_string()),
//                 description: Some("A description of the token".to_string()),
//                 image: "https://ipfs.io/CONTENTHASH/1".to_string(),
//                 external_url: None,
//                 attributes: vec![Attribute::String {
//                     trait_type: "Attribute 1".to_string(),
//                     value: "Value".to_string(),
//                 }],
//                 background_color: None,
//                 created_by: None,
//                 animation_url: None,
//                 youtube_url: None,
//             },
//         ),
//     )]);
//     let serialised = serde_json::to_string(&tokens).unwrap();
//
//     let packed = pack(tokens);
//     let unpacked: BTreeMap<u32, Token> = unpack(packed);
//
//     assert_eq!(serialised, serde_json::to_string(&unpacked).unwrap())
// }
//
// #[derive(Debug, Deserialize, Serialize)]
// struct None {
//     name: Option<String>,
// }
//
// #[test]
// fn none_test() {
//     let none = BTreeMap::from([(1u32, None { name: None })]);
//     let serialised = serde_json::to_string(&none).unwrap();
//
//     let packed = pack(none);
//     let unpacked: BTreeMap<u32, None> = unpack(packed);
//
//     assert_eq!(serialised, serde_json::to_string(&unpacked).unwrap())
// }
//
// #[derive(Clone, Debug, Deserialize, Serialize)]
// pub struct Token {
//     pub id: u32,
//     pub metadata: Option<Metadata>,
//     pub last_viewed: Option<DateTime<Utc>>,
// }
//
// impl Token {
//     fn new(id: u32, metadata: Metadata) -> Token {
//         Token {
//             id,
//             metadata: Some(metadata),
//             last_viewed: None,
//         }
//     }
// }
//
// #[derive(Clone, Debug, Deserialize, Serialize)]
// pub struct Metadata {
//     pub name: Option<String>,
//     pub description: Option<String>,
//     pub image: String,
//     pub external_url: Option<String>,
//     pub attributes: Vec<Attribute>,
//     pub background_color: Option<String>,
//     pub created_by: Option<String>,
//     pub animation_url: Option<String>,
//     pub youtube_url: Option<String>,
// }
//
// #[derive(Clone, Debug, Deserialize, Serialize)]
// pub enum Attribute {
//     String {
//         trait_type: String,
//         value: String,
//     },
//     Number {
//         trait_type: String,
//         value: i64,
//         max_value: Option<usize>,
//     },
//     BoostPercentage {
//         trait_type: String,
//         value: f64,
//         max_value: Option<usize>,
//     },
//     BoostNumber {
//         trait_type: String,
//         value: f64,
//         max_value: Option<usize>,
//     },
//     Date {
//         trait_type: String,
//         value: u64,
//     },
// }
