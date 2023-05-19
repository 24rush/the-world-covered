use std::collections::HashMap;

use crate::{
    data_types::{
        common::DocumentId,
        gc::{effort::Effort, route::Route, segment::Segment},
    },
    logln,
    util::{benchmark::Benchmark, dependencies::Dependencies},
};

use self::commonality::Commonality;

pub mod commonality;
pub mod routes;

pub struct PipelineOptions {
    pub commonalities: bool,
    pub route_processor: bool,
    pub gradient_finder: bool,
}

pub struct DataCreationPipeline<'a> {
    pub dependencies: &'a mut Dependencies<'a>,
}

impl<'a> DataCreationPipeline<'a> {
    const CC: &str = "Pipeline";

    pub fn new(dependencies: &'a mut Dependencies<'a>) -> Self {
        Self { dependencies }
    }

    pub async fn start(&self, options: &PipelineOptions) {
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
            self.run_route_processor(self.dependencies.gc_db().get_routes(self.dependencies.athlete_id()).await)
                .await;
        }

        if options.gradient_finder {
            // 3 //
            // Find climbs and descends
        }
    }

    async fn run_commonalities(&self) {
        Benchmark::start("commonalities");

        let mut processor: Commonality = Default::default();
        let mut telemetries = self.dependencies.strava_db().get_telemetry(self.dependencies.athlete_id()).await;

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
            route.athlete_id = self.dependencies.athlete_id();
            self.dependencies.gc_db().update_route(&route).await;
        }
    }

    async fn run_route_processor(&self, mut routes: mongodb::Cursor<Route>) {
        let mut discovered_segments: HashMap<DocumentId, Segment> = HashMap::new();
        let mut discovered_efforts: Vec<Effort> = Vec::new();

        while routes.advance().await.unwrap() {
            let mut route = routes.deserialize_current().unwrap();

            let mut activities = self.dependencies.strava_db()
                .get_athlete_activities_with_ids(self.dependencies.athlete_id(), &route.activities)
                .await;

            let mut max_distance = 0.0;

            while activities.advance().await.unwrap() {
                let activity = activities.deserialize_current().unwrap();

                // Determine which matched activity is the longest and set it to master
                if activity.distance > max_distance {
                    max_distance = activity.distance;
                    route.master_activity_id = activity._id as DocumentId;
                }

                // Populate efforts collection
                activity.segment_efforts.iter().for_each(|effort| {
                    // Extract segment
                    discovered_efforts.push(Effort {
                        _id: effort.id as f64,
                        athlete_id: effort.athlete.id,
                        segment_id: effort.segment.id,
                        activity_id: effort.activity.id,

                        moving_time: effort.moving_time,

                        //TODO
                        distance_from_start: 0,
                    })
                });
            }

            if let Some(master_activity) = self.dependencies.strava_db().get_activity(route.master_activity_id).await {
                // Extract data from master activity and put it into route
                route.polyline = master_activity.map.polyline;
                route.climb_per_km =
                    master_activity.total_elevation_gain / master_activity.distance;

                // Populate segment_ids from master activity
                for effort_in_master in master_activity.segment_efforts {
                    // Extract segment
                    let seg_id = effort_in_master.segment.id as DocumentId;

                    if let Some(db_segment) = self.dependencies.strava_db().get_segment(seg_id).await {
                        discovered_segments.insert(
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
                        logln!("Cannot find segment id: {}", seg_id);
                    }
                }
            }

            self.dependencies.gc_db().update_route(&route).await;
        }

        for segment in discovered_segments {
            self.dependencies.gc_db().update_segment(&segment.1).await;
        }

        for effort in discovered_efforts {
            self.dependencies.gc_db().update_effort(&effort).await;
        }
    }
}
