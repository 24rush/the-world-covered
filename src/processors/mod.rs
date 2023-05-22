use geo_types:: Coord;
use std::collections::{HashMap};

use crate::{
    data_types::{
        common::{DocumentId, Identifiable},
        gc::{effort::Effort, segment::Segment},
        strava::athlete::AthleteId,
    },
    util::{
        benchmark::Benchmark,
        facilities::{Facilities, Required},
    },
};

use self::commonality::Commonality;

pub mod commonality;
pub mod gradient_finder;

pub struct DataCreationPipelineOptions {
    pub commonalities: bool,
    pub route_processor: bool,
}

pub struct DataCreationPipeline<'a> {
    athlete_id: AthleteId,
    dependencies: &'a mut Facilities<'a>,
}

impl<'a> DataCreationPipeline<'a> {
    const CC: &str = "Pipeline";

    pub fn new(dependencies: &'a mut Facilities<'a>) -> Self {
        dependencies.check(vec![Required::GcDB, Required::StravaDB]);

        Self {
            athlete_id: 0,
            dependencies,
        }
    }

    pub async fn start(&mut self, athlete_id: AthleteId, options: &DataCreationPipelineOptions) {
        self.athlete_id = athlete_id;

        if options.commonalities {
            // 1 //
            // Clear previous results and store latest ones
            self.dependencies.gc_db().clear_routes().await;

            // Run route matching module => routes collections
            self.run_commonalities().await;
        }

        if options.route_processor {
            // 2 //
            // Clear previous segments
            self.dependencies.gc_db().clear_segments().await;

            // Using created routes, run RouteProcessor => segments collections, updates route collection
            self.run_route_processor().await;
        }
    }

    async fn run_commonalities(&self) {
        Benchmark::start("commonalities");

        let mut processor: Commonality = Default::default();
        let mut telemetries = self
            .dependencies
            .strava_db()
            .get_athlete_telemetries(self.athlete_id)
            .await;

        let mut items_to_process = 0;
        while telemetries.advance().await.unwrap() {
            let telemetry = telemetries.deserialize_current().unwrap();

            if items_to_process >= 50 {
                break;
            }

            processor.process(&telemetry);
            items_to_process += 1;
        }

        for mut route in processor.end_session() {
            // Mark the routes as being the athlete's
            route.athlete_id = self.athlete_id;
            self.dependencies.gc_db().update_route(&route).await;
        }
    }

    async fn run_route_processor(&self) {
        let mut segments_in_master_activity: HashMap<DocumentId, Segment> = HashMap::new();
        let mut efforts_in_all_activities: Vec<Effort> = Vec::new();

        let mut routes = self.dependencies.gc_db().get_routes(self.athlete_id).await;

        while routes.advance().await.unwrap() {
            let mut route = routes.deserialize_current().unwrap();
            
            // Find the master activity of this routes: the longest one from the matched ones
            let master_activity = self
                .dependencies
                .strava_db()
                .get_max_distance_activity_in_ids(&route.activities)
                .await
                .unwrap();

            // Extract data from master activity and put it into route
            route.master_activity_id = master_activity._id as DocumentId;

            // Get all matched activities and fill in all the efforts
            let mut activities = self
                .dependencies
                .strava_db()
                .get_athlete_activities_with_ids(self.athlete_id, &route.activities)
                .await;

            while activities.advance().await.unwrap() {
                let activity = activities.deserialize_current().unwrap();
                let telemetry = self
                    .dependencies
                    .strava_db()
                    .get_telemetry_by_id(activity.as_i64())
                    .await
                    .unwrap();

                // Populate efforts collection
                activity.segment_efforts.iter().for_each(|effort| {
                    // Extract segment effort
                    efforts_in_all_activities.push(Effort {
                        _id: effort.id as f64,
                        athlete_id: effort.athlete.id,
                        segment_id: effort.segment.id,
                        activity_id: effort.activity.id,

                        moving_time: effort.moving_time,
                        distance_from_start: telemetry.distance.data[effort.start_index as usize],
                    })
                });

                if activity.as_i64() == route.master_activity_id {
                    let gradients = gradient_finder::GradientFinder::find_gradients(&telemetry);

                    if gradients.len() > 0 {
                        route.gradients = gradients;
                    }
                }
            }

            // Collect segment_ids from master activity's efforts (effort 1-1 segment)
            for effort_in_master in &master_activity.segment_efforts {
                let seg_id = effort_in_master.segment.id as DocumentId;

                if segments_in_master_activity.contains_key(&seg_id) {
                    continue;
                }

                if let Some(db_segment) = self.dependencies.strava_db().get_segment(seg_id).await {
                    segments_in_master_activity.insert(
                        seg_id,
                        Segment {
                            _id: seg_id as f64,
                            polyline: db_segment.map.polyline,
                            kom: db_segment.xoms.kom,
                            qom: db_segment.xoms.qom,
                            time_to_xom: 0,
                            start_index: effort_in_master.start_index,
                            end_index: effort_in_master.end_index,
                        },
                    );
                } else {
                    /*logln!(
                        "WARNING: Cannot find segment {} in DB for activity: {}",
                        seg_id,
                        master_activity.as_i64()
                    );*/

                    let telemetry = self
                        .dependencies
                        .strava_db()
                        .get_telemetry_by_id(master_activity.as_i64())
                        .await
                        .unwrap();

                    let telemetry_data = &telemetry.latlng.data;
                    let coordinates: Vec<Coord> = (effort_in_master.start_index as usize
                        ..=effort_in_master.end_index as usize)
                        .map(|index| {
                            let x = telemetry_data[index][1] as f64;
                            let y = telemetry_data[index][0] as f64;

                            Coord { x, y }
                        })
                        .collect();

                    segments_in_master_activity.insert(
                        seg_id,
                        Segment {
                            _id: seg_id as f64,
                            polyline: polyline::encode_coordinates(coordinates, 5)
                                .unwrap_or("".to_string()),
                            kom: "0".to_string(), // Missing data
                            qom: "0".to_string(), // Missing data
                            time_to_xom: 0,       // Missing data
                            start_index: effort_in_master.start_index,
                            end_index: effort_in_master.end_index,
                        },
                    );
                }
            }

            self.dependencies.gc_db().update_route(&route).await;
        }

        for segment in segments_in_master_activity {
            self.dependencies.gc_db().update_segment(&segment.1).await;
        }

        for effort in efforts_in_all_activities {
            self.dependencies.gc_db().update_effort(&effort).await;
        }
    }
}
