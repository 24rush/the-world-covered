use geo_types::Coord;
use std::collections::HashSet;

use crate::{
    data_types::{common::DocumentId, strava::athlete::AthleteId},
    util::{
        facilities::{Facilities, Required},
        geo::GeoUtils,
    },
};

use self::commonality::Commonality;

pub mod commonality;
pub mod gradient_finder;

#[derive(PartialEq, Default)]
pub enum SubOperationType {
    #[default]
    None,
    Update,
    Rewrite,
}

#[derive(PartialEq, Default)]
pub enum PipelineOperationType {
    Enabled(SubOperationType),

    #[default]
    Disabled,
}

pub struct DataCreationPipelineOptions {
    pub commonalities: PipelineOperationType,
    pub route_processor: PipelineOperationType,
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

        if options.commonalities != PipelineOperationType::Disabled {
            // 1 //
            // Run route matching module => update routes collections
            if options.commonalities == PipelineOperationType::Enabled(SubOperationType::Rewrite) {
                // Clear previous results and store latest ones
                self.dependencies.gc_db().routes.clear_routes().await;
                self.run_rewrite_commonalities().await;
            }

            if options.commonalities == PipelineOperationType::Enabled(SubOperationType::Update) {
                self.run_update_commonalities().await;
            }
        }

