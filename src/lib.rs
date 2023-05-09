use chrono::{Utc};
use data_types::athlete::AthleteData;
use database::persistance::Persistance;
use strava::api::Api;

use crate::util::DateTimeUtils;

mod data_types;
mod util;
mod database;
mod strava;

pub struct App {
    loggedin_athlete_id: String,
    strava_api: Api,
    persistance: Persistance,
}

impl App {
    pub fn new(id: &String) -> Self {
        Self {
            loggedin_athlete_id: id.to_string(),
            strava_api: Api::authenticate_athlete(id),
            persistance: Persistance::new(),
        }
    }

    pub fn get_athlete(&self, id: &String) -> Option<AthleteData> {
        self.persistance.get_athlete_activity_ids(id);
        self.persistance.get_athlete_data(id)
    }

    pub fn create_athlete(&self, id: &String) -> AthleteData {
        let default_athlete = AthleteData::new();
        self.persistance.set_athlete_data(id, &default_athlete);

        default_athlete
    }

    fn download_activities_in_range(
        &self,
        id: &String,
        after_ts: i64,
        before_ts: i64,
    ) -> (i64, bool) {
        println!(
            "download from {} to {}",
            DateTimeUtils::timestamp_to_str(after_ts),
            DateTimeUtils::timestamp_to_str(before_ts)
        );

        const ACTIVITIES_PER_PAGE: usize = 2;
        
        let mut last_activity_ts = before_ts;        
        let mut page = 1;
        let mut has_more_items = false;

        loop {
            if let Some(activities_list) = self.strava_api.list_athlete_activities(
                after_ts,
                before_ts,
                ACTIVITIES_PER_PAGE,
                page,
            ) {
                has_more_items = activities_list.len() == ACTIVITIES_PER_PAGE;

                for activity in activities_list {
                    let act_id = activity["id"].to_string();

                    last_activity_ts = DateTimeUtils::zulu2ts(&activity["start_date"].as_str().unwrap());

                    self.persistance
                        .save_after_before_timestamps(id, after_ts, last_activity_ts);

                    if self.persistance.activity_exists(id, &act_id) {
                        println!("Activity {} already in DB. Skipping download.", act_id);

                        continue;
                    }

                    self.store_athlete_activity(id, &act_id);
                }
            }

            if !has_more_items {                
                break;
            }

            page += 1;
        }

        (last_activity_ts, has_more_items)
    }

    pub fn store_athlete_activity(&self, id: &String, act_id: &String) {
        println!("Downloading activity: {}", act_id);

        if let Some(new_activity) = self.strava_api.get_activity(&act_id) {
            self.persistance
                .store_athlete_activity(id, &act_id, &new_activity);
        }
    }

    pub fn sync_athlete_activities(&self, id: &String) {
        // Sync =
        // all activities from 0 to before_ts (if before_ts is not 0)
        //  +
        // all activities from after_ts to current timestamp (if interval passed and first stage is completed)

        let (after_ts, before_ts) = self.persistance.get_after_before_timestamps(id);
        
        if before_ts != 0 && after_ts != before_ts {
            if let (last_activity_ts, false) = self.download_activities_in_range(id, 0, before_ts) {
                // when done move before to 0 and after to last activity ts
                self.persistance
                    .save_after_before_timestamps(id, last_activity_ts, 0);
            }
        } else {
            let current_ts: i64 = Utc::now().timestamp();
            let days_since_last_sync = (current_ts - after_ts) / 86400;
            
            if days_since_last_sync >= 1 && before_ts == 0 {
                if let (_, false) = self.download_activities_in_range(id, after_ts, current_ts) {
                    // when done move after to current
                    self.persistance
                        .save_after_before_timestamps(id, current_ts, current_ts);
                }
            }
        }
    }

    pub fn check_database_integrity(&self) {

        for activity_db_id in self.persistance.get_athlete_activity_ids(&self.loggedin_athlete_id) {
            let act_id = activity_db_id.split(":").collect::<Vec<&str>>()[3];

            if self.persistance.telemetry_exists(&self.loggedin_athlete_id, act_id) {
                continue;
            }


            println!("Downloading telemetry for: {act_id}");
            if let Some(telemetry_json) = self.strava_api.get_telemetry(act_id) {
                self.persistance.set_activity_streams(&self.loggedin_athlete_id, act_id, &telemetry_json);
            } 
        }        
    }
}
