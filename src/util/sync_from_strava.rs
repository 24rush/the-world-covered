use chrono::Utc;

use crate::{logvbln, strava::api::StravaApi, database::strava_db::{StravaDB, ResourceType}, logln, util::DateTimeUtils};

pub struct UtilitiesContext<'a> {
    pub strava_api: &'a mut StravaApi,
    pub persistance: &'a StravaDB,
}

pub struct StravaDBSync;

impl StravaDBSync {
    const CC: &str = "Util";

    pub async fn sync_athlete_activities(ctx: &mut UtilitiesContext<'_>, id: i64) {
        // Sync =
        // all activities from 0 to before_ts (if before_ts is not 0)
        //  +
        // all activities from after_ts to current timestamp (if interval passed and first stage is completed)
        let athlete_data = ctx.persistance.get_athlete_data(id).await.unwrap();
        let (after_ts, before_ts) = (athlete_data.after_ts, athlete_data.before_ts);

        logvbln!("sync_athlete_activities {} {}", before_ts, after_ts);

        if before_ts != 0 && after_ts != before_ts {
            if let (last_activity_ts, false) =
                StravaDBSync::download_activities_in_range(ctx, id, 0, before_ts).await
            {
                // when done move before to 0 and after to last activity ts
                ctx.persistance
                    .save_after_before_timestamps(id, last_activity_ts, 0)
                    .await;
            }
        } else {
            let current_ts: i64 = Utc::now().timestamp();
            let days_since_last_sync = (current_ts - after_ts) / 86400;

            if days_since_last_sync >= 0 {
                if let (_, false) =
                    StravaDBSync::download_activities_in_range(ctx, id, after_ts, current_ts).await
                {
                    // when done move after to current
                    ctx.persistance
                        .save_after_before_timestamps(id, current_ts, current_ts)
                        .await;
                }
            }
        }

        logvbln!("done syncing.");
    }

    async fn download_activities_in_range(
        ctx: &mut UtilitiesContext<'_>,
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
            ).await {
                has_more_items = activities_list.len() == ACTIVITIES_PER_PAGE;

                for activity in activities_list {
                    let act_id = activity["id"].as_i64().unwrap();

                    last_activity_ts =
                        DateTimeUtils::zulu2ts(&activity["start_date"].as_str().unwrap());

                    ctx.persistance
                        .save_after_before_timestamps(id, after_ts, last_activity_ts)
                        .await;

                    if ctx
                        .persistance
                        .exists_resource(ResourceType::Activity, act_id)
                        .await
                    {
                        logvbln!("Activity {} already in DB. Skipping download.", act_id);

                        continue;
                    }

                    if let Some(mut new_activity) = ctx.strava_api.get_activity(act_id).await {
                        ctx.persistance
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