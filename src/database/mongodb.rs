use crate::{data_types::common::DocumentId, database::mongodb::bson::Bson};
use mongodb::{
    bson::{self, doc, Document},
    options::{FindOptions, ReplaceOptions},
    Collection, Database,
};
use serde::de::DeserializeOwned;
use std::borrow::Borrow;

#[derive(Debug, Clone)]
pub struct MongoDatabase {
    database: Database,
}

impl MongoDatabase {
    pub fn new(db: &Database) -> Self {
        Self {
            database: db.clone(),
        }
    }

    pub fn typed_collection<T>(&self, name: &str) -> Collection<T> {
        self.database.collection(name)
    }

    // Return T in which 'field' has largest value
    pub async fn max<T: DeserializeOwned + Unpin + Send + Sync + std::fmt::Debug>(
        &self,
        collection: &Collection<T>,
        query: Document,
        field: &str,
    ) -> Option<T> {
        let find_res = collection
            .find(
                query,
                FindOptions::builder()
                    .limit(1)
                    .sort(doc! {field: -1})
                    .build(),
            )
            .await
            .ok();

        if let Some(mut find_data) = find_res {
            if find_data.advance().await.unwrap() {
                return Some(find_data.deserialize_current().unwrap());
            }
        }

        None
    }

    // Inserts doc_id if it doesn't exist, otherwise it replaces it
    pub async fn upsert_one<T: DeserializeOwned + Unpin + Send + Sync + serde::Serialize>(
        &self,
        collection: &Collection<T>,
        doc_id: DocumentId,
        doc: &T,
    ) {
        collection
            .replace_one(
                doc! {"_id": doc_id},
                doc,
                ReplaceOptions::builder().upsert(true).build(),
            )
            .await
            .ok();
    }

    pub async fn update_field<KT, T: DeserializeOwned + Unpin + Send + Sync, V>(
        &self,
        key_path: String,
        key_value: KT,
        collection: &Collection<T>,
        field: &str,
        value: &V,
    ) where
        V: std::clone::Clone + Into<Bson>,
        KT: std::clone::Clone + Into<Bson>,
        Bson: From<KT> + From<V>,
    {
        collection
            .update_one(
                doc! {key_path: key_value},
                doc! {"$set": {field:value}},
                None,
            )
            .await
            .unwrap();
    }

    pub async fn exists<KT, T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &Collection<T>,
        id: KT,
    ) -> bool
    where
        Bson: From<KT>,
        mongodb::bson::Document: Borrow<T>,
    {
        let found = collection.find_one(Some(doc! {"_id": id}), None).await;

        if let Ok(search_op) = found {
            if let Some(_) = search_op {
                return true;
            }
        }

        return false;
    }

    // To be used with limit as it returns Vec
    pub async fn query<T: DeserializeOwned + Unpin + Send + Sync + std::fmt::Debug>(
        &self,
        collection: &Collection<T>,
        stages: Vec<bson::Document>,
    ) -> Vec<T> {
        let mut results: Vec<T> = Vec::new();

        if let Ok(mut aggregate_res) = collection.aggregate(stages, None).await {
            while aggregate_res.advance().await.unwrap() {
                let doc = aggregate_res.deserialize_current().unwrap();                
                results.push(bson::from_bson(bson::Bson::Document(doc)).unwrap());
            }
        }

        results
    }
}
