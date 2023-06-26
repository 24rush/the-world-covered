use std::borrow::Borrow;

use crate::data_types::{
    common::{DocumentId, Identifiable},
    strava::{
        activity::Activity,
        athlete::{AthleteData, AthleteTokens},
        telemetry::Telemetry,
    },
};
use mongodb::{
    bson::{self, doc, DateTime},
    Client, Collection,
};

use super::mongodb::MongoDatabase;

pub struct ActivitiesCollection {
    db_conn: MongoDatabase,
}

pub struct TelemetriesCollection {
    db_conn: MongoDatabase,
}

pub struct AthletesCollection {
    db_conn: MongoDatabase,
}

impl ActivitiesCollection {
    const COLL_NAME: &str = "activities";

    pub fn new(db_conn: &MongoDatabase) -> Self {
        Self {
            db_conn: db_conn.clone(),
        }
    }

    fn typed_collection(&self) -> Collection<Activity> {
        self.db_conn.typed_collection(ActivitiesCollection::COLL_NAME)
    }

    fn raw_collection(&self) -> Collection<mongodb::bson::Document> {
        self.db_conn.typed_collection(ActivitiesCollection::COLL_NAME)
    }

    // GETTERS
    pub async fn get(&self, id: i64) -> Option<Activity> {
        self.typed_collection()
            .find_one(doc! {"_id": id}, None)
            .await
            .ok()
            .unwrap()
    }

    pub async fn get_athlete_activities(&self, ath_id: i64) -> Option<mongodb::Cursor<Activity>> {
        self.typed_collection()
            .find(doc! {"athlete.id": ath_id}, None)
            .await
            .ok()
    }

    pub async fn get_athlete_activities_with_ids(
        &self,
        ath_id: i64,
        ids: &Vec<DocumentId>,
    ) -> Option<mongodb::Cursor<Activity>> {
        self.typed_collection()
            .find(doc! {"athlete.id": ath_id, "_id": {"$in": ids}}, None)
            .await
            .ok()
    }

    pub async fn get_athlete_activity_ids(&self, ath_id: i64) -> Vec<DocumentId> {
        let mut act_ids: Vec<DocumentId> = Vec::new();

        if let Some(mut cursor) = self.get_athlete_activities(ath_id).await {
            while cursor.advance().await.unwrap() {
                act_ids.push(cursor.deserialize_current().unwrap().as_i64());
            }
        }

        act_ids
    }

    pub async fn get_athlete_activities_sorted_distance_asc(
        &self,
        ath_id: i64,
    ) -> Option<mongodb::Cursor<mongodb::bson::Document>> {
        self.typed_collection()
            .aggregate(
                vec![
                    doc! {"$match": {"athlete.id": ath_id}},
                    doc! {"$sort": { "distance": 1 } },
                ],
                None,
            )
            .await
            .ok()
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

    // SETTERS
    pub async fn set_location_city(&self, act_id: i64, city: &Option<String>) {
        self.update("_id".to_owned(), act_id, "location_city", &city)
            .await;
    }

    pub async fn set_location_country(&self, act_id: i64, country: &String) {
        self.update("_id".to_owned(), act_id, "location_country", &country)
            .await;
    }

    pub async fn set_segment_start_index_poly(&self, seg_id: i64, start_index_poly: &Option<i32>) {
        if let Some(start_index) = start_index_poly {
            self.update(
                "segment_efforts.id".to_owned(),
                seg_id,
                "segment_efforts.$.start_index_poly",
                &start_index,
            )
            .await;
        }
    }

    pub async fn set_segment_end_index_poly(&self, seg_id: i64, start_index_poly: &Option<i32>) {
        if let Some(start_index) = start_index_poly {
            self.update(
                "segment_efforts.id".to_owned(),
                seg_id,
                "segment_efforts.$.start_end_poly",
                &start_index,
            )
            .await;
        }
    }

    pub async fn set_segment_distance_from_start(&self, seg_id: i64, distance_from_start: f32) {
        self.update(
            "segment_efforts.id".to_owned(),
            seg_id,
            "segment_efforts.$.distance_from_start",
            &distance_from_start,
        )
        .await;
    }

    pub async fn set_start_date_local_date(
        &self,
        seg_id: i64,
        start_index_poly: &Option<DateTime>,
    ) {
        if let Some(start_index) = start_index_poly {
            self.update(
                "_id".to_owned(),
                seg_id,
                "start_date_local_date",
                &start_index,
            )
            .await;
        }
    }

    pub async fn query_activities(&self, stages: Vec<bson::Document>) -> Vec<Activity> {
        self.db_conn
            .query(&self.typed_collection(), stages)
            .await
            .to_owned()
    }

    pub async fn query_activities_docs(&self, stages: Vec<bson::Document>) -> Vec<bson::Document> {
        self.db_conn
            .query(&self.raw_collection(), stages)
            .await
            .to_owned()
    }

    pub async fn exists(&self, act_id: i64) -> bool {
        self.db_conn
            .exists(&self.raw_collection(), act_id)
            .await
    }

    pub async fn store(&self, act_id: i64, json: &mut serde_json::Value) {
        json["_id"] = serde_json::Value::Number(act_id.into());

        self.db_conn
            .upsert_one(
                &self.raw_collection(),
                json["_id"].as_i64().unwrap(),
                bson::to_document(&json).unwrap().borrow(),
            )
            .await;
    }

    pub async fn update<KT, V>(&self, key_path: String, key_value: KT, field: &str, value: &V)
    where
        V: std::clone::Clone + Into<bson::Bson>,
        KT: std::clone::Clone + Into<bson::Bson>,
        bson::Bson: From<KT> + From<V>,
    {
        self.db_conn
            .update_field(
                key_path,
                &key_value,
                &self.typed_collection(),
                field,
                &value,
            )
            .await;
    }
}

impl TelemetriesCollection {
    const COLL_NAME: &str = "telemetry";

