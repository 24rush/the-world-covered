use futures_util::TryStreamExt;

use crate::{
    data_types::{common::Identifiable, strava::activity::Activity},
    database::strava_db::ResourceType,
    logln,
    util::dependencies::{Dependencies, DependenciesBuilder, Required},
};

use super::sync_from_strava::StravaDBSync;

#[derive(Default, Copy, Clone)]
pub struct Options {
    pub skip_activity_sync: bool,
    pub skip_activity_telemetry: bool,
    pub skip_segment_caching: bool,
    pub skip_segment_telemetry: bool,
}

pub struct DBIntegrityChecker<'a> {
    pub options: Options,
    pub dependencies: &'a mut Dependencies<'a>,
}

impl<'a> DBIntegrityChecker<'a> {
    const CC: &str = "DBIntegrityChecker";

    pub fn new(dependencies: &'a mut Dependencies<'a>, options: &Options) -> Self {
        Self {
            dependencies,
            options: options.clone(),
        }
    }

    async fn process_activity(&mut self, activity: &Activity) {
        let act_id = activity.as_i64();

        logln!("Checking activity: {}", act_id);

        if !self.options.skip_activity_telemetry {
            // Check telemetry for activity
            if !self
                .dependencies
                .strava_db()
                .exists_resource(ResourceType::Telemetry, act_id)
                .await
            {
                let act = self
                    .dependencies
                    .strava_db()
                    .get_activity(act_id)
                    .await
                    .unwrap();

                logln!("Downloading activity telemetry...");
                if let Some(mut telemetry_json) = self
                    .dependencies
                    .strava_api()
                    .get_activity_telemetry(act_id)
                    .await
                {
                    let mut m = telemetry_json.as_object().unwrap().clone();
                    m.insert(
                        "athlete".to_string(),
                        serde_json::json!({ "id": self.dependencies.athlete_id }),
                    );
                    m.insert("type".to_string(), serde_json::Value::String(act.r#type));

                    telemetry_json = serde_json::Value::Object(m);
                    self.dependencies
                        .strava_db()
                        .store_resource(ResourceType::Telemetry, act_id, &mut telemetry_json)
                        .await;
                }
            }
        }
    }

    pub async fn start(&mut self) {
        let mut athlete_data = self
            .dependencies
            .strava_db()
            .get_athlete_data(self.dependencies.athlete_id())
            .await
            .unwrap();

        if !self.options.skip_activity_sync {
            StravaDBSync::new(
                DependenciesBuilder::new(vec![Required::AthleteId, Required::StravaDB, Required::StravaApi])
                    .with_athlete_id(self.dependencies.athlete_id())
                    .with_strava_db(self.dependencies.strava_db.unwrap())
                    .with_strava_api(self.dependencies.strava_api())
                    .build(),
            )
            .sync_athlete_activities()
            .await;
        }

        if !self.options.skip_activity_telemetry || !self.options.skip_segment_caching {
            let mut athlete_data_touched = false;

            while let Some(activity) = self
                .dependencies
                .strava_db()
                .get_athlete_activities(self.dependencies.athlete_id())
                .await
                .try_next()
                .await
                .unwrap()
            {
                // Go over segment efforts, pickup all the segments ids and add them to the user's visited
                for effort in &activity.segment_efforts {
                    let seg_id = effort.segment.id;

                    athlete_data.incr_visited_segment(seg_id);
                    athlete_data_touched = true;
                }

                self.process_activity(&activity).await;
            }

            if athlete_data_touched {
                logln!("Saving athlete data...");

                self.dependencies
                    .strava_db()
                    .set_athlete_data(&athlete_data)
                    .await;
            }

            if !self.options.skip_segment_caching || !self.options.skip_segment_telemetry {
                let athlete_segments = &athlete_data.segments;

                for (seg_id_str, _) in athlete_segments {
                    let seg_id = seg_id_str.parse().unwrap();

                    if !self.options.skip_segment_caching {
                        if !self
                            .dependencies
                            .strava_db()
                            .exists_resource(ResourceType::Segment, seg_id)
                            .await
                        {
                            logln!("Downloading segment {}...", seg_id_str);
                            if let Some(mut segment_json) =
                                self.dependencies.strava_api().get_segment(seg_id).await
                            {
                                self.dependencies
                                    .strava_db()
                                    .store_resource(
                                        ResourceType::Segment,
                                        seg_id,
                                        &mut segment_json,
                                    )
                                    .await;
                            }
                        }
                    }

                    if !self.options.skip_segment_telemetry {
                        if !self
                            .dependencies
                            .strava_db()
                            .exists_resource(ResourceType::Telemetry, seg_id)
                            .await
                        {
                            logln!("Downloading segment {} telemetry...", seg_id_str);
                            if let Some(mut telemetry_json) = self
                                .dependencies
                                .strava_api()
                                .get_segment_telemetry(seg_id)
                                .await
                            {
                                self.dependencies
                                    .strava_db()
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
