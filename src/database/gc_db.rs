use std::vec;

use ::mongodb::{Collection, Cursor};
use ::mongodb::bson::Document;
use ::mongodb::bson::{self, doc};

use crate::data_types::gc::{route::Route, effort::Effort};

use super::mongodb::{MongoConnection};

struct GCCollections {
    routes: Collection<Route>,
    efforts: Collection<Effort>,
    statistics: Collection<Document>
}

pub struct GCDB {
    pub db_conn: MongoConnection,
    colls: GCCollections,
}

impl GCDB {
    pub async fn new() -> Self {
        let mongo_conn = MongoConnection::new("gc_db").await;
        let routes: Collection<Route> = mongo_conn.collection("routes");
        let efforts: Collection<Effort> = mongo_conn.collection("efforts");
        let statistics: Collection<Document> = mongo_conn.collection("statistics");
        Self {
            db_conn: mongo_conn,
            colls: GCCollections { routes, efforts, statistics },
        }
    }

    pub async fn clear_routes(&self) {
        self.db_conn.remove_all(&self.colls.routes).await;
    }

    pub async fn clear_efforts(&self) {
        self.db_conn.remove_all(&self.colls.efforts).await;
    }

    pub async fn update_effort(&self, effort: &Effort) {
        self.db_conn.upsert_one(&self.colls.efforts, effort).await;
    }

    pub async fn update_route(&self, route: &Route) {
        self.db_conn.upsert_one(&self.colls.routes, route).await;
    }

    pub async fn get_routes(&self, ath_id: i64) -> Cursor<Route> {
        self.db_conn
            .find::<Route>(&self.colls.routes, doc! {"athlete_id": ath_id}).await
    }

    pub async fn query_efforts(&self, stages: Vec<bson::Document>) -> Vec<Effort> {
        self.db_conn.query(&self.colls.efforts, stages).await
    }

    pub async fn query_routes(&self, stages: Vec<bson::Document>) -> Vec<Route> {
        self.db_conn.query(&self.colls.routes, stages).await
    }

    pub async fn query_statistics(&self) -> Document {
        self.db_conn.query(&self.colls.statistics, vec![]).await[0].to_owned()
    }
}