        if options.route_processor == PipelineOperationType::Enabled(SubOperationType::None) {
            // 2 //
            // Using created routes, run RouteProcessor => efforts collections, updates route collection
            self.run_route_processor().await;
        }
    }

    async fn run_update_commonalities(&self) {
        //DEBUG
        let mut allocated_activities = 0;

        let mut processor: Commonality = Default::default();
        let mut already_grouped_act_ids: HashSet<DocumentId> = HashSet::new();

        {
            let mut existing_routes = self
                .dependencies
                .gc_db()
                .routes
                .get_athlete_routes(self.athlete_id)
                .await;

            // Read routes and determine the activity IDs already matched
            while existing_routes.advance().await.unwrap() {
                let route = existing_routes.deserialize_current().unwrap();

                // Collect already matched activities
                route.activities.iter().for_each(|ma| {
                    already_grouped_act_ids.insert(*ma);
                })
            }
        }

        {
            // Update the last index of the processor so it starts creating new routes from there onwards
            let last_route_idx = self
                .dependencies
                .gc_db()
                .routes
                .get_last_route_id(self.athlete_id)
                .await
                .unwrap();
            processor.set_set_first_route_index(last_route_idx + 1);
        }

        {
            // Load processor with the missing activities' telemetry
            let all_activity_ids = self
                .dependencies
                .strava_db()
                .activities
                .get_athlete_activity_ids(self.athlete_id)
                .await;
            let all_activities: HashSet<DocumentId> = HashSet::from_iter(all_activity_ids);

            // Find which activities are not matched by subtracting matched ones from all activities
            let missing_activity_ids: Vec<&DocumentId> = all_activities
                .difference(&already_grouped_act_ids)
                .collect();

            let mut count_missing = 0;
            for index in 0..missing_activity_ids.len() {
                let missing_activity_id = missing_activity_ids[index];

                let telemetry_missing = self
                    .dependencies
                    .strava_db()
                    .telemetries
                    .get(*missing_activity_id)
                    .await
                    .unwrap();

                if processor.load_telemetry(&telemetry_missing) {
                    count_missing += 1;
                }
            }

            println!("Loaded {} missing activities", count_missing);
        }

        {
            // Take all unmatched activities and try to group them together
            // result should be groups which contain either merged activities or standalone ones
            let mut matched_missing_activities = processor.matched_routes();

            // Go over the routes again and try to match the master activity of the route with the activities in each group from the missing ones
            let mut existing_routes = self
                .dependencies
                .gc_db()
                .routes
                .get_athlete_routes(self.athlete_id)
                .await;

            while existing_routes.advance().await.unwrap() {
                let mut route = existing_routes.deserialize_current().unwrap();

                let telemetry_master = self
                    .dependencies
                    .strava_db()
                    .telemetries
                    .get(route.master_activity_id)
                    .await
                    .unwrap();

                processor.load_telemetry(&telemetry_master);

                // Go over the matched missing activities and test if they can be grouped with current route
                // - remove from list at first match
                for index_grouped_route in 0..matched_missing_activities.len() {
                    let missing_group =
                        matched_missing_activities.get(index_grouped_route).unwrap();

                    let is_matched =
                        processor.is_matched(route.master_activity_id, &missing_group.activities);

                    if is_matched {
                        println!(
                            "Missing group can be merged with {}",
                            route.master_activity_id
                        );

                        allocated_activities += missing_group.activities.len();

                        route
                            .activities
                            .extend(missing_group.activities.iter().cloned());

                        matched_missing_activities.remove(index_grouped_route as usize);
                        self.dependencies.gc_db().routes.update(&route).await;

                        break;
                    }
                }

                if matched_missing_activities.len() == 0 {
                    break;
                }
            }

            // Unmatched groups need to create new routes
            if matched_missing_activities.len() > 0 {
                println!(
                    "{} routes not merged and need to be created",
                    matched_missing_activities.len()
                );

                for mut unmatched_group in matched_missing_activities {
                    allocated_activities += unmatched_group.activities.len();
                    // Mark the owner of the route so we can retrieve them later in the following processors
                    unmatched_group.athlete_id = self.athlete_id;
                    self.dependencies
                        .gc_db()
                        .routes
                        .update(&unmatched_group)
                        .await;
                }
            }
        }

        println!("Update commonalities allocated {}", allocated_activities);
    }

    async fn run_rewrite_commonalities(&self) {
        let mut processor: Commonality = Default::default();

        let mut sorted_activities_cursor = self
            .dependencies
            .strava_db()
            .activities
            .get_athlete_activities_sorted_distance_asc(self.athlete_id)
            .await
            .unwrap();

        let mut items_to_process = 0;

        while sorted_activities_cursor.advance().await.unwrap() {
            let mut act_id: DocumentId = 0;

            let res_float = sorted_activities_cursor.current().get_f64("_id");

            if let Ok(id) = res_float {
                act_id = id as i64;
            } else {
                let res_int = sorted_activities_cursor.current().get_i32("_id");
                if let Ok(id) = res_int {
                    act_id = id as i64;
                }
            }

            let res_telemetry = self.dependencies.strava_db().telemetries.get(act_id).await;

            if let None = res_telemetry {
                continue;
            }

            if processor.load_telemetry(&res_telemetry.unwrap()) {
                items_to_process += 1;
            }

            if items_to_process >= 55 {
                //    break;
            }
        }

        for mut matched_route in processor.matched_routes() {
            // Mark the owner of the route so we can retrieve them later in the following processors
            matched_route.athlete_id = self.athlete_id;
            self.dependencies
                .gc_db()
                .routes
                .update(&matched_route)
                .await;
        }
    }

    async fn run_route_processor(&self) {
        struct EffortSegmentDetails {
            pub distance: f32,
            pub activity_id: DocumentId,
            pub start_index: i32,
            pub end_index: i32,
        }

        let mut routes = self
            .dependencies
            .gc_db()
            .routes
            .get_athlete_routes(self.athlete_id)
            .await;

        while routes.advance().await.unwrap() {
            let mut route = routes.deserialize_current().unwrap();

            // Find the master activity of this routes: the longest one from the matched ones
            let master_activity = self
                .dependencies
                .strava_db()
                .activities
                .get(route.master_activity_id)
                .await
                .unwrap();

            // Extract data from master activity and put it into route
            //route.master_activity_id = master_activity._id as DocumentId;
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

            // Get all matched activities and fill in all the efforts
            if let Some(mut activities) = self
                .dependencies
                .strava_db()
                .activities
                .get_athlete_activities_with_ids(self.athlete_id, &route.activities)
                .await
            {
                while activities.advance().await.unwrap() {
                    let activity = activities.deserialize_current().unwrap();
                    let act_id: DocumentId =
                        crate::data_types::common::Identifiable::as_i64(&activity);

                    let telemetry = self
                        .dependencies
                        .strava_db()
                        .telemetries
                        .get(act_id)
                        .await
                        .unwrap();

                    // Run GradientFinder
                    if act_id == route.master_activity_id {
                        let mut gradients =
                            gradient_finder::GradientFinder::find_gradients(&telemetry);

                        if gradients.len() > 0 {
                            let remapped_indexes = GeoUtils::get_index_mapping(
                                &route.polyline,
                                &telemetry.latlng.data,
                            );
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
            }

            self.dependencies.gc_db().routes.update(&route).await;
        }
    }
}
