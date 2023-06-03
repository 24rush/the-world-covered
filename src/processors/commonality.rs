use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
};

use crate::data_types::{
    common::{DocumentId, Identifiable},
    gc::route::Route,
    strava::telemetry::Telemetry,
};

// PUBLIC Types
pub type MatchedRoutesResult = Vec<Route>;

// INTERNAL Types
type LatLngReduced = u32;
type ActivityTypeData = HashMap<LatLngReduced, HashMap<LatLngReduced, ActivityOccurence>>;

type Telem2TelemMatchResult = (i32, DocumentId, DocumentId);
type MatchedTelemetriesResult = Vec<HashSet<DocumentId>>;

const DIGIT_ACCURACY: f32 = 3.5;
const MATCH_THRESHOLD: i32 = 85;

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

    points_total: u32,
    acts_total: u32,
    acts_unique: HashSet<DocumentId>,
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
        }

        let results = self.generate_match_results();
        self.merge_results(&results)
    }

    // Functions using streamed data
    pub fn process(&mut self, telemetry_ref: &Telemetry) {
        let src_id = telemetry_ref.as_i64();
        let telem_data_size = telemetry_ref.latlng.data.len();

        self.acts_total += 1;
        self.acts_unique.insert(src_id);

        *self.data_size.entry(src_id).or_insert(telem_data_size) = telem_data_size;

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

            set_telem_ids.insert(src_id);

            let vec_act_ids: Vec<&i64> = set_telem_ids.iter().collect();

            for i in 0..vec_act_ids.len() {
                let dest_act = *vec_act_ids[i];

                if src_id == dest_act {
                    //continue;
                }

                self.act_to_act_points
                    .entry(src_id)
                    .or_insert(HashMap::from([(dest_act, 0)]))
                    .entry(dest_act)
                    .and_modify(|v| *v += 1);
            }
        }
    }

    pub fn end_session(&mut self) -> MatchedRoutesResult {
        println!("Processed {} {}", self.acts_total, self.acts_unique.len());

        self.acts_total = 0;
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

                    ..Default::default()
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
        //println!("{:#?}", results);

        let mut merge_result: MatchedTelemetriesResult = Vec::new(); // Vector of unique IDs

        type GroupId = DocumentId;

        let mut act_to_group: HashMap<DocumentId, GroupId> = HashMap::new(); // Existing activity to which group it is allocated
        let mut groups: HashMap<GroupId, Cell<HashSet<DocumentId>>> = HashMap::new(); // Group composition

        let mut group_id_counter: GroupId = 1;

        for two_telem_result in results {
            let src_act_id = two_telem_result.1;
            let dest_act_id = two_telem_result.2;

            let src_group_id = *(act_to_group.get(&src_act_id).unwrap_or(&0));
            let dest_group_id = *(act_to_group.get(&dest_act_id).unwrap_or(&0));

            let src_has_group = src_group_id != 0;
            let dest_has_group = dest_group_id != 0;

            if two_telem_result.0 < MATCH_THRESHOLD {
                if !src_has_group {
                    let group_members = groups.entry(group_id_counter).or_default();
                    group_members.get_mut().insert(src_act_id);
                    act_to_group.insert(src_act_id, group_id_counter);

                    group_id_counter += 1;
                }

                if !dest_has_group {
                    let group_members = groups.entry(group_id_counter).or_default();
                    group_members.get_mut().insert(dest_act_id);
                    act_to_group.insert(dest_act_id, group_id_counter);

                    group_id_counter += 1;
                }

                continue;
            }

            if !src_has_group && !dest_has_group {
                // None exist - create new group with just the 2 of them
                act_to_group.insert(src_act_id, group_id_counter);
                act_to_group.insert(dest_act_id, group_id_counter);

                let group_members = groups.entry(group_id_counter).or_default();

                group_members.get_mut().insert(src_act_id);
                group_members.get_mut().insert(dest_act_id);

                group_id_counter += 1;
            } else if src_has_group && dest_has_group {
                // Both are in groups
                if src_group_id != dest_group_id {
                    // Merge src and dest groups
                    let dest_group = groups.get(&dest_group_id).unwrap().take();
                    let src_group = groups.get_mut(&src_group_id).unwrap();

                    for dest_member_id in dest_group.iter() {
                        src_group.get_mut().insert(*dest_member_id);
                        act_to_group.insert(*dest_member_id, src_group_id);
                    }
                }
            } else if src_has_group {
                groups
                    .entry(src_group_id)
                    .or_default()
                    .get_mut()
                    .insert(dest_act_id);
                act_to_group.insert(dest_act_id, src_group_id);
            } else if dest_has_group {
                groups
                    .entry(dest_group_id)
                    .or_default()
                    .get_mut()
                    .insert(src_act_id);
                act_to_group.insert(src_act_id, dest_group_id);
            }
        }

        let mut count = 0;
        let mut resulting_set: HashSet<DocumentId> = HashSet::new();

        for (_, group) in groups {
            let group = group.take();

            if group.len() == 0 {
                continue;
            }

            for item in &group {
                resulting_set.insert(*item);
            }

            merge_result.push(HashSet::from_iter(group.iter().cloned()));
            count += group.len();
        }

        println!("{}", count);
        let diff = self.acts_unique.difference(&resulting_set);
        println!("diff {:?}", diff);
        merge_result
    }

    fn reduce_accuracy(value: f32) -> u32 {
        let multiplier: f32 = 10_i32.pow(DIGIT_ACCURACY.floor() as u32) as f32;

        let delta = DIGIT_ACCURACY - DIGIT_ACCURACY.floor();
        let adj = if delta != 0.0 { 10.0 } else { 1.0 };

        (((value * multiplier).floor() + delta) * adj).floor() as u32
    }
}
