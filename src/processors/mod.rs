use geo_types::Coord;
use mongodb::bson::doc;
use std::collections::HashMap;

use crate::{
    data_types::{
        common::{DocumentId, Identifiable},
        gc::effort::Effort,
        strava::athlete::AthleteId,
    },
    util::{
        facilities::{Facilities, Required},
        geo::GeoUtils,
    },
};

use self::{commonality::Commonality};

pub mod commonality;
pub mod gradient_finder;

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

        if false {
            // 0 //
            // Add location city and country from first segment effort
            self.run_location_fixer_activities().await;
            
            // Fix string dates to DateTime
            self.run_date_fixer_activities().await;
            return;
        }

        if false {
            // 0 //
            // Remap indexes of segments from the whole telemetry to the polyline's telemetry
            // special procedure for re-writing activities
            self.run_indexes_remapper().await;
            return;
        }

        if options.commonalities {
            // 1 //
            // Clear previous results and store latest ones
            self.dependencies.gc_db().clear_routes().await;

            // Run route matching module => routes collections
            self.run_commonalities().await;
        }

        if options.route_processor {
            // 2 //
            // Clear previous efforts
            self.dependencies.gc_db().clear_efforts().await;

            // Using created routes, run RouteProcessor => segments collections, updates route collection
            self.run_route_processor().await;
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

            if items_to_process >= 550 {
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
            route.r#type = "Route".to_string();
            route.r#type.push_str(&master_activity.r#type.to_string());

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
            route.dist_from_capital =
                GeoUtils::distance(route.center_coord, Coord::from((26.096306, 44.439663))) as i32
                    / 100;

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
                            activity_id: effort.activity.id,
                            start_index: effort.start_index,
                            end_index: effort.end_index,
                            distance: 0.0,
                        });
                });

                // Run GradientFinder
                if activity.as_i64() == route.master_activity_id {
                    let mut gradients = gradient_finder::GradientFinder::find_gradients(&telemetry);

                    if gradients.len() > 0 {
                        let remapped_indexes =
                            GeoUtils::get_index_mapping(&route.polyline, &telemetry.latlng.data);
                        gradients.iter_mut().for_each(|gradient| {
                            // Search through the segment efforts and find a matching start to fill the location data
                            gradient.location_city = route.location_city.clone();
                            gradient.location_country = Some(route.location_country.clone());

                            for effort in &activity.segment_efforts {
                                if gradient.start_index >= effort.start_index as usize {
                                    gradient.location_city = effort.segment.city.clone();
                                    gradient.location_country = effort.segment.country.clone();
                                }
                            }

                            // Rewrite indexes with the remapped ones
                            gradient.start_index = remapped_indexes[gradient.start_index];
                            gradient.end_index = remapped_indexes[gradient.end_index];
                        });

                        route.gradients = gradients;
                    }
                }
            }

            self.dependencies.gc_db().update_route(&route).await;

            for effort in efforts_in_matched_activities {
                self.dependencies.gc_db().update_effort(&effort).await;
            }
        }
    }

    async fn run_indexes_remapper(&self) {
        let athlete_acts = self
            .dependencies
            .strava_db()
            .get_athlete_activity_ids(self.athlete_id)
            .await;

        for act_id in athlete_acts {
            let activity = self
                .dependencies
                .strava_db()
                .get_activity(act_id)
                .await
                .unwrap();

            let mut needs_remapping = true;

            for effort in &activity.segment_efforts {
                if let Some(_) = effort.start_index_poly {
                    needs_remapping = false;
                }
            }

            if !needs_remapping {
                continue;
            }

            println!("Remapping {}", act_id);

            let telemetry = self
                .dependencies
                .strava_db()
                .get_telemetry_by_id(activity.as_i64())
                .await
                .unwrap();

            let remapped_indexes: Vec<usize> =
                GeoUtils::get_index_mapping(&activity.map.polyline, &telemetry.latlng.data);

            for mut effort in activity.segment_efforts {
                effort.start_index_poly =
                    Some(remapped_indexes[effort.start_index as usize] as i32);
                effort.end_index_poly = Some(remapped_indexes[effort.end_index as usize] as i32);

                self.dependencies
                    .strava_db()
                    .update_activity_field(
                        "segment_efforts.id".to_owned(),
                        effort.id,
                        "segment_efforts.$.start_index_poly",
                        &effort.start_index_poly,
                    )
                    .await
                    .unwrap();

                self.dependencies
                    .strava_db()
                    .update_activity_field(
                        "segment_efforts.id".to_owned(),
                        effort.id,
                        "segment_efforts.$.end_index_poly",
                        &effort.end_index_poly,
                    )
                    .await
                    .unwrap();
            }
        }
    }

    async fn run_date_fixer_activities(&self) {
        // Query for adding a date field from String field
        let query = doc! {
          "$addFields":
          {
            "start_date_local_date": {
              "$dateFromString": {
                "dateString": "$start_date_local",
                "onError": "null"
              }
            }
          }
        };

        let fixed_activities = self
            .dependencies
            .strava_db()
            .query_activities(vec![query])
            .await;

        let mut count = 0;
        for activity in fixed_activities {
            println!(
                "Fixing {:?} with {:?}",
                activity._id, activity.start_date_local_date
            );

            self.dependencies
                .strava_db()
                .update_activity_field(
                    "_id".to_owned(),
                    activity._id,
                    "start_date_local_date",
                    &activity.start_date_local_date,
                )
                .await;

            count += 1;
        }

        println!("Fixed {}", count);
    }

    async fn run_location_fixer_activities(&self) {
        let athlete_acts = self
            .dependencies
            .strava_db()
            .get_athlete_activity_ids(self.athlete_id)
            .await;

        for act_id in athlete_acts {
            println!("Fixing {}", act_id);

            let mut activity = self
                .dependencies
                .strava_db()
                .get_activity(act_id)
                .await
                .unwrap();

            activity.segment_efforts.iter().for_each(|effort| {
                if let Some(effort_city) = &effort.segment.city {
                    activity.location_city = Some(effort_city.to_string());
                    return;
                }
            });

            activity.segment_efforts.iter().for_each(|effort| {
                if let Some(effort_country) = &effort.segment.country {
                    activity.location_country = effort_country.to_string();
                    return;
                }
            });

            self.dependencies
                .strava_db()
                .update_activity_field(
                    "_id".to_owned(),
                    activity._id,
                    "location_city",
                    &activity.location_city,
                )
                .await;

            self.dependencies
                .strava_db()
                .update_activity_field(
                    "_id".to_owned(),
                    activity._id,
                    "location_country",
                    &activity.location_country,
                )
                .await;
        }
    }
}
