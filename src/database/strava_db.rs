use futures_util::{TryStreamExt};
use mongodb::{
    bson::{self, doc}, Collection,
};

use crate::data_types::{strava:: {
    activity::{Activity},
    athlete::{AthleteData, AthleteTokens},
    telemetry::Telemetry, segment::Segment,
}, common::DocumentId};

use super::{mongodb::MongoConnection};

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
    pub async fn new() -> Self {
        let mongo_conn = MongoConnection::new("strava_db").await;

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

    pub async fn get_athlete_data(&self, id: i64) -> Option<AthleteData> {
        self.db_conn
            .find_one(&self.colls.typed_athletes, doc! {"_id": id}).await
    }

    pub async fn get_activity(&self, id: i64) -> Option<Activity> {
        self.db_conn
            .find_one(&self.colls.typed_activities, doc! {"_id": id}).await
    }

    pub async fn get_athlete_activity_ids(&self, ath_id: i64) -> Vec<DocumentId> {
        let mut cursor = self.db_conn
            .find(&self.colls.typed_activities, doc! {"athlete.id": ath_id}).await;

        let mut act_ids : Vec<DocumentId> = Vec::new();

        while let Some(act) = cursor.try_next().await.unwrap() {
            act_ids.push(act._id as i64);
        }

        act_ids
    }

    pub async fn get_athlete_activities(&self, ath_id: i64) -> mongodb::Cursor<Activity> {
        self.db_conn
            .find::<Activity>(&self.colls.typed_activities, doc! {"athlete.id": ath_id}).await
    }
    
    pub async fn get_athlete_activities_with_ids(&self, ath_id: i64, ids: &Vec<DocumentId>) -> mongodb::Cursor<Activity> {
        self.db_conn
            .find::<Activity>(&self.colls.typed_activities, doc! {"athlete.id": ath_id, "_id": {"$in": ids}}).await
    }
    
    pub async fn get_telemetry_by_id(&self, id: i64) -> Option<Telemetry> {
        self.db_conn
            .find_one(&self.colls.typed_telemetry, doc! {"_id": id}).await
    }

    pub async fn get_telemetry(&self, ath_id: i64) -> mongodb::Cursor<Telemetry> {
        self.db_conn
            .find::<Telemetry>(&self.colls.typed_telemetry, doc! {"athlete.id": ath_id}).await
    }

    pub async fn get_telemetry_by_type(&self, ath_id: i64, r#type: &str) -> mongodb::Cursor<Telemetry> {
        self.db_conn
            .find::<Telemetry>(&self.colls.typed_telemetry, doc! {"athlete.id": ath_id, "type": r#type}).await
    }

    pub async fn get_segment(&self, seg_id: i64) -> Option<Segment> {
        self.db_conn
            .find_one::<Segment>(&self.colls.typed_segments, doc! {"_id": seg_id}).await
    }

    pub async fn exists_resource(&self, res_type: ResourceType, res_id: i64) -> bool {
        match res_type {
            ResourceType::Activity => self.db_conn.exists(&self.colls.docs_activities, res_id).await,
            ResourceType::Segment => self.db_conn.exists(&self.colls.segments, res_id).await,
            ResourceType::Telemetry => self.db_conn.exists(&self.colls.docs_telemetry, res_id).await,
        }
    }

    pub async fn save_after_before_timestamps(
        &self,
        id: i64,
        after_ts: i64,
        before_ts: i64,
    ) -> Option<bool> {
        self.db_conn
            .update_field(id, &self.colls.typed_athletes, &"before_ts", &before_ts).await
            .unwrap();

        self.db_conn
            .update_field(id, &self.colls.typed_athletes, &"after_ts", &after_ts).await
    }

    pub async fn set_athlete_data(&self, athlete_data: &AthleteData) -> Option<bool> {
        self.db_conn
            .upsert_one::<AthleteData>(&self.colls.typed_athletes, athlete_data).await
    }

    pub async fn get_athlete_tokens(&self, id: i64) -> Option<AthleteTokens> {
        if let Some(athlete_data) = self.get_athlete_data(id).await {
            return Some(athlete_data.tokens);
        }

        None
    }

    pub async fn set_athlete_tokens(&self, id: i64, athlete_tokens: &AthleteTokens) -> Option<bool> {
        self.db_conn.update_field(
            id,
            &self.colls.typed_athletes,
            "tokens",
            &bson::to_document(athlete_tokens).unwrap(),
        ).await
    }

    pub async fn store_resource(
        &self,
        res_type: ResourceType,
        res_id: i64,
        json: &mut serde_json::Value,
    ) -> Option<bool> {
        json["_id"] = serde_json::Value::Number(res_id.into());

        match res_type {
            ResourceType::Activity => self.db_conn.upsert_one_raw(&self.colls.docs_activities, &json).await,
            ResourceType::Segment => self.db_conn.upsert_one_raw(&self.colls.segments, &json).await,
            ResourceType::Telemetry => self.db_conn.upsert_one_raw(&self.colls.docs_telemetry, &json).await,
        }
    }
}
