use std::collections::{HashMap, HashSet};

use crate::{
    data_types::{
        common::{DocumentId, Identifiable},
        gc::route::Route,
        strava::telemetry::Telemetry,
    },
    logln, logsl,
};

// PUBLIC Types
pub type MatchedRoutesResult = Vec<Route>;

// INTERNAL Types
type LatLngReduced = u32;
type ActivityTypeData = HashMap<LatLngReduced, HashMap<LatLngReduced, ActivityOccurence>>;

type Telem2TelemMatchResult = (i32, DocumentId, DocumentId);
type MatchedTelemetriesResult = Vec<HashSet<DocumentId>>;

const DIGIT_ACCURACY: f32 = 3.5;
const MATCH_THRESHOLD: i32 = 90;

#[derive(Default, Debug)]
struct ActivityOccurence {
    pub set_activities: HashSet<DocumentId>,
}

#[derive(Default)]
pub struct Commonality<'a> {
    data: HashMap<DocumentId, &'a Telemetry>,
    data_size: HashMap<DocumentId, usize>,

    unique_points: HashMap<String, ActivityTypeData>,
    act_to_act_points: HashMap<DocumentId, HashMap<DocumentId, u32>>,

    act_count_processed: f32,
    points_total: u32,
}

impl<'a> Commonality<'a> {
    const CC: &str = "Commonality";

    // Functions using full set of data
    pub fn set_data(&mut self, data: Vec<&'a Telemetry>) {
        data.iter().for_each(|v| {
            self.data.insert(v.as_i64(), v);
        });
    }

    pub fn execute(&mut self) -> MatchedTelemetriesResult {
        let references: Vec<&Telemetry> = self.data.iter().map(|(_, refs)| *refs).collect();

        for telemetry_ref in &references {
            self.process(&telemetry_ref);
            self.act_count_processed += 1.0;
        }

        let results = self.generate_match_results();
        self.merge_results(&results)
    }

    // Functions using streamed data
    pub fn process(&mut self, telemetry_ref: &Telemetry) {
        let telem_id = telemetry_ref.as_i64();
        let telem_data_size = telemetry_ref.latlng.data.len();

        *self.data_size.entry(telem_id).or_insert(telem_data_size) = telem_data_size;

        for latlngs in &telemetry_ref.latlng.data {
            self.points_total += 1;
            let lat = latlngs[0];
            let long = latlngs[1];

            let reduced_lat = Commonality::reduce_accuracy(lat);
            let reduced_long = Commonality::reduce_accuracy(long);

            let set_telem_ids = &mut self
                .unique_points
                .entry(telemetry_ref.r#type.to_string())
                .or_default()
                .entry(reduced_lat)
                .or_insert(HashMap::from([(
                    reduced_long,
                    ActivityOccurence::default(),
                )]))
                .entry(reduced_long)
                .or_default()
                .set_activities;

            set_telem_ids.insert(telem_id);

            let vec_act_ids: Vec<&i64> = set_telem_ids.iter().collect();

            for i in 0..vec_act_ids.len() {
                let dest_act = *vec_act_ids[i];

                if telem_id == dest_act {
                    continue;
                }

                self.act_to_act_points
                    .entry(telem_id)
                    .or_insert(HashMap::from([(dest_act, 0)]))
                    .entry(dest_act)
                    .and_modify(|v| *v += 1);
            }
        }
    }

    pub fn end_session(&mut self) -> MatchedRoutesResult {
        logln!("Processed {} activities", { self.act_count_processed });

        let results = self.generate_match_results();
        let merged_routes = self.merge_results(&results);

        let mut route_idx = 0;
        merged_routes
            .iter()
            .map(|result| {
                route_idx += 1;

                Route {
                    _id: route_idx as f64,
                    activities: result.iter().map(|act_id| *act_id).collect(),
                    athlete_id: 0,
                    master_activity_id: 0,
                    polyline: "".to_string(),
                    segment_ids: Vec::new(),
                }
            })
            .collect()
    }

    // PRIVATES
    fn generate_match_results(&mut self) -> Vec<Telem2TelemMatchResult> {
        let mut results: Vec<Telem2TelemMatchResult> = Vec::new();

        let compute_match_percent = |src, count| -> i32 {
            let data_len = self.data_size[src] as f32;
            let clamp_count = if count as f32 > data_len {
                data_len
            } else {
                count as f32
            };

            (100.0 * (clamp_count / data_len)) as i32
        };

        self.act_to_act_points.iter().for_each(|(src, dest_map)| {
            dest_map.iter().for_each(|(dest, count)| {
                results.push((compute_match_percent(src, *count), *src, *dest));
                results.push((compute_match_percent(dest, *count), *dest, *src));
            })
        });

        results.sort_by_key(|k| std::cmp::Reverse(100 * k.0 as i32));

        results
    }

    fn merge_results(&self, results: &Vec<Telem2TelemMatchResult>) -> MatchedTelemetriesResult {
        let mut merge_result: MatchedTelemetriesResult = Vec::new();

        for two_telem_result in results {
            if two_telem_result.0 < MATCH_THRESHOLD {
                continue;
            }

            let mut group_found = false;

            for match_group in merge_result.iter_mut() {
                if match_group.contains(&two_telem_result.1)
                    || match_group.contains(&two_telem_result.2)
                {
                    group_found = true;

                    match_group.insert(two_telem_result.1);
                    match_group.insert(two_telem_result.2);
                }
            }

            if !group_found {
                merge_result.push(HashSet::from([two_telem_result.1, two_telem_result.2]));
            }
        }

        merge_result.iter().for_each(|v| logln!("{:?}", v));

        merge_result
    }

    fn reduce_accuracy(value: f32) -> u32 {
        let multiplier: f32 = 10_i32.pow(DIGIT_ACCURACY.floor() as u32) as f32;

        let delta = DIGIT_ACCURACY - DIGIT_ACCURACY.floor();
        let adj = if delta != 0.0 { 10.0 } else { 1.0 };

        (((value * multiplier).floor() + delta) * adj).floor() as u32
    }
}
