use mongodb::{
    bson::{self, doc},
    sync::Collection,
};

use crate::data_types::{strava:: {
    activity::{Activity},
    athlete::{AthleteData, AthleteTokens},
    telemetry::Telemetry, segment::Segment,
}, common::DocumentId};

use super::{cursors::I64Cursor, mongodb::MongoConnection};

struct StravaCollections {
    typed_athletes: Collection<AthleteData>,

    docs_activities: Collection<mongodb::bson::Document>,
    typed_activities: Collection<Activity>,

    docs_telemetry: Collection<mongodb::bson::Document>,
    typed_telemetry: Collection<Telemetry>,

    segments: Collection<mongodb::bson::Document>,
    typed_segments: Collection<Segment>,
}

pub enum ResourceType {
    Activity,
    Segment,
    Telemetry,
}

pub struct StravaDB {
    pub db_conn: MongoConnection,
    colls: StravaCollections,
}

impl StravaDB {
    pub fn new() -> Self {
        let mongo_conn = MongoConnection::new("strava_db");

        let typed_athletes: Collection<AthleteData> = mongo_conn.collection("athletes");

        let docs_activities: Collection<mongodb::bson::Document> =
            mongo_conn.collection("activities");
        let typed_activities: Collection<Activity> = mongo_conn.collection("activities");

        let docs_telemetry: Collection<mongodb::bson::Document> =
            mongo_conn.collection("telemetry");
        let typed_telemetry: Collection<Telemetry> = mongo_conn.collection("telemetry");

        let segments: Collection<mongodb::bson::Document> = mongo_conn.collection("segments");
        let typed_segments: Collection<Segment> = mongo_conn.collection("segments");

        Self {
            db_conn: mongo_conn,
            colls: StravaCollections {
                typed_athletes,

                docs_activities,
                typed_activities,

                docs_telemetry,
                typed_telemetry,

                segments,
                typed_segments
            },
        }
    }

    pub fn get_athlete_data(&self, id: i64) -> Option<AthleteData> {
        self.db_conn
            .find_one(&self.colls.typed_athletes, doc! {"_id": id})
    }

    pub fn get_activity(&self, id: i64) -> Option<Activity> {
        self.db_conn
            .find_one(&self.colls.typed_activities, doc! {"_id": id})
    }

    pub fn get_athlete_activity_ids(&self, _id: i64) -> I64Cursor {
        self.db_conn
            .keys_id::<bson::Document>(&self.colls.docs_activities)
    }

    pub fn get_athlete_activities(&self, ath_id: i64) -> mongodb::sync::Cursor<Activity> {
        self.db_conn
            .find::<Activity>(&self.colls.typed_activities, doc! {"athlete.id": ath_id})
    }
    
    pub fn get_athlete_activities_with_ids(&self, ath_id: i64, ids: &Vec<DocumentId>) -> mongodb::sync::Cursor<Activity> {
        self.db_conn
            .find::<Activity>(&self.colls.typed_activities, doc! {"athlete.id": ath_id, "_id": {"$in": ids}})
    }
    
    pub fn get_telemetry_by_id(&self, id: i64) -> Option<Telemetry> {
        self.db_conn
            .find_one(&self.colls.typed_telemetry, doc! {"_id": id})
    }

    pub fn get_telemetry(&self, ath_id: i64) -> mongodb::sync::Cursor<Telemetry> {
        self.db_conn
            .find::<Telemetry>(&self.colls.typed_telemetry, doc! {"athlete.id": ath_id})
    }

    pub fn get_telemetry_by_type(&self, ath_id: i64, r#type: &str) -> mongodb::sync::Cursor<Telemetry> {
        self.db_conn
            .find::<Telemetry>(&self.colls.typed_telemetry, doc! {"athlete.id": ath_id, "type": r#type})
    }

    pub fn get_segment(&self, seg_id: i64) -> Option<Segment> {
        self.db_conn
            .find_one::<Segment>(&self.colls.typed_segments, doc! {"_id": seg_id})
    }

    pub fn exists_resource(&self, res_type: ResourceType, res_id: i64) -> bool {
        match res_type {
            ResourceType::Activity => self.db_conn.exists(&self.colls.docs_activities, res_id),
            ResourceType::Segment => self.db_conn.exists(&self.colls.segments, res_id),
            ResourceType::Telemetry => self.db_conn.exists(&self.colls.docs_telemetry, res_id),
        }
    }

    pub fn save_after_before_timestamps(
        &self,
        id: i64,
        after_ts: i64,
        before_ts: i64,
    ) -> Option<bool> {
        self.db_conn
            .update_field(id, &self.colls.typed_athletes, &"before_ts", &before_ts)
            .unwrap();

        self.db_conn
            .update_field(id, &self.colls.typed_athletes, &"after_ts", &after_ts)
    }

    pub fn set_athlete_data(&self, athlete_data: &AthleteData) -> Option<bool> {
        self.db_conn
            .upsert_one::<AthleteData>(&self.colls.typed_athletes, athlete_data)
    }

    pub fn get_athlete_tokens(&self, id: i64) -> Option<AthleteTokens> {
        Some(self.get_athlete_data(id).unwrap().tokens)
    }

    pub fn set_athlete_tokens(&self, id: i64, athlete_tokens: &AthleteTokens) -> Option<bool> {
        self.db_conn.update_field(
            id,
            &self.colls.typed_athletes,
            "tokens",
            &bson::to_document(athlete_tokens).unwrap(),
        )
    }

    pub fn store_resource(
        &self,
        res_type: ResourceType,
        res_id: i64,
        json: &mut serde_json::Value,
    ) -> Option<bool> {
        json["_id"] = serde_json::Value::Number(res_id.into());

        match res_type {
            ResourceType::Activity => self.db_conn.upsert_one_raw(&self.colls.docs_activities, &json),
            ResourceType::Segment => self.db_conn.upsert_one_raw(&self.colls.segments, &json),
            ResourceType::Telemetry => self.db_conn.upsert_one_raw(&self.colls.docs_telemetry, &json),
        }
    }
}
