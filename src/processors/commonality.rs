use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
};

use crate::{data_types::{
    common::{DocumentId, Identifiable},
    gc::route::Route,
    strava::telemetry::Telemetry,
}, util::geo::GeoUtils};

// PUBLIC Types
pub type MatchedRoutesResult = Vec<Route>;

// INTERNAL Type
type MatchIndex = i32;
type LatLngReduced = i32;
type ActivityTypeData = HashMap<LatLngReduced, HashMap<LatLngReduced, ActivityOccurence>>;

type Telem2TelemMatchResult = (MatchIndex, DocumentId, DocumentId);
type MatchedTelemetriesResult = Vec<HashSet<DocumentId>>;

const DIGIT_ACCURACY_MIN: f32 = 3.5;
const DIGIT_ACCURACY_MAX: f32 = 6.5;

const MATCH_THRESHOLD: MatchIndex = 85;

#[derive(Default, Debug)]
struct ActivityOccurence {
    pub set_activities: HashSet<DocumentId>,
}

#[derive(Default)]
pub struct Commonality<'a> {
    data: HashMap<DocumentId, &'a Telemetry>,
    activity_datapoints_count: HashMap<DocumentId, usize>,
    activity_lengths: HashMap<DocumentId, usize>,

    unique_points: HashMap<String, ActivityTypeData>,
    act_to_act_points: HashMap<DocumentId, HashMap<DocumentId, u32>>,

    points_total: u32,
    acts_total: u32,
    acts_unique: HashSet<DocumentId>,
    route_idx: DocumentId,
}

impl<'a> Commonality<'a> {
    const CC: &str = "Commonality";

    // Functions using full set of data
    pub fn set_data(&mut self, data: Vec<&'a Telemetry>) {
        data.iter().for_each(|v| {
            self.data.insert(v.as_i64(), v);
        });
    }

    // Used for update procedure so we know where to start creating new indexes from
    pub fn set_set_first_route_index(&mut self, start: DocumentId) {
        self.route_idx = start;
    }

    // Using stored data returnes the maximum match index between master_act_id and an activity from the dest set
    pub fn is_matched(&self, master_act_id: DocumentId, dest: &Vec<DocumentId>) -> bool {
        let results = self.generate_match_results();

        let mut max_match_index = 0;

        results.iter().for_each(|result| {
            if (result.1 == master_act_id && dest.contains(&result.2))
                || (result.2 == master_act_id && dest.contains(&result.1))
            {
                if result.0 > max_match_index {
                    max_match_index = result.0;
                }
            }
        });

        max_match_index >= MATCH_THRESHOLD
    }

    // Functions using streamed data
    pub fn load_telemetry(&mut self, telemetry: &Telemetry) -> bool {
        if telemetry.latlng.data.len() == 0 {
            return false;
        }

        let src_act_id = telemetry.as_i64();

        self.acts_total += 1;
        self.acts_unique.insert(src_act_id);

        self.activity_datapoints_count
            .entry(src_act_id)
            .or_insert(telemetry.latlng.data.len());

        self.activity_lengths
            .entry(src_act_id)
            .or_insert(telemetry.distance.data[telemetry.distance.data.len() - 1] as usize);

        for latlngs in &telemetry.latlng.data {
            self.points_total += 1;
            let lat = latlngs[0];
            let long = latlngs[1];

            let reduced_lat = GeoUtils::reduced_accuracy(lat);
            let reduced_long = GeoUtils::reduced_accuracy(long);

            let set_acts_sharing_point = &mut self
                .unique_points
                .entry(telemetry.r#type.to_string())
                .or_default()
                .entry(reduced_lat)
                .or_insert(HashMap::from([(
                    reduced_long,
                    ActivityOccurence::default(),
                )]))
                .entry(reduced_long)
                .or_default()
                .set_activities;

            set_acts_sharing_point.insert(src_act_id);

            for dest_act_id in set_acts_sharing_point.iter().collect::<Vec<&DocumentId>>() {
                *self
                    .act_to_act_points
                    .entry(src_act_id)
                    .or_insert(HashMap::from([(*dest_act_id, 0)]))
                    .entry(*dest_act_id)
                    .or_default() += 1;
            }
        }

        return true;
    }

    // Output vector of routes which contain the matched activities or standalone ones
    pub fn matched_routes(&mut self) -> MatchedRoutesResult {
        //println!("Processed {} {}", self.acts_total, self.acts_unique.len());

        self.acts_total = 0;

        let results = self.generate_match_results();
        //println!("{:#?}", results);

        let merged_routes = self.merge_results(&results);
        //println!("{:#?}", merged_routes);

        merged_routes
            .iter()
            .map(|result| {
                self.route_idx += 1;

                Route {
                    _id: self.route_idx as f64,
                    activities: result.iter().map(|act_id| *act_id).collect(),

                    ..Default::default()
                }
            })
            .collect()
    }

    // PRIVATES
    fn generate_match_results(&self) -> Vec<Telem2TelemMatchResult> {
        let mut results: Vec<Telem2TelemMatchResult> = Vec::new();

        let compute_match_percent = |src, count| -> i32 {
            let data_len = self.activity_datapoints_count[src] as f32;
            let clamp_count = if count as f32 > data_len {
                data_len
            } else {
                count as f32
            };

            (100.0 * (clamp_count / data_len)) as i32
        };

        let distance_difference_percentage = |src, dest| -> i32 {
            let src_size = self.activity_lengths[src] as i32;
            let dest_size = self.activity_lengths[dest] as i32;

            let size_diff = (src_size - dest_size).abs() as f32;

            (100. * size_diff / (src_size.max(dest_size) as f32)) as i32
        };

        self.act_to_act_points.iter().for_each(|(src, dest_map)| {
            dest_map.iter().for_each(|(dest, count)| {
                let src_2_dest_match = compute_match_percent(src, *count);

                if src_2_dest_match >= MATCH_THRESHOLD
                    && distance_difference_percentage(src, dest) <= (100 - MATCH_THRESHOLD)
                {
                    results.push((src_2_dest_match, *src, *dest));
                }
            })
        });

        results.sort_by_key(|k| std::cmp::Reverse(100 * k.0 as i32));

        results
    }

    fn merge_results(&self, results: &Vec<Telem2TelemMatchResult>) -> MatchedTelemetriesResult {
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

                // Already created a group for source activity
                if src_act_id == dest_act_id {
                    continue;
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
                        act_to_group.remove(&dest_member_id);
                        act_to_group.insert(*dest_member_id, src_group_id);
                    }

                    groups.remove(&dest_group_id);
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

        println!("Allocated activities {}", count);
        println!(
            "  Missing activities {:?}",
            self.acts_unique.difference(&resulting_set)
        );

        merge_result
    }
}
