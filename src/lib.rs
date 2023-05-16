use chrono::Utc;
use data_types::strava::{activity::Activity, athlete::{AthleteData, AthleteId}};
use database::{strava_db::StravaDB, gc_db::GCDB};

use processors::Pipeline;
use serde_json::json;
use strava::api::StravaApi;

use crate::{
    database::strava_db::ResourceType,
    util::{logging, DateTimeUtils},
};

mod data_types;
mod database;
mod strava;
mod util;

mod processors;

pub struct App {
    loggedin_athlete_id: AthleteId,
    strava_api: StravaApi,
    strava_db: StravaDB,
    gc_db: GCDB
}

struct UtilitiesContext<'a> {
    strava_api: &'a StravaApi,
    persistance: &'a StravaDB,
}

struct Utilities {}

impl Utilities {
    const CC: &str = "Util";

    pub fn sync_athlete_activities(ctx: &UtilitiesContext, id: i64) {
        // Sync =
        // all activities from 0 to before_ts (if before_ts is not 0)
        //  +
        // all activities from after_ts to current timestamp (if interval passed and first stage is completed)
        let athlete_data = ctx.persistance.get_athlete_data(id).unwrap();
        let (after_ts, before_ts) = (athlete_data.after_ts, athlete_data.before_ts);

        logvbln!("sync_athlete_activities {} {}", before_ts, after_ts);

        if before_ts != 0 && after_ts != before_ts {
            if let (last_activity_ts, false) =
                Utilities::download_activities_in_range(ctx, id, 0, before_ts)
            {
                // when done move before to 0 and after to last activity ts
                ctx.persistance
                    .save_after_before_timestamps(id, last_activity_ts, 0);
            }
        } else {
            let current_ts: i64 = Utc::now().timestamp();
            let days_since_last_sync = (current_ts - after_ts) / 86400;

            if days_since_last_sync >= 0 {
                if let (_, false) =
                    Utilities::download_activities_in_range(ctx, id, after_ts, current_ts)
                {
                    // when done move after to current
                    ctx.persistance
                        .save_after_before_timestamps(id, current_ts, current_ts);
                }
            }
        }

        logvbln!("done syncing.");
    }

    fn download_activities_in_range(
        ctx: &UtilitiesContext,
        id: i64,
        after_ts: i64,
        before_ts: i64,
    ) -> (i64, bool) {
        logln!(
            "download from {} to {}",
            DateTimeUtils::timestamp_to_str(after_ts),
            DateTimeUtils::timestamp_to_str(before_ts)
        );

        const ACTIVITIES_PER_PAGE: usize = 2;

        let mut last_activity_ts = before_ts;
        let mut page = 1;
        let mut has_more_items = false;

        loop {
            if let Some(activities_list) = ctx.strava_api.list_athlete_activities(
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

                    ctx.persistance
                        .save_after_before_timestamps(id, after_ts, last_activity_ts);

                    if ctx
                        .persistance
                        .exists_resource(ResourceType::Activity, act_id)
                    {
                        logvbln!("Activity {} already in DB. Skipping download.", act_id);

                        continue;
                    }

                    if let Some(mut new_activity) = ctx.strava_api.get_activity(act_id) {
                        ctx.persistance.store_resource(
                            ResourceType::Activity,
                            act_id,
                            &mut new_activity,
                        );
                    }
                }
            } else {
                logln!("No activities in range.")
            }

            if !has_more_items {
                break;
            }

            page += 1;
        }

        (last_activity_ts, has_more_items)
    }
}

impl App {
    const CC: &str = "App";

    pub fn new(id: AthleteId) -> Self {
        logging::set_global_level(logging::LogLevel::VERBOSE);

        let strava_api = StravaApi::authenticate_athlete(id);

        Self {
            loggedin_athlete_id: id,
            strava_api,
            strava_db: StravaDB::new(),
            gc_db: GCDB::new()
        }
    }
    
    pub fn get_athlete_data(&self, id: i64) -> Option<AthleteData> {
        self.strava_db.get_athlete_data(id)
    }

    pub fn get_activity(&self, id: i64) -> Option<Activity> {
        self.strava_db.get_activity(id)
    }

