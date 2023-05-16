use mongodb::{sync::Collection, bson::doc};

use crate::data_types::gc::{route::Route, segment::{Segment, self}, effort::Effort};

use super::mongodb::MongoConnection;

struct GCCollections {
    routes: Collection<Route>,
    segments: Collection<Segment>,
    efforts: Collection<Effort>
}

pub struct GCDB {
    pub db_conn: MongoConnection,
    colls: GCCollections,
}

impl GCDB {
    pub fn new() -> Self {
        let mongo_conn = MongoConnection::new("gc_db");
        let routes: Collection<Route> = mongo_conn.collection("routes");
        let segments: Collection<Segment> = mongo_conn.collection("segments");
        let efforts: Collection<Effort> = mongo_conn.collection("efforts");

        Self {
            db_conn: mongo_conn,
            colls: GCCollections { routes, segments, efforts },
        }
    }

    pub fn clear_routes(&self) {
        self.db_conn.remove_all(&self.colls.routes);
    }

    pub fn clear_segments(&self) {
        self.db_conn.remove_all(&self.colls.segments);
    }

    pub fn update_segment(&self, segment: &Segment) {
        self.db_conn.upsert_one(&self.colls.segments, segment);
    }

    pub fn update_effort(&self, effort: &Effort) {
        self.db_conn.upsert_one(&self.colls.efforts, effort);
    }

    pub fn update_route(&self, route: &Route) {
        self.db_conn.upsert_one(&self.colls.routes, route);
    }

    pub fn get_routes(&self, ath_id: i64) -> mongodb::sync::Cursor<Route> {
        self.db_conn
            .find::<Route>(&self.colls.routes, doc! {"athlete_id": ath_id})
    }
}
