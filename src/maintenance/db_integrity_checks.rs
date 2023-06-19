use crate::{
    data_types::{
        strava::athlete::AthleteId,
    },
    database::strava_db::ResourceType,
    logln,
    util::facilities::{DependenciesBuilder, Facilities, Required},
};

use super::sync_from_strava::StravaDBSync;

#[derive(Default, Copy, Clone)]
pub struct Options {
    pub activity_sync: bool,
    pub segment_caching: bool,
    pub segment_telemetry: bool,
}

pub struct DBIntegrityChecker<'a> {
    athlete_id: AthleteId,
    pub options: Options,
    pub dependencies: &'a mut Facilities<'a>,
}

impl<'a> DBIntegrityChecker<'a> {
    const CC: &str = "DBIntegrityChecker";

    pub fn new(dependencies: &'a mut Facilities<'a>, options: &Options) -> Self {
        dependencies.check(vec![Required::StravaDB, Required::StravaApi]);

        Self {
            athlete_id: 0,
            dependencies,
            options: options.clone(),
        }
    }

    pub async fn start(&mut self, athlete_id: AthleteId) {
        self.athlete_id = athlete_id;

        let athlete_data = self
            .dependencies
            .strava_db()
            .get_athlete_data(athlete_id)
            .await
            .unwrap();

        if self.options.activity_sync {
            StravaDBSync::new(
                DependenciesBuilder::new()
                    .with_strava_db(self.dependencies.strava_db())
                    .with_strava_api(self.dependencies.strava_api())
                    .build(),
            )
            .sync_athlete_activities(athlete_id)
            .await;
        }

        if self.options.segment_caching || self.options.segment_telemetry {
            let athlete_segments = &athlete_data.segments;

            for (seg_id_str, _) in athlete_segments {
                let seg_id = seg_id_str.parse().unwrap();

                if self.options.segment_caching {
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
                                .store_resource(ResourceType::Segment, seg_id, &mut segment_json)
                                .await;
                        }
                    }
                }

                if self.options.segment_telemetry {
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