    pub fn create_athlete(&self, id: i64) -> AthleteData {
        let default_athlete = AthleteData::new(id);
        self.strava_db.set_athlete_data(&default_athlete);

        default_athlete
    }

    pub fn store_athlete_activity(&self, act_id: i64) {
        logln!("Downloading activity: {}", act_id);

        if let Some(mut new_activity) = self.strava_api.get_activity(act_id) {
            self.strava_db
                .store_resource(ResourceType::Activity, act_id, &mut new_activity);
        }
    }

    pub fn start_db_pipeline(&self) {
        Pipeline::start(self.loggedin_athlete_id, &self.strava_db, &self.gc_db);
    }
    
    pub fn perform_db_integrity_check(&self) {
        struct Options {
            skip_activity_sync: bool,
            skip_activity_telemetry: bool,
            skip_segment_caching: bool,
            skip_segment_telemetry: bool,
        }

        let options = Options {
            skip_activity_sync: true,
            skip_activity_telemetry: true,
            skip_segment_caching: false,
            skip_segment_telemetry: true,
        };

        let mut athlete_data = self.get_athlete_data(self.loggedin_athlete_id).unwrap();

        let utilities_ctx = UtilitiesContext {
            strava_api: &self.strava_api,
            persistance: &self.strava_db,
        };

        if !options.skip_activity_sync {
            Utilities::sync_athlete_activities(&utilities_ctx, self.loggedin_athlete_id);
        }

        if !options.skip_activity_telemetry || !options.skip_segment_caching {
            let mut athlete_data_touched = false;

            for act_id in self
                .strava_db
                .get_athlete_activity_ids(self.loggedin_athlete_id)
            {
                logln!("Checking activity: {}", act_id);

                if !options.skip_activity_telemetry {
                    // Check telemetry for activity
                    if !self
                        .strava_db
                        .exists_resource(ResourceType::Telemetry, act_id)
                    {
                        let act = self.strava_db.get_activity(act_id).unwrap();

                        logln!("Downloading activity telemetry...");
                        if let Some(mut telemetry_json) =
                            self.strava_api.get_activity_telemetry(act_id)
                        {
                            let mut m = telemetry_json.as_object().unwrap().clone();
                            m.insert(
                                "athlete".to_string(),
                                json!({"id" : self.loggedin_athlete_id}),
                            );
                            m.insert("type".to_string(), serde_json::Value::String(act.r#type));

                            telemetry_json = serde_json::Value::Object(m);

                            self.strava_db.store_resource(
                                ResourceType::Telemetry,
                                act_id,
                                &mut telemetry_json,
                            );
                        }
                    }
                }

                // Go over segment efforts, pickup all the segments ids and add them to the user's visited
                if let Some(activity) = self.strava_db.get_activity(act_id) {
                    for effort in activity.segment_efforts {
                        let seg_id = effort.segment.id;

                        athlete_data.incr_visited_segment(seg_id);
                        athlete_data_touched = true;
                    }
                }
            }

            if athlete_data_touched {
                logln!("Saving athlete data...");
                self.strava_db.set_athlete_data(&athlete_data);
            }

            if !options.skip_segment_caching || !options.skip_segment_telemetry {
                let athlete_segments = &athlete_data.segments;

                for (seg_id_str, _) in athlete_segments {
                    let seg_id = seg_id_str.parse().unwrap();

                    if !options.skip_segment_caching {
                        if !self
                            .strava_db
                            .exists_resource(ResourceType::Segment, seg_id)
                        {
                            logln!("Downloading segment {seg_id_str}...");
                            if let Some(mut segment_json) = self.strava_api.get_segment(seg_id) {
                                self.strava_db.store_resource(
                                    ResourceType::Segment,
                                    seg_id,
                                    &mut segment_json,
                                );
                            }
                        }
                    }

                    if !options.skip_segment_telemetry {
                        if !self
                            .strava_db
                            .exists_resource(ResourceType::Telemetry, seg_id)
                        {
                            logln!("Downloading segment {seg_id_str} telemetry...");
                            if let Some(mut telemetry_json) =
                                self.strava_api.get_segment_telemetry(seg_id)
                            {
                                self.strava_db.store_resource(
                                    ResourceType::Telemetry,
                                    seg_id,
                                    &mut telemetry_json,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
