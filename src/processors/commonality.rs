use std::{
    collections::{HashMap, HashSet}, io::Write
};

use crate::data_types::{
    common::Identifiable,
    telemetry::{Telemetry, TelemetryId},
};

type LatLngReduced = u32;
type Telem2TelemMatchResult = (i32, TelemetryId, TelemetryId);
type MatchedTelemetriesResult = Vec<HashSet<TelemetryId>>;

const DIGIT_ACCURACY: f32 = 3.5;
const MATCH_THRESHOLD: i32 = 90;

#[derive(Default, Debug)]
struct PointOccurence {
    pub acts: HashSet<TelemetryId>,
}

impl PointOccurence {
    pub fn new() -> Self {
        Self {
            acts: HashSet::new(),
        }
    }
}

pub struct Commonality<'a> {
    data: HashMap<TelemetryId, &'a Telemetry>,
}

impl<'a> Commonality<'a> {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn set_data(&mut self, data: Vec<&'a Telemetry>) {
        data.iter().for_each(|v| {
            self.data.insert(v.id(), v);
        });
    }

    pub fn execute(&self) -> MatchedTelemetriesResult {
        type ActivityTypeData = HashMap<LatLngReduced, HashMap<LatLngReduced, PointOccurence>>;

        let mut unique_points: HashMap<String, ActivityTypeData> = HashMap::new();
        let mut act_to_act_points: HashMap<TelemetryId, HashMap<TelemetryId, u32>> = HashMap::new();

        let mut act_count_processed = 1.0;

        for (_, telemetry_ref) in &self.data {
            let act_id = telemetry_ref.id();
                    
            print!("\rProcessing {:.0}%", 100.0 * act_count_processed / (self.data.len() as f32));
            std::io::stdout().flush().unwrap();
            act_count_processed += 1.0;

            for latlngs in &telemetry_ref.latlng.data {
                let lat = latlngs[0];
                let long = latlngs[1];

                let reduced_lat = Commonality::reduce_accuracy(lat);
                let reduced_long = Commonality::reduce_accuracy(long);

                let set_telem_ids = &mut unique_points
                    .entry(telemetry_ref.r#type.to_string())
                    .or_default()
                    .entry(reduced_lat)
                    .or_insert(HashMap::from([(reduced_long, PointOccurence::new())]))
                    .entry(reduced_long)
                    .or_default()
                    .acts;

                set_telem_ids.insert(act_id);

                let vec_act_ids: Vec<&i64> = set_telem_ids.iter().collect();

                if vec_act_ids.len() > 1 {
                    for i in 0..vec_act_ids.len() {
                        let dest_act = *vec_act_ids[i];

                        if act_id == dest_act {
                            continue;
                        }

                        act_to_act_points
                            .entry(act_id)
                            .or_insert(HashMap::from([(dest_act, 0)]))
                            .entry(dest_act)
                            .and_modify(|v| *v += 1);
                    }
                }
            }
        }

        print!("\r");

        let mut results : Vec<Telem2TelemMatchResult> = Vec::new();

        let compute_match_percent = |src, count| -> i32 {
            let data_len = self.data[src].latlng.data.len() as f32;
            let clamp_count = if count as f32 > data_len {data_len} else {count as f32};

            (100.0 * (clamp_count / data_len)) as i32
        };

        act_to_act_points.iter().for_each(|(src, dest_map)| {
            dest_map.iter().for_each(|(dest, count)| {
                results.push((compute_match_percent(src, *count), *src, *dest));
                results.push((compute_match_percent(dest, *count), *dest, *src));
            })
        });

        results.sort_by_key(|k| std::cmp::Reverse(100 * k.0 as i32));

        let merge_result = self.merge_results(&results);

        merge_result.iter().for_each(|v| println!("{:?}", v));

        merge_result
    }

    fn merge_results(&self, results: &Vec<Telem2TelemMatchResult>) -> MatchedTelemetriesResult {
        let mut merge_result : MatchedTelemetriesResult = Vec::new();

        for two_telem_result in results {
            if two_telem_result.0 < MATCH_THRESHOLD {
                continue;
            }

            let mut group_found = false;

            for match_group in merge_result.iter_mut() {
                if match_group.contains(&two_telem_result.1) || match_group.contains(&two_telem_result.2) {
                    group_found = true;

                    match_group.insert(two_telem_result.1);
                    match_group.insert(two_telem_result.2);
                }
            }

            if !group_found {
                merge_result.push(HashSet::from([two_telem_result.1, two_telem_result.2]));
            }
        }

        merge_result
    }

    fn reduce_accuracy(value: f32) -> u32 {
        let multiplier: f32 = 10_i32.pow(DIGIT_ACCURACY.floor() as u32) as f32;

        let delta = DIGIT_ACCURACY - DIGIT_ACCURACY.floor();
        let adj = if delta != 0.0 {10.0} else {1.0};

        (((value * multiplier).floor() + delta) * adj).floor() as u32
    }
}
