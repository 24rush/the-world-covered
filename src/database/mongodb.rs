use crate::{data_types::{common::Identifiable}, database::mongodb::bson::Bson};
use mongodb::{
    bson::{self, doc, Document},
    options::{FindOptions, ReplaceOptions},
    Client, Collection, Database,
};
use serde::de::DeserializeOwned;
use std::borrow::Borrow;

#[derive(Debug, Clone)]
pub struct MongoConnection {
    database: Database,
}

impl MongoConnection {
    pub async fn new(db: &'static str) -> Self {
        Self {
            database: Client::with_uri_str("mongodb://localhost:27017")
                .await
                .unwrap()
                .database(db),
        }
    }

    pub fn collection<T>(&self, name: &str) -> Collection<T> {
        self.database.collection(name)
    }

    pub async fn max<T: DeserializeOwned + Unpin + Send + Sync + std::fmt::Debug>(
        &self,
        collection: &Collection<T>,
        query: Document,
        field: &str,
    ) -> Option<T> {
        let mut cursor = collection
            .find(
                query,
                FindOptions::builder()
                    .limit(1)
                    .sort(doc! {field: -1})
                    .build(),
            )
            .await
            .unwrap();

        if let Ok(found) = cursor.advance().await {
            if found {
                return Some(cursor.deserialize_current().unwrap());
            }
        }

        None
    }

    pub async fn aggregate<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &Collection<T>,
        query: Vec<Document>,
    ) -> mongodb::Cursor<Document> {
        collection.aggregate(query, None).await.ok().unwrap()
    }

    pub async fn find<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &Collection<T>,
        query: Document,
    ) -> mongodb::Cursor<T> {
        collection.find(query, None).await.ok().unwrap()
    }

    pub async fn find_one<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &Collection<T>,
        query: Document,
    ) -> Option<T> {
        collection.find_one(query, None).await.ok().unwrap()
    }

    // Function to set a JSON (used when retrieving data from web APIs)
    pub async fn upsert_one_raw<T: DeserializeOwned + Unpin + Send + Sync + serde::Serialize>(
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
            .await
            .unwrap();

        Some(res.modified_count > 0)
    }

    // Function to set a typed object
    pub async fn upsert_one<T: DeserializeOwned + Unpin + Send + Sync + serde::Serialize>(
        &self,
        collection: &Collection<T>,
        doc: &T,
    ) -> Option<bool>
    where
        T: Identifiable,
    {
        let res = collection
            .replace_one(
                doc! {"_id": doc.as_i64()},
                doc,
                ReplaceOptions::builder().upsert(true).build(),
            )
            .await
            .unwrap();

        Some(res.modified_count > 0)
    }

    pub async fn remove_all<T: DeserializeOwned + Unpin + Send + Sync + serde::Serialize>(
        &self,
        collection: &Collection<T>,
    ) -> Option<bool>
    where
        T: Identifiable,
    {
        let res = collection.delete_many(doc! {}, None).await.unwrap();

        Some(res.deleted_count > 0)
    }

    pub async fn update_field<KT, T: DeserializeOwned + Unpin + Send + Sync, V>(
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
                .await
                .unwrap()
                .modified_count
                > 0,
        )
    }

    pub async fn exists<KT, T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &Collection<mongodb::bson::Document>,
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
}
