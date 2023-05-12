use chrono::Utc;
use data_types::{activity::Activity, athlete::AthleteData};
use database::persistance::Persistance;

use mongodb::bson::{self};
use strava::api::Api;

use crate::util::DateTimeUtils;

mod data_types;
mod database;
mod strava;
mod util;

pub struct App {
    loggedin_athlete_id: i64,
    strava_api: Api,
    persistance: Persistance,
}

impl App {
    pub fn new(id: i64) -> Self {
        Self {
            loggedin_athlete_id: id,
            strava_api: Api::authenticate_athlete(id),
            persistance: Persistance::new(),
        }
    }

    pub fn get_athlete(&self, id: i64) -> Option<AthleteData> {
        self.persistance.get_athlete_data(id)
    }

    pub fn create_athlete(&self, id: i64) -> AthleteData {
        let default_athlete = AthleteData::new(id);
        self.persistance.set_athlete_data(&default_athlete);

        default_athlete
    }

    fn download_activities_in_range(&self, id: i64, after_ts: i64, before_ts: i64) -> (i64, bool) {
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
                    let act_id = activity["id"].as_i64().unwrap();

                    last_activity_ts =
                        DateTimeUtils::zulu2ts(&activity["start_date"].as_str().unwrap());

                    self.persistance
                        .save_after_before_timestamps(id, after_ts, last_activity_ts);

                    if self.persistance.activity_exists(act_id) {
                        println!("Activity {} already in DB. Skipping download.", act_id);

                        continue;
                    }

                    self.store_athlete_activity(act_id);
                }
            }

            if !has_more_items {
                break;
            }

            page += 1;
        }

        (last_activity_ts, has_more_items)
    }

    pub fn store_athlete_activity(&self, act_id: i64) {
        println!("Downloading activity: {}", act_id);

        if let Some(mut new_activity) = self.strava_api.get_activity(act_id) {
            self.persistance
                .store_athlete_activity(act_id, &mut new_activity);
        }
    }

    pub fn sync_athlete_activities(&self, id: i64) {
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
        struct Options {
            skip_telemetry: bool,
            skip_segment_caching: bool,
        }

        let options = Options {
            skip_telemetry: true,
            skip_segment_caching: false,
        };

        let mut athlete_data_touched = false;
        let mut athlete_data = self.get_athlete(self.loggedin_athlete_id).unwrap();

        for act_id in self
            .persistance
            .get_athlete_activity_ids(self.loggedin_athlete_id)
        {
            println!("ActID: {act_id}");

            if !options.skip_telemetry {
                // Check telemetry for activity
                if !self.persistance.telemetry_exists(act_id) {
                    println!("Downloading telemetry for: {act_id}");
                    if let Some(mut telemetry_json) = self.strava_api.get_telemetry(act_id) {
                        self.persistance
                            .set_activity_streams(act_id, &mut telemetry_json);
                    }
                }
            }

            if !options.skip_segment_caching {
                // Go over segment efforts, pickup all the segments ids and check if they exist
                let db_activity = &self.persistance.get_activity(act_id).unwrap();

                if let Ok(activity) = bson::from_bson::<Activity>(db_activity.into()) {
                    for effort in activity.segment_efforts {
                        let seg_id = effort.segment.id;

                        athlete_data.incr_visited_segment(seg_id);
                        athlete_data_touched = true;
                    }
                }
            }
        }

        if athlete_data_touched {
            self.persistance.set_athlete_data(&athlete_data);
        }

        println!("{:#?}", athlete_data);
    }
}
