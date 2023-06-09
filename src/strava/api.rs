
use chrono::Utc;
use curl::easy::{Easy, List};
use serde_derive::Deserialize;
use serde_json::Value;
use toml;

use crate::data_types::strava::athlete::{AthleteId, AthleteTokens};
use crate::{logln, logvbln, TokenExchange};

const STRAVA_BASE_URL: &str = "https://www.strava.com/api/v3/";

#[derive(Deserialize, Debug)]
struct Secrets {
    client_id: String,
    client_secret: String,
    user_authorization_code: String,
}

pub struct StravaApi {
    athlete_id: AthleteId,
    token_exchange: TokenExchange,
    secrets: Secrets,
}

impl StravaApi {
    const CC: &str = "StravaAPI";

    fn get_refreshed_tokens(&self) -> AthleteTokens {
        let mut handle = Easy::new();

        let header = format!(
            "client_id={}&client_secret={}&code={}&\
             grant_type=refresh_token&\
             refresh_token={}",
            self.secrets.client_id,
            self.secrets.client_secret,
            self.secrets.user_authorization_code,

            self.token_exchange.get_tokens().refresh_token
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

        serde_json::from_str(s.unwrap()).unwrap()
    }

    async fn get_request(&self, url: &str) -> Option<serde_json::Value> {
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

    fn get_access_token(&self) -> String {
        return self.token_exchange.get_tokens().access_token.clone();
    }

    async fn refresh_tokens_if_expired(&self) {
        let current_ts: i64 = Utc::now().timestamp();

        if current_ts > self.token_exchange.get_tokens().expires_at as i64 {
            logln!("Tokens EXPIRED. Refreshing");

            let new_tokens = self.get_refreshed_tokens();            
            self.token_exchange.set_tokens(&new_tokens).await;
        }
    }

    pub fn new(
        token_exchange: TokenExchange,
        athlete_id: i64,
    ) -> Self {
        Self {
            athlete_id,
            token_exchange,
            secrets: StravaApi::read_secrets_from_file(),
        }
    }

    pub async fn get_activity(&self, act_id: i64) -> Option<serde_json::Value> {
        self.get_request(
            &(STRAVA_BASE_URL.to_string() + &format!("activities/{}", act_id.to_string())),
        )
        .await
    }

    pub async fn get_activity_telemetry(&self, act_id: i64) -> Option<serde_json::Value> {
        self.get_request(
            &(STRAVA_BASE_URL.to_string() + &format!("activities/{}/streams?keys=time,latlng,altitude,velocity_smooth,grade_smooth,distance&key_by_type=true", act_id.to_string()))
        ).await
    }

    pub async fn get_segment(&self, seg_id: i64) -> Option<serde_json::Value> {
        self.get_request(
            &(STRAVA_BASE_URL.to_string() + &format!("segments/{}", seg_id.to_string())),
        )
        .await
    }

    pub async fn get_segment_telemetry(&self, seg_id: i64) -> Option<serde_json::Value> {
        self.get_request(
            &(STRAVA_BASE_URL.to_string()
                + &format!(
                    "/segments/{}/streams?keys=latlng,distance,altitude&key_by_type=true",
                    seg_id.to_string()
                )),
        )
        .await
    }

    pub async fn list_athlete_activities(
        &self,
        after_ts: i64,
        before_ts: i64,
        per_page: usize,
        page: usize,
    ) -> Option<Vec<Value>> {
        let result = self
            .get_request(
                &(STRAVA_BASE_URL.to_string()
                    + &format!(
                        "athlete/activities?after={}&before={}&per_page={}&page={}",
                        after_ts, before_ts, per_page, page
                    )),
            )
            .await;

        if let Some(activities) = result {
            return Some(activities.as_array().unwrap().to_vec());
        } else {
            return None;
        }
    }
}
