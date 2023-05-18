use crate::{data_types::strava::athlete::AthleteId, database::strava_db::ResourceType, logln};

use super::sync_from_strava::{StravaDBSync, UtilitiesContext};

pub struct Options {
    pub skip_activity_sync: bool,
    pub skip_activity_telemetry: bool,
    pub skip_segment_caching: bool,
    pub skip_segment_telemetry: bool,
}

pub struct DBIntegrityChecker;

impl DBIntegrityChecker {
    const CC: &str = "DBIntegrityChecker";

    pub async fn perform_db_integrity_check(
        ath_id: AthleteId,
        options: &Options,
        utilities_ctx: &mut UtilitiesContext<'_>,
    ) {
        let mut athlete_data = utilities_ctx.persistance.get_athlete_data(ath_id).await.unwrap();

        if !options.skip_activity_sync {
            StravaDBSync::sync_athlete_activities(utilities_ctx, ath_id).await;
        }

        if !options.skip_activity_telemetry || !options.skip_segment_caching {
            let mut athlete_data_touched = false;

            for act_id in utilities_ctx
                .persistance
                .get_athlete_activity_ids(ath_id)
                .await
            {
                logln!("Checking activity: {}", act_id);

                if !options.skip_activity_telemetry {
                    // Check telemetry for activity
                    if !utilities_ctx
                        .persistance
                        .exists_resource(ResourceType::Telemetry, act_id)
                        .await
                    {
                        let act = utilities_ctx
                            .persistance
                            .get_activity(act_id)
                            .await
                            .unwrap();

                        logln!("Downloading activity telemetry...");
                        if let Some(mut telemetry_json) =
                        utilities_ctx.strava_api.get_activity_telemetry(act_id).await
                        {
                            let mut m = telemetry_json.as_object().unwrap().clone();
                            m.insert("athlete".to_string(), serde_json::json!({ "id": ath_id }));
                            m.insert("type".to_string(), serde_json::Value::String(act.r#type));

                            telemetry_json = serde_json::Value::Object(m);

                            utilities_ctx
                                .persistance
                                .store_resource(
                                    ResourceType::Telemetry,
                                    act_id,
                                    &mut telemetry_json,
                                )
                                .await;
                        }
                    }
                }

                // Go over segment efforts, pickup all the segments ids and add them to the user's visited
                if let Some(activity) = utilities_ctx.persistance.get_activity(act_id).await {
                    for effort in activity.segment_efforts {
                        let seg_id = effort.segment.id;

                        athlete_data.incr_visited_segment(seg_id);
                        athlete_data_touched = true;
                    }
                }
            }

            if athlete_data_touched {
                logln!("Saving athlete data...");
                utilities_ctx
                    .persistance
                    .set_athlete_data(&athlete_data)
                    .await;
            }

            if !options.skip_segment_caching || !options.skip_segment_telemetry {
                let athlete_segments = &athlete_data.segments;

                for (seg_id_str, _) in athlete_segments {
                    let seg_id = seg_id_str.parse().unwrap();

                    if !options.skip_segment_caching {
                        if !utilities_ctx
                            .persistance
                            .exists_resource(ResourceType::Segment, seg_id)
                            .await
                        {
                            logln!("Downloading segment {}...", seg_id_str);
                            if let Some(mut segment_json) =
                            utilities_ctx.strava_api.get_segment(seg_id).await
                            {
                                utilities_ctx
                                    .persistance
                                    .store_resource(
                                        ResourceType::Segment,
                                        seg_id,
                                        &mut segment_json,
                                    )
                                    .await;
                            }
                        }
                    }

                    if !options.skip_segment_telemetry {
                        if !utilities_ctx
                            .persistance
                            .exists_resource(ResourceType::Telemetry, seg_id)
                            .await
                        {
                            logln!("Downloading segment {} telemetry...", seg_id_str);
                            if let Some(mut telemetry_json) =
                            utilities_ctx.strava_api.get_segment_telemetry(seg_id).await
                            {
                                utilities_ctx
                                    .persistance
                                    .store_resource(
                                        ResourceType::Telemetry,
                                        seg_id,
                                        &mut telemetry_json,
                                    )
                                    .await;
                            }
                        }
                    }
                }
            }
        }
    }
}
