use std::collections::HashMap;

use futures_util::{TryStreamExt};

use crate::{
    data_types::{
        common::DocumentId,
        gc::{effort::Effort, route::Route, segment::Segment},
        strava::{athlete::AthleteId},
    },
    database::{gc_db::GCDB, strava_db::StravaDB},
    logln,
    util::time::Benchmark,
};

use self::commonality::Commonality;

pub mod commonality;
pub mod routes;

struct PipelineContext<'a> {
    athlete_id: AthleteId,
    strava_db: &'a StravaDB,
    gc_db: &'a GCDB,
}

pub struct Pipeline {}

impl Pipeline {
    const CC: &str = "Pipeline";

    pub async fn start(athlete_id: AthleteId, strava_db: &StravaDB, gc_db: &GCDB) {
        let pipe_context = PipelineContext {
            athlete_id,
            strava_db,
            gc_db,
        };

        // 1 //
        // Clear previous results and store latest ones
        gc_db.clear_routes().await;

        // Run route matching module => routes collections
        Pipeline::run_commonalities((&pipe_context).into()).await;

        // 2 //
        // Clear previous segments
        gc_db.clear_segments().await;

        // Using created routes, run RouteProcessor => segments collections, updates route collection
        Pipeline::run_route_processor(&pipe_context, gc_db.get_routes(athlete_id).await).await;
    }

    async fn run_commonalities(pipe_context: &PipelineContext<'_>) {
        let b = Benchmark::start("th");
        let mut telemetries = pipe_context
            .strava_db
            .get_telemetry(pipe_context.athlete_id)
            .await;

        let mut processor: Commonality = Default::default();

        let mut items_to_process = 0;
        while let Some(telemetry) = telemetries.try_next().await.unwrap() {
            if items_to_process >= 50 {
                break;
            }

            processor.process(&telemetry);
            items_to_process += 1;
        }

        for mut route in processor.end_session() {
            // Mark the routes as being the athlete's
            route.athlete_id = pipe_context.athlete_id;
            pipe_context.gc_db.update_route(&route).await;
        };

        logln!("{}", b);
    }

    async fn run_route_processor(
        pipe_context: &PipelineContext<'_>,
        mut routes: mongodb::Cursor<Route>,
    ) {
        let mut discovered_segments: HashMap<DocumentId, Segment> = HashMap::new();
        let mut discovered_efforts: Vec<Effort> = Vec::new();

        while let Some(mut route) = routes.try_next().await.unwrap() {
            let mut activities = pipe_context
                .strava_db
                .get_athlete_activities_with_ids(pipe_context.athlete_id, &route.activities)
                .await;

            let mut max_distance = 0.0;

            while let Some(activity) = activities.try_next().await.unwrap() {
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

            if let Some(master_activity) = pipe_context
                .strava_db
                .get_activity(route.master_activity_id)
                .await
            {
                // Extract data from master activity and put it into route
                route.polyline = master_activity.map.polyline;
                route.climb_per_km =
                    master_activity.total_elevation_gain / master_activity.distance;

                // Populate segment_ids from master activity
                for effort_in_master in master_activity.segment_efforts {
                    // Extract segment
                    let seg_id = effort_in_master.segment.id as DocumentId;

                    if let Some(db_segment) = pipe_context.strava_db.get_segment(seg_id).await {
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

            pipe_context.gc_db.update_route(&route).await;
        }

        for segment in discovered_segments {
            pipe_context.gc_db.update_segment(&segment.1).await;
        };

        for effort in discovered_efforts {
            pipe_context.gc_db.update_effort(&effort).await;
        };
    }
}
