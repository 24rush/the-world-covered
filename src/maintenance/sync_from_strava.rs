use chrono::Utc;

use crate::{
    data_types::strava::athlete::AthleteId,
    database::strava_db::ResourceType,
    logln, logvbln,
    util::{
        facilities::{Facilities, Required},
        DateTimeUtils,
    },
};

pub struct StravaDBSync<'a> {
    athlete_id: AthleteId,
    dependencies: &'a mut Facilities<'a>,
}

impl<'a> StravaDBSync<'a> {
    const CC: &str = "Util";

    pub fn new(dependencies: &'a mut Facilities<'a>) -> Self {
        dependencies.check(vec![Required::StravaDB, Required::StravaApi]);

        Self {
            athlete_id: 0,
            dependencies,
        }
    }

    pub async fn sync_athlete_activities(&mut self, athlete_id: AthleteId) {
        // Sync =
        // all activities from 0 to before_ts (if before_ts is not 0)
        //  +
        // all activities from after_ts to current timestamp (if interval passed and first stage is completed)
        self.athlete_id = athlete_id;

        let athlete_data = self
            .dependencies
            .strava_db()
            .get_athlete_data(self.athlete_id)
            .await
            .unwrap();
        let (after_ts, before_ts) = (athlete_data.after_ts, athlete_data.before_ts);

        logvbln!("sync_athlete_activities {} {}", before_ts, after_ts);

        if before_ts != 0 && after_ts != before_ts {
            if let (last_activity_ts, false) = self.download_activities_in_range(0, before_ts).await
            {
                // when done move before to 0 and after to last activity ts
                self.dependencies
                    .strava_db()
                    .save_after_before_timestamps(self.athlete_id, last_activity_ts, 0)
                    .await;
            }
        } else {
            let current_ts: i64 = Utc::now().timestamp();
            let days_since_last_sync = (current_ts - after_ts) / 86400;

            if days_since_last_sync >= 0 {
                if let (_, false) = self
                    .download_activities_in_range(after_ts, current_ts)
                    .await
                {
                    // when done move after to current
                    self.dependencies
                        .strava_db()
                        .save_after_before_timestamps(self.athlete_id, current_ts, current_ts)
                        .await;
                }
            }
        }

        logvbln!("done syncing.");
    }

    async fn download_activities_in_range(&mut self, after_ts: i64, before_ts: i64) -> (i64, bool) {
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
            if let Some(activities_list) = self
                .dependencies
                .strava_api()
                .list_athlete_activities(after_ts, before_ts, ACTIVITIES_PER_PAGE, page)
                .await
            {
                has_more_items = activities_list.len() == ACTIVITIES_PER_PAGE;

                for activity in activities_list {
                    let act_id = activity["id"].as_i64().unwrap();

                    last_activity_ts =
                        DateTimeUtils::zulu2ts(&activity["start_date"].as_str().unwrap());

                    self.dependencies
                        .strava_db()
                        .save_after_before_timestamps(self.athlete_id, after_ts, last_activity_ts)
                        .await;

                    if self
                        .dependencies
                        .strava_db()
                        .exists_resource(ResourceType::Activity, act_id)
                        .await
                    {
                        logvbln!("Activity {} already in DB. Skipping download.", act_id);

                        continue;
                    }

                    if let Some(mut new_activity) =
                        self.dependencies.strava_api().get_activity(act_id).await
                    {
                        self.dependencies
                            .strava_db()
                            .store_resource(ResourceType::Activity, act_id, &mut new_activity)
                            .await;
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
