use crate::{data_types::common::Identifiable, database::mongodb::bson::Bson};
use mongodb::{
    bson::{self, doc, Document},
    options::ReplaceOptions,
    sync::{Client, Collection, Database},
};
use serde::de::DeserializeOwned;
use std::borrow::Borrow;

use super::cursors::I64Cursor;

#[derive(Debug, Clone)]
pub struct MongoConnection {
    database: Database,
}

impl MongoConnection {
    pub fn new() -> Self {
        Self {
            database: Client::with_uri_str("mongodb://localhost:27017")
                .unwrap()
                .database("strava_db"),
        }
    }

    pub fn collection<T>(&self, name: &str) -> Collection<T> {
        self.database.collection(name)
    }

    fn doc_id_to_i64(data: Document) -> i64 {
        match data.get("_id").unwrap() {
            Bson::Int32(_) => data.get_i32("_id").unwrap().into(),
            Bson::Int64(_) => data.get_i64("_id").unwrap().into(),
            Bson::Double(_) => data.get_f64("_id").unwrap() as i64,
            _ => panic!("Can't map"),
        }
    }

    pub fn keys_id<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &Collection<mongodb::bson::Document>,
    ) -> I64Cursor {
        let cursor: mongodb::sync::Cursor<mongodb::bson::Document> =
            collection.find(None, None).unwrap();

        I64Cursor::new(cursor, Self::doc_id_to_i64)
    }

    pub fn find<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,        
        collection: &Collection<T>,
        query: Document,
    ) -> mongodb::sync::Cursor<T>    
    {
        collection
            .find(query, None)
            .ok()
            .unwrap()
    }

    pub fn find_one<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,        
        collection: &Collection<T>,
        query: Document,
    ) -> Option<T>
    {
        collection
            .find_one(query, None)
            .ok()
            .unwrap()
    }

    // Function to set a JSON (used when retrieving data from web APIs)
    pub fn upsert_one_raw<T: DeserializeOwned + Unpin + Send + Sync + serde::Serialize>(
        &self,
        collection: &Collection<T>,
        doc: &serde_json::Value,
    ) -> Option<bool>
    where
        mongodb::bson::Document: Borrow<T>,
    {
        let res = collection
            .replace_one(
                doc! {"_id": doc.get("_id").unwrap().as_f64().unwrap() as i64},
                bson::to_document(doc).unwrap().borrow(),
                ReplaceOptions::builder().upsert(true).build(),
            )
            .unwrap();

        Some(res.modified_count > 0)
    }

    // Function to set a typed object 
    pub fn upsert_one<T: DeserializeOwned + Unpin + Send + Sync + serde::Serialize>(
        &self,
        collection: &Collection<T>,
        doc: &T,
    ) -> Option<bool>
    where
        T: Identifiable,
    {
        let res = collection
            .replace_one(doc! {"_id": doc.as_i64()}, doc, None)
            .unwrap();

        Some(res.modified_count > 0)
    }

    pub fn update_field<KT, T: DeserializeOwned + Unpin + Send + Sync, V>(
        &self,
        key: KT,
        collection: &Collection<T>,
        field: &str,
        value: &V,
    ) -> Option<bool>
    where
        V: std::clone::Clone + Into<Bson>,
        KT: std::clone::Clone + Into<Bson>,
        Bson: From<KT> + From<V>,
    {
        let filter = doc! {"_id": key};
        let update = doc! {"$set": {field:value}};

        Some(
            collection
                .update_one(filter, update, None)
                .unwrap()
                .modified_count
                > 0,
        )
    }

    pub fn exists<KT, T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &Collection<mongodb::bson::Document>,
        id: KT,
    ) -> bool
    where
        Bson: From<KT>,
        mongodb::bson::Document: Borrow<T>,
    {
        let found = collection.find_one(Some(doc! {"_id": id}), None);

        if let Ok(search_op) = found {
            if let Some(_) = search_op {
                return true;
            }
        }

        return false;
    }
}
