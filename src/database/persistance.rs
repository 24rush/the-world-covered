use redis::Commands;

use crate::{
    data_types::athlete::{AthleteData, AthleteTokens},
    database::redis::redis_db::RedisConnection,
};

pub struct Persistance {
    db_conn: RedisConnection,
}

impl Persistance {
    pub fn new() -> Self {
        Self {
            db_conn: RedisConnection::new(),
        }
    }

    fn fmt_key_athlete_id(id: &str) -> String {
        format!("athlete:{}", id)
    }
    fn fmt_key_athlete_activity_id(id: &str, act_id: &str) -> String {
        format!("athlete:{}:activity:{}", id, act_id)
    }
    fn fmt_key_athlete_activity_id_telemetry(id: &str, act_id: &str) -> String {
        format!("athlete:{}:activity:{}:telemetry", id, act_id)
    }
    fn fmt_query_athlete_activity_ids(id: &str) -> String {
        format!("athlete:{}:activity:*", id)
    }

    pub fn get_athlete_data(&self, id: &String) -> Option<AthleteData> {
        if let Ok(athlete_data_str) = self
            .db_conn
            .json_get::<String>(&Persistance::fmt_key_athlete_id(id), "$")
        {
            return Some(
                serde_json::from_str::<Vec<AthleteData>>(&athlete_data_str).unwrap()[0].clone(),
            );
        }

        return None;
    }

    pub fn get_athlete_activity_ids(&self, id: &String) -> Vec<String> {
        let result = self
            .db_conn
            .connection()
            .keys(Persistance::fmt_query_athlete_activity_ids(id));

        result.unwrap()
    }

    pub fn activity_exists(&self, id: &String, act_id: &String) -> bool {
        self.db_conn
            .exists(&Persistance::fmt_key_athlete_activity_id(id, act_id))
    }

    pub fn telemetry_exists(&self, id: &str, act_id: &str) -> bool {
        self.db_conn
            .exists(&&Persistance::fmt_key_athlete_activity_id_telemetry(id, act_id))
    }

    pub fn store_athlete_activity(
        &self,
        id: &str,
        act_id: &str,
        json: &serde_json::Value,
    ) -> Option<bool> {
        self.db_conn
            .json_set(
                &Persistance::fmt_key_athlete_activity_id(id, act_id),
                "$",
                json,
            )
            .ok()
    }

    pub fn get_after_before_timestamps(&self, id: &String) -> (i64, i64) {
        let after_ts = self
            .db_conn
            .json_get::<i64>(&Persistance::fmt_key_athlete_id(id), "after_ts")
            .unwrap();

        let before_ts = self
            .db_conn
            .json_get::<i64>(&Persistance::fmt_key_athlete_id(id), "before_ts")
            .unwrap();

        (after_ts, before_ts)
    }

    pub fn save_after_before_timestamps(
        &self,
        id: &String,
        after_ts: i64,
        before_ts: i64,
    ) -> Option<bool> {
        self.db_conn
            .json_set::<i64, bool>(
                &Persistance::fmt_key_athlete_id(id),
                &"before_ts",
                &before_ts,
            )
            .unwrap();

        self.db_conn
            .json_set::<i64, bool>(&Persistance::fmt_key_athlete_id(id), &"after_ts", &after_ts)
            .ok()
    }

    pub fn set_athlete_data(&self, id: &String, athlete_data: &AthleteData) -> Option<bool> {
        self.db_conn
            .json_set(&Persistance::fmt_key_athlete_id(id), "$", athlete_data)
            .ok()
    }

    pub fn get_athlete_tokens(&self, id: &String) -> Option<AthleteTokens> {
        if let Ok(athlete_data_str) = self
            .db_conn
            .json_get::<String>(&Persistance::fmt_key_athlete_id(id), "tokens")
        {
            return Some(
                serde_json::from_str::<AthleteTokens>(&athlete_data_str)
                    .unwrap()
                    .clone(),
            );
        }

        return None;
    }

    pub fn set_athlete_tokens(&self, id: &String, athlete_tokens: &AthleteTokens) -> Option<bool> {
        self.db_conn
            .json_set(
                &Persistance::fmt_key_athlete_id(id),
                "tokens",
                athlete_tokens,
            )
            .ok()
    }

    pub fn set_activity_streams(
        &self,
        id: &str,
        act_id: &str,
        json: &serde_json::Value,
    ) -> Option<bool> {
        
        let res = self.db_conn
        .json_set(
            &Persistance::fmt_key_athlete_activity_id_telemetry(id, act_id),
            "$",
            json,
        );

        println!("{} - {} {:?}", id, act_id, res);

        res.ok()
        
    }
}
