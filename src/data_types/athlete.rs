use chrono::Utc;
use serde_derive::{Deserialize, Serialize};

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct AthleteData {    
    pub tokens: AthleteTokens,
    pub before_ts: i64,
    pub after_ts: i64,
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct AthleteTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

impl AthleteData {
    pub fn new() -> Self {
        Self {            
            before_ts: Utc::now().timestamp(),
            after_ts: 0,
            tokens: AthleteTokens {
                access_token: "168f77da961a3d36941a6f63a52570f6c007ebf6".to_string(),
                refresh_token: "1448cdba35873b1db89f6001afae1eeff6dd972a".to_string(),
                expires_at: 0,                
            },
        }
    }
}