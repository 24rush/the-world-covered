use chrono::Utc;
use curl::easy::Easy;
use redis::JsonCommands;
use serde_derive::{Deserialize, Serialize};
use std::str;

use crate::database::redis_db::RedisConnection;

extern crate serde;
extern crate serde_json;

const STRAVA_BASE_URL: &str = "https://www.strava.com/api/v3/";
const STRAVA_CLIENT_ID: &str = "106790";

#[derive(Deserialize, Debug, Serialize, Clone)]
struct AthleteTokens {
    access_token: String,
    refresh_token: String,
    expires_at: i64,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
struct AthleteData {
    athlete_id: String,
    tokens: AthleteTokens,
}

impl AthleteData {
    fn new() -> Self {
        Self {
            athlete_id: "4399230".to_string(),
            tokens: AthleteTokens {
                access_token: "a3217525cc075e81370dd838195044a4cfeae6cd".to_string(),
                refresh_token: "ef9890be0ff863740c50fe0e829409f035cec95b".to_string(),
                expires_at: 0,
            },
        }
    }
}

struct Credentials<'a> {
    strava_client_id: &'a str,
    access_token: String,
    refresh_token: String,
    expires_at: i64,
    db_conn: &'a RedisConnection,
}

pub struct Api<'a> {
    credentials: Credentials<'a>,
}

impl<'a> Credentials<'a> {
    fn new(redis_con: &'a RedisConnection) -> Self {
        let mut athlete_data = AthleteData::new();

        if let Ok(athlete_data_str) = redis_con.json_get::<String>(&athlete_data.athlete_id, "$")
        {
            athlete_data =
                serde_json::from_str::<Vec<AthleteData>>(&athlete_data_str).unwrap()[0].clone();
        } else {
            redis_con.json_set(&athlete_data.athlete_id, "$", &athlete_data);
        }

        println!(
            "Found credentials for athlete {}:\naccess: {}\nrefresh: {}\nexpiration: {}",
            athlete_data.athlete_id,
            athlete_data.tokens.access_token,
            athlete_data.tokens.refresh_token,
            athlete_data.tokens.expires_at
        );

        let current_ts: i64 = Utc::now().timestamp();

        if current_ts > athlete_data.tokens.expires_at as i64 {
            println!("Tokens EXPIRED. Refreshing");

            Credentials::get_refreshed_tokens(STRAVA_CLIENT_ID, &mut athlete_data.tokens);
            redis_con.json_set(&athlete_data.athlete_id, "tokens", &athlete_data.tokens);
        }

        Self {
            strava_client_id: STRAVA_CLIENT_ID,
            db_conn: redis_con,
            access_token: athlete_data.tokens.access_token,
            refresh_token: athlete_data.tokens.refresh_token,
            expires_at: athlete_data.tokens.expires_at,
        }
    }

    fn get_access_token(&self) -> &String {
        return &self.access_token;
    }

    fn get_refreshed_tokens(strava_client_id: &str, tokens: &mut AthleteTokens) {
        let mut handle = Easy::new();

        let header = format!(
            "client_id={}&client_secret=3aa980b2dd77bcbbae1464ea77e808ac22672ad5&\
             grant_type=refresh_token&\
             refresh_token={}",
            strava_client_id, tokens.refresh_token
        );

        handle
            .url(&(STRAVA_BASE_URL.to_string() + "oauth/token?" + &header))
            .unwrap();

        handle.post(true).unwrap();

        let mut buffer_response = Vec::new();
        let mut transfer = handle.transfer();

        transfer
            .write_function(|data| {
                buffer_response.extend_from_slice(data);
                Ok(data.len())
            })
            .unwrap();

        transfer.perform().unwrap();
        drop(transfer);

        let s = std::str::from_utf8(&buffer_response);
        println!("{:?}", s);

        let new_tokens: AthleteTokens = serde_json::from_str(s.unwrap()).unwrap();
        tokens.access_token = new_tokens.access_token;
        tokens.refresh_token = new_tokens.refresh_token;
        tokens.expires_at = new_tokens.expires_at;
    }
}

impl<'a> Api<'a> {
    pub fn new(redis_con: &'a RedisConnection) -> Self {
        Self {
            credentials: Credentials::new(redis_con),
        }
    }

    pub fn get_activity() {

    }
}
