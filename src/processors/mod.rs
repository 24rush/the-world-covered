use geo_types::Coord;
use std::collections::HashMap;

use crate::{
    data_types::{
        common::{DocumentId, Identifiable},
        gc::{effort::Effort, segment::Segment},
        strava::athlete::AthleteId,
    },
    logln,
    util::{facilities::{Facilities, Required}, geo::GeoUtils},
};

use self::{commonality::Commonality, statistics::Statistics};

pub mod commonality;
pub mod gradient_finder;
pub mod statistics;

pub struct DataCreationPipelineOptions {
    pub commonalities: bool,
    pub route_processor: bool,
    pub statistics: bool,
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

    pub async fn start(&'a mut self, athlete_id: AthleteId, options: &DataCreationPipelineOptions) {
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
            // Clear previous segments and efforts
            self.dependencies.gc_db().clear_efforts().await;
            self.dependencies.gc_db().clear_segments().await;

            // Using created routes, run RouteProcessor => segments collections, updates route collection
            self.run_route_processor().await;
        }

        if options.statistics {
            // 3 //
            // Clear previous statistics

            // Create new statistics
            let statistics = Statistics::new(self.dependencies);

            let yearly_stats = statistics.collect_yearly_stats(self.athlete_id).await;
            logln!("Yearly stats {:?}", yearly_stats);
        }
    }

    async fn run_commonalities(&self) {
        let mut processor: Commonality = Default::default();
        let mut telemetries = self
            .dependencies
            .strava_db()
            .get_athlete_telemetries(self.athlete_id)
            .await;

        let mut items_to_process = 0;
        while telemetries.advance().await.unwrap() {
            let telemetry = telemetries.deserialize_current().unwrap();

            if telemetry.latlng.data.len() == 0 {
                continue;
            }

            processor.process(&telemetry);
            items_to_process += 1;

            if items_to_process >= 50 {
                break;
            }
        }

        for mut route in processor.end_session() {
            // Mark the owner of the route so we can retrieve them later in the following processors
            route.athlete_id = self.athlete_id;
            self.dependencies.gc_db().update_route(&route).await;
        }
    }

    async fn run_route_processor(&self) {
        struct EffortSegmentDetails {
            pub segment: Segment,
            pub distance: f32,
            pub activity_id: DocumentId,
            pub start_index: i32,
            pub end_index: i32,
        }

        let mut segments_in_matched_activities: HashMap<DocumentId, EffortSegmentDetails> =
            HashMap::new();

        let mut routes = self.dependencies.gc_db().get_routes(self.athlete_id).await;

        while routes.advance().await.unwrap() {
            let mut efforts_in_matched_activities: Vec<Effort> = Vec::new();
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
            route.polyline = master_activity.map.polyline.to_string();
            route.r#type = master_activity.r#type.to_string();
            route.distance = master_activity.distance;
            route.average_speed = master_activity.average_speed;
            route.total_elevation_gain = master_activity.total_elevation_gain;
            route.polyline = master_activity.map.polyline.to_string();
            route.description = Some(
                master_activity
                    .description
                    .unwrap_or("".to_string())
                    .to_string(),
            );
            route.location_city = master_activity.location_city;
            route.location_country = master_activity.location_country;

            let bbox = GeoUtils::get_bounding_box(&route.polyline);
            route.center_coord = GeoUtils::get_center_of_bbox(bbox.0, bbox.1);

            // Reference is Bucharest (coordinates opposite)
            route.dist_from_capital = GeoUtils::distance(route.center_coord, Coord::from((26.096306, 44.439663))) as i32 / 100;

            if let None = route.location_city {
                if let Some(effort) = master_activity.segment_efforts.get(0) {
                    if let Some(effort_city) = &effort.segment.city {
                        route.location_city = Some(effort_city.to_string());
                    }
                }
            }

            if route.location_country == "" {
                if let Some(effort) = master_activity.segment_efforts.get(0) {
                    if let Some(effort_country) = &effort.segment.country {
                        route.location_country = effort_country.to_string();
                    }
                }
            }

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
                    efforts_in_matched_activities.push(Effort {
                        _id: effort.id as f64,
                        athlete_id: effort.athlete.id,
                        segment_id: effort.segment.id,
                        activity_id: effort.activity.id,

                        moving_time: effort.moving_time,
                        start_date_local: effort.start_date_local.clone(),
                        distance_from_start: telemetry.distance.data[effort.start_index as usize],
                    });

                    // Update the segment if we found only a shorter one
                    let existing_segment = segments_in_matched_activities.get(&effort.segment.id);
                    if let Some(segment) = existing_segment {
                        if segment.distance < effort.segment.distance {
                            return;
                        }
                    }

                    segments_in_matched_activities
                        .entry(effort.segment.id)
                        .or_insert(EffortSegmentDetails {
                            segment: Segment {
                                _id: effort.segment.id as f64,
                                ..Default::default()
                            },
                            activity_id: effort.activity.id,
                            start_index: effort.start_index,
                            end_index: effort.end_index,
                            distance: 0.0,
                        });
                });

                if false && activity.as_i64() == route.master_activity_id {
                    let gradients = gradient_finder::GradientFinder::find_gradients(&telemetry);

                    if gradients.len() > 0 {
                        route.gradients = gradients;
                    }
                }
            }

            self.dependencies.gc_db().update_route(&route).await;

            for effort in efforts_in_matched_activities {
                self.dependencies.gc_db().update_effort(&effort).await;
            }
        }

        // Collect segment_ids from master activity's efforts (effort 1-1 segment)
        for segment_effort in &mut segments_in_matched_activities {
            let seg_id = *segment_effort.0 as DocumentId;
            let mut seg_details = segment_effort.1;

            // If the segment is already in the strava DB, pick up the info from there
            if let Some(db_segment) = self.dependencies.strava_db().get_segment(seg_id).await {
                seg_details.distance = db_segment.distance;
                seg_details.segment.polyline = db_segment.map.polyline.to_string();
            } else {
                // Get the telemetry for the activity and compute the polyline
                let telemetry = self
                    .dependencies
                    .strava_db()
                    .get_telemetry_by_id(seg_details.activity_id)
                    .await
                    .unwrap();

                let telemetry_data = &telemetry.latlng.data;
                let coordinates: Vec<Coord> = (seg_details.start_index as usize
                    ..=seg_details.end_index as usize)
                    .map(|index| {
                        let x = telemetry_data[index][1] as f64;
                        let y = telemetry_data[index][0] as f64;

                        Coord { x, y }
                    })
                    .collect();

                seg_details.segment.polyline =
                    polyline::encode_coordinates(coordinates, 5).unwrap_or("".to_string());
            }
        }

        for segment in segments_in_matched_activities {
            self.dependencies
                .gc_db()
                .update_segment(&segment.1.segment)
                .await;
        }
    }
}
