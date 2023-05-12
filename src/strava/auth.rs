use chrono::Utc;
use serde_derive::Deserialize;
use toml;

use crate::{data_types::athlete::{AthleteData, AthleteTokens}, database::persistance::{Persistance}, strava::api::Api};

#[derive(Deserialize, Debug)]
struct Secrets {
    client_id: String,
    client_secret: String,
    user_authorization_code: String,
}

pub struct Auth {
    athlete_id: i64,
    athlete_tokens: AthleteTokens,
    secrets: Secrets,
    persistance: Persistance
}

impl Auth {
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

    fn refresh_tokens_if_expired(&mut self) {
        let current_ts: i64 = Utc::now().timestamp();

        if current_ts > self.athlete_tokens.expires_at as i64 {
            println!("Tokens EXPIRED. Refreshing");

            Api::get_refreshed_tokens(
                &self.secrets.client_id,
                &self.secrets.client_secret,
                &self.secrets.user_authorization_code,
                &mut self.athlete_tokens,
            );

            self.persistance.set_athlete_tokens(self.athlete_id, &self.athlete_tokens);
        }
    }

    pub fn new(athlete_id: i64) -> Self {        
        // Start with defaults
        let mut this = Self {
            athlete_id,
            athlete_tokens: AthleteData::new(athlete_id).tokens,
            secrets: Auth::read_secrets_from_file(),
            persistance :Persistance::new()
        };

        if let Some(athlete_tokens) = this.persistance.get_athlete_tokens(athlete_id) {
            this.athlete_tokens = athlete_tokens;
            
            Auth::refresh_tokens_if_expired(&mut this);
        }        

        println!(
            "Credentials for athlete {}:\naccess: {}\nrefresh: {}\nexpiration: {}",
            athlete_id,
            this.athlete_tokens.access_token,
            this.athlete_tokens.refresh_token,
            this.athlete_tokens.expires_at
        );

        this
    }

    pub fn get_access_token(&self) -> &String {
        return &self.athlete_tokens.access_token;
    }
}
