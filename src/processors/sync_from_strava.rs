use mongodb::bson::doc;

use crate::{
    data_types::{
        common::Identifiable,
        strava::{activity::Activity, athlete::AthleteId},
    },
    logln, logvbln,
    util::{
        facilities::{Facilities, Required},
        geo::GeoUtils,
        DateTimeUtils,
    },
};

pub struct StravaDBSync {
    athlete_id: AthleteId,
    dependencies: Facilities,
}

impl StravaDBSync {
    const CC: &str = "Util";

    pub fn new(dependencies: Facilities, athlete_id: AthleteId) -> Self {
        dependencies.check(vec![Required::StravaDB, Required::StravaApi]);

        Self {
            athlete_id,
            dependencies,
        }
    }

    pub async fn download_activities_in_range(
        &mut self,
        after_ts: i64,
        before_ts: i64,
    ) -> (i64, bool) {
        logln!(
            "download from {} to {}",
            DateTimeUtils::timestamp_to_str(after_ts),
            DateTimeUtils::timestamp_to_str(before_ts)
        );

        const ACTIVITIES_PER_PAGE: usize = 2;

        let mut last_activity_ts = before_ts;
        let mut page = 1;
        let mut has_more_items = false;

        loop {
            if let Some(activities_list) = self
                .dependencies
                .strava_api()
                .list_athlete_activities(after_ts, before_ts, ACTIVITIES_PER_PAGE, page)
                .await
            {
                has_more_items = activities_list.len() == ACTIVITIES_PER_PAGE;

                for activity in activities_list {
                    let act_id = activity["id"].as_i64().unwrap();

                    last_activity_ts =
                        DateTimeUtils::zulu2ts(&activity["start_date"].as_str().unwrap());

                    self.dependencies
                        .strava_db()
                        .athletes
                        .save_after_before_timestamps(self.athlete_id, after_ts, last_activity_ts)
                        .await;

                    if self
                        .dependencies
                        .strava_db()
                        .activities
                        .exists(act_id)
                        .await
                    {
                        logvbln!("Activity {} already in DB. Skipping download.", act_id);

                        continue;
                    }

                    if let Some(mut new_activity) =
                        self.dependencies.strava_api().get_activity(act_id).await
                    {
                        self.dependencies
                            .strava_db()
                            .activities
                            .store(act_id, &mut new_activity)
                            .await;

                        let mut db_activity = self
                            .dependencies
                            .strava_db()
                            .activities
                            .get(act_id)
                            .await
                            .unwrap();

                        // Download telemetry streams
                        self.download_telemetry(&db_activity).await;

                        // Remap indexes of segments from the whole telemetry to the polyline's telemetry
                        // special procedure for re-writing activities
                        self.run_segment_poly_indexer(&mut db_activity).await;

                        // Add location city and country from first segment effort
                        self.run_location_fixer_activities(&mut db_activity).await;

                        // Fix string dates to DateTime
                        self.run_date_fixer_activities(&mut db_activity).await;
                    }
                }
            } else {
                logln!("No activities in range.")
            }

            if !has_more_items {
                break;
            }

            page += 1;
        }

        (last_activity_ts, has_more_items)
    }

    async fn download_telemetry(&mut self, activity: &Activity) {
        let act_id = activity.as_i64();
        logln!("Checking if telemetry exists for activity: {}", act_id);

        // Check telemetry for activity
        if !self
            .dependencies
            .strava_db()
            .telemetries
            .exists(act_id)
            .await
        {
            let act = self
                .dependencies
                .strava_db()
                .activities
                .get(act_id)
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
                    serde_json::json!({ "id": self.athlete_id }),
                );
                m.insert("type".to_string(), serde_json::Value::String(act.r#type));

                telemetry_json = serde_json::Value::Object(m);
                self.dependencies
                    .strava_db()
                    .telemetries
                    .store(act_id, &mut telemetry_json)
                    .await;
            }
        }
    }

    async fn run_segment_poly_indexer(&self, activity: &mut Activity) {
        let act_id = activity.as_i64();

        println!("Remapping {}", act_id);

        let telemetry = self
            .dependencies
            .strava_db()
            .telemetries
            .get(act_id)
            .await
            .unwrap();

        let mut indexes_in_polyline: Vec<usize> = vec![];
        let mut needs_poly_index_update = false;

        // If field already exists then skip (for new activities does not apply just in case code is run on existing ones)
        for effort in &activity.segment_efforts {
            if let None = effort.start_index_poly {
                indexes_in_polyline = GeoUtils::create_polyline_mapping_table(
                    &activity.map.polyline,
                    &telemetry.latlng.data,
                );

                needs_poly_index_update = true;
                break;
            }
        }

        for mut segment_effort in activity.segment_efforts.iter_mut() {
            if needs_poly_index_update {
                segment_effort.start_index_poly =
                    Some(indexes_in_polyline[segment_effort.start_index as usize] as i32);
                segment_effort.end_index_poly =
                    Some(indexes_in_polyline[segment_effort.end_index as usize] as i32);

                self.dependencies
                    .strava_db()
                    .activities
                    .set_segment_start_index_poly(
                        segment_effort.id,
                        &segment_effort.start_index_poly,
                    )
                    .await;

                self.dependencies
                    .strava_db()
                    .activities
                    .set_segment_end_index_poly(segment_effort.id, &segment_effort.end_index_poly)
                    .await;
            }

            self.dependencies
                .strava_db()
                .activities
                .set_segment_distance_from_start(
                    segment_effort.id,
                    telemetry.distance.data[segment_effort.start_index as usize],
                )
                .await;
        }
    }

    async fn run_date_fixer_activities(&self, activity: &mut Activity) {
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
            .activities
            .query_activities(vec![doc! {"$match": {"_id":activity._id}}, query])
            .await;

        for activity in fixed_activities {
            self.dependencies
                .strava_db()
                .activities
                .set_start_date_local_date(activity.as_i64(), &activity.start_date_local_date)
                .await;
        }
    }

    async fn run_location_fixer_activities(&self, activity: &mut Activity) {
        let act_id = activity.as_i64();

        println!("Fixing {}", act_id);

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
            .activities
            .set_location_city(activity.as_i64(), &activity.location_city)
            .await;

        self.dependencies
            .strava_db()
            .activities
            .set_location_country(activity.as_i64(), &activity.location_country)
            .await;
    }
}
