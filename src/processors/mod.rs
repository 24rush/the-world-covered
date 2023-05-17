use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{
    data_types::{
        common::DocumentId,
        gc::{effort::Effort, route::Route, segment::Segment},
        strava::{athlete::AthleteId, telemetry::Telemetry},
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

    pub fn start(athlete_id: AthleteId, strava_db: &StravaDB, gc_db: &GCDB) {
        let pipe_context = PipelineContext {
            athlete_id,
            strava_db,
            gc_db,
        };

        // 1 //
        // Clear previous results and store latest ones
        gc_db.clear_routes();

        // Run route matching module => routes collections
        Pipeline::run_commonalities((&pipe_context).into());
 
        // 2 //
        // Clear previous segments
        gc_db.clear_segments();

        // Using created routes, run RouteProcessor => segments collections, updates route collection
        Pipeline::run_route_processor(&pipe_context, gc_db.get_routes(athlete_id));
    }

    fn run_commonalities(pipe_context: &PipelineContext) {
        let b = Benchmark::start("th");
        let telemetries = pipe_context
            .strava_db
            .get_telemetry(pipe_context.athlete_id);

        let mut processor: Commonality = Default::default();

        let mut items_to_process = 0;
        for telemetry in telemetries {
            if items_to_process >= 50 {
                break;
            }

            processor.process(&telemetry.unwrap());
            items_to_process += 1;
        }

        processor.end_session().iter_mut().for_each(|route| {
            // Mark the routes as being the athlete's
            route.athlete_id = pipe_context.athlete_id;
            pipe_context.gc_db.update_route(route);
        });

        logln!("{}", b);
    }

    fn run_route_processor(pipe_context: &PipelineContext, routes: mongodb::sync::Cursor<Route>) {
        let mut discovered_segments: HashMap<DocumentId, Segment> = HashMap::new();
        let mut discovered_efforts: Vec<Effort> = Vec::new();

        routes.for_each(|route_res| {
            if let Ok(mut route) = route_res {
                let activities = pipe_context
                    .strava_db
                    .get_athlete_activities_with_ids(pipe_context.athlete_id, &route.activities);

                let mut max_distance = 0.0;

                activities.for_each(|activity_res| {
                    if let Ok(activity) = activity_res {
                        // Determine which matched activity is the longest and set it to master
                        if activity.distance > max_distance {
                            max_distance = activity.distance;
                            route.master_activity_id = activity._id as DocumentId;
                            route.polyline = activity.map.polyline;
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
                                start_index: effort.start_index,
                                end_index: effort.end_index,
                            })
                        });
                    }
                });

                if let Some(master_activity) = pipe_context
                    .strava_db
                    .get_activity(route.master_activity_id)
                {
                    // Populate segment_ids from master activity
                    master_activity.segment_efforts.iter().for_each(|effort| {
                        // Extract segment
                        let seg_id = effort.segment.id as DocumentId;

                        if let Some(db_segment) = pipe_context.strava_db.get_segment(seg_id) {
                            discovered_segments.insert(
                                seg_id,
                                Segment {
                                    _id: seg_id as f64,
                                    polyline: db_segment.map.polyline,
                                    kom: db_segment.xoms.kom,
                                    qom: db_segment.xoms.qom,
                                },
                            );
                        } else {
                            logln!("Cannot find segment id: {}", seg_id);
                        }
                    });
                }

                pipe_context.gc_db.update_route(&route);
            }
        });

        discovered_segments.iter().for_each(|(_, segment)| {
            pipe_context.gc_db.update_segment(segment);
        });

        discovered_efforts.iter().for_each(|effort| {
            pipe_context.gc_db.update_effort(effort);
        });
    }
}