    pub fn new(db_conn: &MongoDatabase) -> Self {
        Self {
            db_conn: db_conn.clone(),
        }
    }

    fn typed_collection(&self) -> Collection<Telemetry> {
        self.db_conn.typed_collection(TelemetriesCollection::COLL_NAME)
    }

    fn raw_collection(&self) -> Collection<mongodb::bson::Document> {
        self.db_conn.typed_collection(TelemetriesCollection::COLL_NAME)
    }

    pub async fn get(&self, id: i64) -> Option<Telemetry> {
        self.typed_collection()
            .find_one(doc! {"_id": id}, None)
            .await
            .ok()
            .unwrap()
    }

    pub async fn exists(&self, act_id: i64) -> bool {
        self.db_conn
            .exists(&self.raw_collection(), act_id)
            .await
    }

    pub async fn store(&self, act_id: i64, json: &mut serde_json::Value) {
        json["_id"] = serde_json::Value::Number(act_id.into());

        self.db_conn
            .upsert_one(
                &self.raw_collection(),
                json["_id"].as_i64().unwrap(),
                bson::to_document(&json).unwrap().borrow(),
            )
            .await;
    }
}

impl AthletesCollection {
    pub fn new(db_conn: &MongoDatabase) -> Self {
        Self {
            db_conn: db_conn.clone(),
        }
    }

    fn typed_docs_collection(&self) -> Collection<AthleteData> {
        self.db_conn.typed_collection("athletes")
    }

    pub async fn get_athlete_data(&self, id: i64) -> Option<AthleteData> {
        self.typed_docs_collection()
            .find_one(doc! {"_id": id}, None)
            .await
            .ok()
            .unwrap()
    }

    pub async fn set_athlete_data(&self, athlete_data: &AthleteData) {
        self.db_conn
            .upsert_one::<AthleteData>(
                &self.typed_docs_collection(),
                athlete_data.as_i64(),
                athlete_data,
            )
            .await;
    }

    pub async fn set_athlete_tokens(&self, id: i64, athlete_tokens: &AthleteTokens) {
        self.db_conn
            .update_field(
                "_id".to_owned(),
                id,
                &self.typed_docs_collection(),
                "tokens",
                &bson::to_document(athlete_tokens).unwrap(),
            )
            .await;
    }

    pub async fn save_after_before_timestamps(&self, ath_id: i64, after_ts: i64, before_ts: i64) {
        self.db_conn
            .update_field(
                "_id".to_owned(),
                ath_id,
                &self.typed_docs_collection(),
                &"before_ts",
                &before_ts,
            )
            .await;

        self.db_conn
            .update_field(
                "_id".to_owned(),
                ath_id,
                &self.typed_docs_collection(),
                &"after_ts",
                &after_ts,
            )
            .await;
    }
}

pub struct StravaDB {
    db_conn: MongoDatabase,

    pub activities: ActivitiesCollection,
    pub telemetries: TelemetriesCollection,
    pub athletes: AthletesCollection,
}

impl StravaDB {
    pub async fn new(db_url: &str) -> Self {
        let db = Client::with_uri_str(db_url)
            .await
            .unwrap()
            .database("strava_db");

        let db_coll = MongoDatabase::new(&db);

        Self {
            db_conn: db_coll.clone(),
            activities: ActivitiesCollection::new(&db_coll),
            telemetries: TelemetriesCollection::new(&db_coll),
            athletes: AthletesCollection::new(&db_coll),
        }
    }

    pub fn get_athletes_collection(&self) -> AthletesCollection {
        AthletesCollection::new(&self.db_conn)
    }
}
