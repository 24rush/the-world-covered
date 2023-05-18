use chrono::Utc;
use curl::easy::{Easy, List};
use serde_derive::Deserialize;
use serde_json::Value;
use toml;

use crate::data_types::strava::athlete::{AthleteTokens, AthleteId};
use crate::database::strava_db::StravaDB;

use crate::{logln, logvbln};

const STRAVA_BASE_URL: &str = "https://www.strava.com/api/v3/";

#[derive(Deserialize, Debug)]
struct Secrets {
    client_id: String,
    client_secret: String,
    user_authorization_code: String,
}

pub struct StravaApi {
    athlete_id: AthleteId, 
    tokens: AthleteTokens,
    secrets: Secrets,
    persistance: StravaDB,
}

impl StravaApi {
    const CC: &str = "StravaAPI";

    fn get_refreshed_tokens(&mut self) {
        let mut handle = Easy::new();

        let header = format!(
            "client_id={}&client_secret={}&code={}&\
             grant_type=refresh_token&\
             refresh_token={}",
            self.secrets.client_id,
            self.secrets.client_secret,
            self.secrets.user_authorization_code,
            self.tokens.refresh_token
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
        logvbln!("{:?}", s);

        let new_tokens: AthleteTokens = serde_json::from_str(s.unwrap()).unwrap();
        self.tokens.access_token = new_tokens.access_token;
        self.tokens.refresh_token = new_tokens.refresh_token;
        self.tokens.expires_at = new_tokens.expires_at;
    }

    async fn get_request(&mut self, url: &str) -> Option<serde_json::Value> {
        self.refresh_tokens_if_expired().await;

        let bearer = self.get_access_token();

        let mut handle = Easy::new();
        let mut list = List::new();

        list.append(&format!("Authorization: Bearer {}", bearer))
            .unwrap();
        handle.http_headers(list).unwrap();

        handle.get(true).unwrap();

        handle.url(url).unwrap();

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

        let result = serde_json::from_str(s.unwrap());

        StravaApi::verify_if_error(result)
    }

    fn verify_if_error(
        result: Result<serde_json::Value, serde_json::Error>,
    ) -> Option<serde_json::Value> {
        if let Some(json_result) = result.ok() {
            if let Some(_) = json_result.get("errors") {
                panic!("{:?}", json_result.get("message").unwrap())
            } else {
                return Some(json_result);
            }
        } else {
            return None;
        }
    }

    fn read_secrets_from_file() -> Secrets {
        let secrets_content = std::fs::read_to_string(
            std::env::current_dir()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned()
                + "/secrets.toml",
        )
        .expect("Unable to open secrets.toml");

        toml::from_str(&secrets_content).unwrap()
    }

    fn get_access_token(&self) -> &String {
        return &self.tokens.access_token;
    }

    async fn refresh_tokens_if_expired(&mut self) {
        let current_ts: i64 = Utc::now().timestamp();

        if current_ts > self.tokens.expires_at as i64 {
            logln!("Tokens EXPIRED. Refreshing");

            self.get_refreshed_tokens();

            self.persistance
                .set_athlete_tokens(self.athlete_id, &self.tokens)
                .await;
        }
    }

    pub async fn new(athlete_id: i64) -> Option<Self> {
        let mut this = Self {
            athlete_id,
            secrets: StravaApi::read_secrets_from_file(),
            persistance: StravaDB::new().await,
            tokens: Default::default(),
        };

        if let Some(athlete_tokens) = this.persistance.get_athlete_tokens(athlete_id).await {
            this.tokens = athlete_tokens;

            return Some(this);
        }

        None
    }

    pub async fn get_activity(&mut self, act_id: i64) -> Option<serde_json::Value> {
        self.get_request(            
            &(STRAVA_BASE_URL.to_string() + &format!("activities/{}", act_id.to_string())),
        ).await
    }

    pub async fn get_activity_telemetry(&mut self, act_id: i64) -> Option<serde_json::Value> {
        self.get_request(
            &(STRAVA_BASE_URL.to_string() + &format!("activities/{}/streams?keys=time,latlng,altitude,velocity_smooth,grade_smooth,distance&key_by_type=true", act_id.to_string()))
        ).await
    }

    pub async fn get_segment(&mut self, seg_id: i64) -> Option<serde_json::Value> {
        self.get_request(
            &(STRAVA_BASE_URL.to_string() + &format!("segments/{}", seg_id.to_string())),
        ).await
    }

    pub async fn get_segment_telemetry(&mut self, seg_id: i64) -> Option<serde_json::Value> {
        self.get_request(
            &(STRAVA_BASE_URL.to_string()
                + &format!(
                    "/segments/{}/streams?keys=latlng,distance,altitude&key_by_type=true",
                    seg_id.to_string()
                )),
        ).await
    }

    pub async fn list_athlete_activities(
        &mut self,
        after_ts: i64,
        before_ts: i64,
        per_page: usize,
        page: usize,
    ) -> Option<Vec<Value>> {
        let result = self.get_request(
            &(STRAVA_BASE_URL.to_string()
                + &format!(
                    "athlete/activities?after={}&before={}&per_page={}&page={}",
                    after_ts, before_ts, per_page, page
                )),
        ).await;

        if let Some(activities) = result {
            return Some(activities.as_array().unwrap().to_vec());
        } else {
            return None;
        }
    }
}
