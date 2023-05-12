use mongodb::{
    bson::{self, Bson},
    sync::Collection,
};

use crate::data_types::{
    athlete::{AthleteData, AthleteTokens},
};

use super::{cursors::I64Cursor, mongodb::MongoConnection};

struct Connections {
    athletes: Collection<AthleteData>,
    activities: Collection<mongodb::bson::Document>,
    telemetry: Collection<mongodb::bson::Document>,
}
pub struct Persistance {
    pub db_conn: MongoConnection,
    colls: Connections,
}

impl Persistance {
    pub fn new() -> Self {
        let mongo_conn = MongoConnection::new();
        let athletes: Collection<AthleteData> = mongo_conn.collection("athletes");
        let activities: Collection<mongodb::bson::Document> = mongo_conn.collection("activities");
        let telemetry: Collection<mongodb::bson::Document> = mongo_conn.collection("telemetry");

        Self {
            db_conn: mongo_conn,
            colls: Connections {
                athletes,
                activities,
                telemetry,
            },
        }
    }

    pub fn get_athlete_data(&self, id: i64) -> Option<AthleteData> {
        return self
            .db_conn
            .json_get::<i64, AthleteData>(id, &self.colls.athletes);
    }

    pub fn get_activity(&self, id: i64) -> Option<mongodb::bson::Document> {
        if let Some(mut activity) = self.db_conn.json_get(id, &self.colls.activities) {
            let i64_id = match activity.get("_id").unwrap() {
                Bson::Int32(_) => activity.get_i32("_id").unwrap() as i64,
                Bson::Int64(_) => activity.get_i64("_id").unwrap() as i64,
                Bson::Double(_) => activity.get_f64("_id").unwrap() as i64,

                _ => panic!("Unknown type for mapping"),
            };

            activity.remove("_id");
            activity.insert("_id", i64_id);

            return Some(activity);
        }

        return None;
    }

    pub fn get_athlete_activity_ids(&self, _id: i64) -> I64Cursor {
        self.db_conn
            .key_ids::<bson::Document>(&self.colls.activities)
    }

    pub fn activity_exists(&self, act_id: i64) -> bool {
        self.db_conn.exists(&self.colls.activities, act_id)
    }

    pub fn telemetry_exists(&self, act_id: i64) -> bool {
        self.db_conn.exists(&self.colls.telemetry, act_id)
    }

    pub fn store_athlete_activity(
        &self,
        act_id: i64,
        json: &mut serde_json::Value,
    ) -> Option<bool> {
        json["_id"] = serde_json::Value::Number(act_id.into());

        self.db_conn.json_set(&self.colls.activities, &json)
    }

    pub fn get_after_before_timestamps(&self, id: i64) -> (i64, i64) {
        let after_ts = 0;
        let before_ts = 0;

        if let Some(athlete_data) = self.db_conn.json_get(id, &self.colls.athletes) {
            return (athlete_data.after_ts, athlete_data.before_ts);
        }

        (after_ts, before_ts)
    }

    pub fn save_after_before_timestamps(
        &self,
        id: i64,
        after_ts: i64,
        before_ts: i64,
    ) -> Option<bool> {
        self.db_conn
            .json_set_field(id, &self.colls.athletes, &"before_ts", &before_ts)
            .unwrap();

        self.db_conn
            .json_set_field(id, &self.colls.athletes, &"after_ts", &after_ts)
    }

    pub fn set_athlete_data(&self, athlete_data: &AthleteData) -> Option<bool> {
        self.db_conn
            .set::<AthleteData>(&self.colls.athletes, athlete_data)
    }

    pub fn get_athlete_tokens(&self, id: i64) -> Option<AthleteTokens> {
        Some(self.get_athlete_data(id).unwrap().tokens)
    }

    pub fn set_athlete_tokens(&self, id: i64, athlete_tokens: &AthleteTokens) -> Option<bool> {
        self.db_conn.json_set_field(
            id,
            &self.colls.athletes,
            "tokens",
            &bson::to_document(athlete_tokens).unwrap(),
        )
    }

    pub fn set_activity_streams(&self, act_id: i64, json: &mut serde_json::Value) -> Option<bool> {
        json["_id"] = serde_json::Value::Number(act_id.into());

        self.db_conn.json_set(&self.colls.telemetry, json)
    }
}
