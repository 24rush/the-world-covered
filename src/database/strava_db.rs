use mongodb::{
    bson::{self, doc},
    Collection,
};

use crate::data_types::{
    common::{DocumentId, Identifiable},
    strava::{
        activity::{Activity, Effort},
        athlete::{AthleteData, AthleteTokens},
        segment::Segment,
        telemetry::Telemetry,
    },
};

use super::mongodb::MongoConnection;

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
                typed_segments,
            },
        }
    }

    pub async fn get_athlete_data(&self, id: i64) -> Option<AthleteData> {
        self.db_conn
            .find_one(&self.colls.typed_athletes, doc! {"_id": id})
            .await
    }

    pub async fn query_activities(&self, stages: Vec<bson::Document>) -> Vec<Activity> {
        let mut act_ids: Vec<Activity> = Vec::new();

        let mut cursor = self
            .db_conn
            .aggregate(&self.colls.typed_activities, stages)
            .await;

        while cursor.advance().await.unwrap() {
            let doc = cursor.deserialize_current().unwrap();

            let mut activity : Activity = bson::from_bson(bson::Bson::Document(doc)).unwrap();

            // Manually fill in location city and country from efforts if not present
            if let None = activity.location_city {
                if let Some(effort) = activity.segment_efforts.get(0) {
                    if let Some(effort_city) = &effort.segment.city {
                        activity.location_city = Some(effort_city.to_string());
                    }
                }
            }

            if activity.location_country == "" {
                if let Some(effort) = activity.segment_efforts.get(0) {
                    if let Some(effort_country) = &effort.segment.country {
                        activity.location_country = effort_country.to_string();
                    }
                }
            }

            act_ids.push(activity);
        }

        act_ids
    }

    pub async fn get_activity(&self, id: i64) -> Option<Activity> {
        self.db_conn
            .find_one(&self.colls.typed_activities, doc! {"_id": id})
            .await
    }

    pub async fn get_athlete_activity_ids(&self, ath_id: i64) -> Vec<DocumentId> {
        let mut cursor = self
            .db_conn
            .find(&self.colls.typed_activities, doc! {"athlete.id": ath_id})
            .await;

        let mut act_ids: Vec<DocumentId> = Vec::new();

        while cursor.advance().await.unwrap() {
            act_ids.push(cursor.deserialize_current().unwrap().as_i64());
        }

        act_ids
    }

    pub async fn get_athlete_activities(&self, ath_id: i64) -> mongodb::Cursor<Activity> {
        self.db_conn
            .find::<Activity>(&self.colls.typed_activities, doc! {"athlete.id": ath_id})
            .await
    }

    pub async fn get_athlete_activities_with_ids(
        &self,
        ath_id: i64,
        ids: &Vec<DocumentId>,
    ) -> mongodb::Cursor<Activity> {
        self.db_conn
            .find::<Activity>(
                &self.colls.typed_activities,
                doc! {"athlete.id": ath_id, "_id": {"$in": ids}},
            )
            .await
    }

    pub async fn get_max_distance_activity_in_ids(
        &self,
        ids: &Vec<DocumentId>,
    ) -> Option<Activity> {
        self.query_activities(Vec::from([
            doc! {"$match": {"_id": {"$in": ids}}},
            doc! {"$sort": { "distance": -1 } },
            doc! {"$limit": 1},
        ]))
        .await
        .get(0)
        .cloned()
    }

    pub async fn get_telemetry_by_id(&self, id: i64) -> Option<Telemetry> {
        self.db_conn
            .find_one(&self.colls.typed_telemetry, doc! {"_id": id})
            .await
    }

    pub async fn get_athlete_telemetries(&self, ath_id: i64) -> mongodb::Cursor<Telemetry> {
        self.db_conn
            .find::<Telemetry>(&self.colls.typed_telemetry, doc! {"athlete.id": ath_id})
            .await
    }

    pub async fn get_telemetry_by_type(
        &self,
        ath_id: i64,
        r#type: &str,
    ) -> mongodb::Cursor<Telemetry> {
        self.db_conn
            .find::<Telemetry>(
                &self.colls.typed_telemetry,
                doc! {"athlete.id": ath_id, "type": r#type},
            )
            .await
    }

    pub async fn get_segment(&self, seg_id: i64) -> Option<Segment> {
        self.db_conn
            .find_one::<Segment>(&self.colls.typed_segments, doc! {"_id": seg_id})
            .await
    }

    pub async fn exists_resource(&self, res_type: ResourceType, res_id: i64) -> bool {
        match res_type {
            ResourceType::Activity => {
                self.db_conn
                    .exists(&self.colls.docs_activities, res_id)
                    .await
            }
            ResourceType::Segment => self.db_conn.exists(&self.colls.segments, res_id).await,
            ResourceType::Telemetry => {
                self.db_conn
                    .exists(&self.colls.docs_telemetry, res_id)
                    .await
            }
        }
    }

    pub async fn save_after_before_timestamps(
        &self,
        ath_id: i64,
        after_ts: i64,
        before_ts: i64,
    ) -> Option<bool> {
        self.db_conn
            .update_field_of_doc_id(ath_id, &self.colls.typed_athletes, &"before_ts", &before_ts)
            .await
            .unwrap();

        self.db_conn
            .update_field_of_doc_id(ath_id, &self.colls.typed_athletes, &"after_ts", &after_ts)
            .await
    }

    pub async fn set_athlete_data(&self, athlete_data: &AthleteData) -> Option<bool> {
        self.db_conn
            .upsert_one::<AthleteData>(&self.colls.typed_athletes, athlete_data)
            .await
    }

    pub async fn update_activity(&self, activity: &Activity) -> Option<bool> {
        self.db_conn
            .upsert_one::<Activity>(&self.colls.typed_activities, activity)
            .await
    }

    pub async fn update_segment_effort_start_end_poly_indexes(
        &self,
        effort: &Effort,
    ) -> Option<bool> {
        self.db_conn
            .update_field(
                "segment_efforts.id".to_owned(),
                effort.id,
                &self.colls.typed_activities,
                "segment_efforts.$.start_index_poly",
                &effort.start_index_poly,
            )
            .await
            .unwrap();

        self.db_conn
            .update_field(
                "segment_efforts.id".to_owned(),
                effort.id,
                &self.colls.typed_activities,
                "segment_efforts.$.end_index_poly",
                &effort.end_index_poly,
            )
            .await
    }

    pub async fn get_athlete_tokens(&self, id: i64) -> Option<AthleteTokens> {
        if let Some(athlete_data) = self.get_athlete_data(id).await {
            return Some(athlete_data.tokens);
        }

        None
    }

    pub async fn set_athlete_tokens(
        &self,
        id: i64,
        athlete_tokens: &AthleteTokens,
    ) -> Option<bool> {
        self.db_conn
            .update_field_of_doc_id(
                id,
                &self.colls.typed_athletes,
                "tokens",
                &bson::to_document(athlete_tokens).unwrap(),
            )
            .await
    }

    pub async fn store_resource(
        &self,
        res_type: ResourceType,
        res_id: i64,
        json: &mut serde_json::Value,
    ) -> Option<bool> {
        json["_id"] = serde_json::Value::Number(res_id.into());

        match res_type {
            ResourceType::Activity => {
                self.db_conn
                    .upsert_one_raw(&self.colls.docs_activities, &json)
                    .await
            }
            ResourceType::Segment => {
                self.db_conn
                    .upsert_one_raw(&self.colls.segments, &json)
                    .await
            }
            ResourceType::Telemetry => {
                self.db_conn
                    .upsert_one_raw(&self.colls.docs_telemetry, &json)
                    .await
            }
        }
    }
}
