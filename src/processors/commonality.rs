use std::{
    collections::{HashMap, HashSet},
    mem::swap,
};

use crate::data_types::{
    common::Identifiable,
    telemetry::{Telemetry, TelemetryId},
};

type LatLngReduced = u32;

const DIGIT_ACCURACY: f32 = 3.5;

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
            self.data.insert(v._id as i64, v);
        });
    }

    pub fn execute(&self) {
        type ActivityTypeData = HashMap<LatLngReduced, HashMap<LatLngReduced, PointOccurence>>;

        let mut unique_points: HashMap<String, ActivityTypeData> = HashMap::new();
        let mut act_to_act_points: HashMap<TelemetryId, HashMap<TelemetryId, u32>> = HashMap::new();

        for (_, act_telemetry_ref) in &self.data {
            let act_id = act_telemetry_ref.id();

            for latlngs in &act_telemetry_ref.latlng.data {
                let lat = latlngs[0];
                let long = latlngs[1];

                let reduced_lat = Commonality::reduce_accuracy(lat);
                let reduced_long = Commonality::reduce_accuracy(long);

                let set_telem_ids = &mut unique_points
                    .entry(act_telemetry_ref.r#type.to_string())
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

        let mut results : Vec<(i32, TelemetryId, TelemetryId)> = Vec::new();

        act_to_act_points.iter().for_each(|(src, dest_map)| {
            dest_map.iter().for_each(|(dest, count)| {
                results.push(((100.0 * (*count as f32 / self.data[src].latlng.data.len() as f32)) as i32, *src, *dest));
                results.push(((100.0 * (*count as f32 / self.data[dest].latlng.data.len() as f32)) as i32, *dest, *src));
            })
        });

        results.sort_by_key(|k| std::cmp::Reverse(100 * k.0 as i32));

        for res in results {
            println!("{}% {} {}", res.0, res.1, res.2);
        }
    }

    fn reduce_accuracy(value: f32) -> u32 {
        let multiplier: f32 = 10_i32.pow(DIGIT_ACCURACY.floor() as u32) as f32;

        let delta = DIGIT_ACCURACY - DIGIT_ACCURACY.floor();
        let adj = if delta != 0.0 {10.0} else {1.0};

        (((value * multiplier).floor() + delta) * adj).floor() as u32
    }
}
