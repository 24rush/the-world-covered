use serde_json::Value;
use curl::easy::{Easy, List};

use crate::data_types::athlete::AthleteTokens;
use crate::logvbln;
use crate::strava::auth::StravaAuth;

const STRAVA_BASE_URL: &str = "https://www.strava.com/api/v3/";

pub struct StravaApi {    
    auth: StravaAuth,
}

impl StravaApi {
    const CC: &str = "StravaAPI";
    
    pub fn get_refreshed_tokens(
        client_id: &str,
        client_secret: &str,
        user_authorization_code: &str,
        tokens: &mut AthleteTokens,
    ) {
        let mut handle = Easy::new();

        let header = format!(
            "client_id={}&client_secret={}&code={}&\
             grant_type=refresh_token&\
             refresh_token={}",
            client_id, client_secret, user_authorization_code, tokens.refresh_token
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
        tokens.access_token = new_tokens.access_token;
        tokens.refresh_token = new_tokens.refresh_token;
        tokens.expires_at = new_tokens.expires_at;
    }

    fn get_request(bearer: &str, url: &str) -> Option<serde_json::Value> {
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

    pub fn authenticate_athlete(athlete_id: i64) -> Self {        
        Self {    
            auth : StravaAuth::new(athlete_id),            
        }
    }

    pub fn get_activity(&self, act_id: i64) -> Option<serde_json::Value> {
        StravaApi::get_request(
            &self.auth.get_access_token(),
            &(STRAVA_BASE_URL.to_string() + &format!("activities/{}", act_id.to_string()))
        )
    }

    pub fn get_activity_telemetry(&self, act_id: i64) -> Option<serde_json::Value> {
        StravaApi::get_request(
            &self.auth.get_access_token(),
            &(STRAVA_BASE_URL.to_string() + &format!("activities/{}/streams?keys=time,latlng,altitude,velocity_smooth,grade_smooth,distance&key_by_type=true", act_id.to_string()))
        )
    }

    pub fn get_segment(&self, seg_id: i64) -> Option<serde_json::Value> {
        StravaApi::get_request(
            &self.auth.get_access_token(),
            &(STRAVA_BASE_URL.to_string() + &format!("segments/{}", seg_id.to_string()))
        )
    }

    pub fn get_segment_telemetry(&self, seg_id: i64) -> Option<serde_json::Value> {
        StravaApi::get_request(
            &self.auth.get_access_token(),
            &(STRAVA_BASE_URL.to_string() + &format!("/segments/{}/streams?keys=latlng,distance,altitude&key_by_type=true", seg_id.to_string()))
        )
    }

    pub fn list_athlete_activities(&self, after_ts: i64, before_ts: i64, per_page:usize, page: usize) -> Option<Vec<Value>> {         
        let result = StravaApi::get_request(
            &self.auth.get_access_token(),
            &(STRAVA_BASE_URL.to_string()
                + &format!(
                    "athlete/activities?after={}&before={}&per_page={}&page={}",
                    after_ts, before_ts, per_page, page
                )),
        );

        if let Some(activities) = result {     
            return Some(activities.as_array().unwrap().to_vec())
        } else {
            return None;
        }
    }
}