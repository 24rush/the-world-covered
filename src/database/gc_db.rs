use std::vec;

use ::mongodb::bson::Document;
use ::mongodb::bson::{self, doc};
use ::mongodb::Collection;
use mongodb::Client;

use crate::data_types::common::Identifiable;
use crate::data_types::gc::route::Route;

use super::mongodb::MongoDatabase;

pub struct Routes {
    db_conn: MongoDatabase,
}

pub struct Statistics {
    db_conn: MongoDatabase,
}

impl Routes {
    const COLL_NAME: &str = "routes";

    pub fn new(db_conn: &MongoDatabase) -> Self {
        Self {
            db_conn: db_conn.clone(),
        }
    }

    fn typed_collection(&self) -> Collection<Route> {
        self.db_conn.typed_collection(Routes::COLL_NAME)
    }

    pub async fn get_athlete_routes(&self, ath_id: i64) -> mongodb::Cursor<Route> {
        self.typed_collection()
            .find(doc! {"athlete_id": ath_id}, None)
            .await
            .unwrap()
    }

    pub async fn get_last_route_id(
        &self,
        ath_id: i64,
    ) -> Option<crate::data_types::common::DocumentId> {
        let mut result = self
            .typed_collection()
            .aggregate(
                vec![
                    doc! {"$match": { "athlete_id": ath_id}},
                    doc! {"$sort": {"_id" : -1}},
                    doc! {"$limit": 1},
                ],
                None,
            )
            .await
            .ok()
            .unwrap();

        if result.advance().await.unwrap() {
            let doc = result.deserialize_current().unwrap();
            let route: Route = bson::from_bson(bson::Bson::Document(doc)).unwrap();

            return Some(route.as_i64());
        }

        None
    }

    pub async fn clear_routes(&self) {
        self.typed_collection()
            .delete_many(doc! {}, None)
            .await
            .ok();
    }

    pub async fn delete(&self, route: &Route) -> u64{
        self.typed_collection()
            .delete_one(doc! {"_id": route._id}, None)
            .await
            .unwrap().deleted_count
    }

    pub async fn update(&self, route: &Route) {
        self.db_conn
            .upsert_one(&self.typed_collection(), route.as_i64(), route)
            .await;
    }

    pub async fn query(&self, stages: Vec<bson::Document>) -> Vec<Route> {
        self.db_conn
            .query(&self.typed_collection(), stages)
            .await
    }
}

impl Statistics {
    const COLL_NAME: &str = "statistics";

    pub fn new(db_conn: &MongoDatabase) -> Self {
        Self {
            db_conn: db_conn.clone(),
        }
    }

    fn raw_collection(&self) -> Collection<mongodb::bson::Document> {
        self.db_conn.typed_collection(Statistics::COLL_NAME)
    }

    pub async fn query(&self) -> Vec<Document> {
        self.db_conn
            .query(
                &self.raw_collection(),
                vec![doc! { "$match": { "_id": 0 } }],
            )
            .await
            .to_owned()
    }
}

pub struct GCDB {
    pub routes: Routes,
    pub statistics: Statistics,
}

impl GCDB {
    pub async fn new(db_url: &str) -> GCDB {
        let db = Client::with_uri_str(db_url)
            .await
            .unwrap()
            .database("gc_db");

        let db_coll = MongoDatabase::new(&db);

        Self {
            routes: Routes::new(&db_coll),
            statistics: Statistics::new(&db_coll),
        }
    }
}
