use std::{
    cell::Cell,
    collections::{HashMap, HashSet},
};

use crate::{
    data_types::{
        common::{DocumentId, Identifiable},
        gc::route::Route,
        strava::telemetry::Telemetry,
    },
    logvbln,
    util::geo::GeoUtils,
};

// PUBLIC Types
pub type MatchedRoutesResult = Vec<Route>;

// INTERNAL Type
type GroupId = DocumentId;
type MatchIndex = i32;

type LatLngReduced = i32;
type ActivityIdsForLatLng = HashMap<LatLngReduced, HashMap<LatLngReduced, HashSet<DocumentId>>>;

type MatchDistribution = HashMap<MatchIndex, (DocumentId, usize)>;

#[derive(Default)]
struct GroupInfo {
    // Actual activities that form the group
    activities: Cell<HashSet<DocumentId>>,
    // Number of times the match percentage occured in a group (so we know which master activity id to pick)
    match_distribution: Cell<MatchDistribution>,
}

// Raw result of match percentage between two activities
type Telem2TelemMatchResult = (MatchIndex, DocumentId, DocumentId);
// Raw merged result for all activities processed
type MatchedTelemetriesResult = Vec<HashSet<DocumentId>>;

// Percentage of GPS points that need to match so that we can consider two routes similar
const MATCH_THRESHOLD: MatchIndex = 85;

#[derive(Default)]
pub struct Commonality {
    // Number of GPS points per each activity loaded
    activity_datapoints_count: HashMap<DocumentId, usize>,

    // Length in meter for each activity loaded
    activity_lengths: HashMap<DocumentId, usize>,

    // All the telemetry points organized by type (cycling, hike, etc.)
    unique_points: HashMap<String, ActivityIdsForLatLng>,

    // Container storing number of matching points between two activies
    // Act A -> Act B (number)
    act_to_act_points: HashMap<DocumentId, HashMap<DocumentId, u32>>,

    // Counter for creating route indexes
    route_idx: DocumentId,

    // Statistics purposes
    points_total: usize,
    acts_total: u32,
    acts_unique: HashSet<DocumentId>,
}

impl<'a> Commonality {
    const CC: &str = "Commonality";

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

    pub fn load_telemetry(&mut self, telemetry: &Telemetry) -> bool {
        if telemetry.latlng.data.len() == 0 {
            return false;
        }

        let src_act_id = telemetry.as_i64();

        self.acts_total += 1;
        self.acts_unique.insert(src_act_id);
        self.points_total += telemetry.latlng.data.len();

        // Store the number of GPS points for this activity
        self.activity_datapoints_count
            .entry(src_act_id)
            .or_insert(telemetry.latlng.data.len());

        // Store the length of the activity
        self.activity_lengths
            .entry(src_act_id)
            .or_insert(telemetry.distance.data[telemetry.distance.data.len() - 1] as usize);

        for latlngs in &telemetry.latlng.data {
            let reduced_lat = GeoUtils::reduced_accuracy(latlngs[0]);
            let reduced_long = GeoUtils::reduced_accuracy(latlngs[1]);

            // Store the current GPS point in the map
            let set_acts_sharing_point = &mut self
                .unique_points
                .entry(telemetry.r#type.to_string())
                .or_default()
                .entry(reduced_lat)
                .or_insert(HashMap::from([(reduced_long, HashSet::new())]))
                .entry(reduced_long)
                .or_default();

            // Add the current activity id to the list of activities containing this point
            set_acts_sharing_point.insert(src_act_id);

            // Increment the number of shared points between current activity and the rest of activities that contain this point
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

    // Output is vector of routes which contain matched activities or standalone ones (that don't have any match)
    pub fn matched_routes(&mut self) -> Vec<Route> {
        logvbln!("Processed {} activities with {} points", self.acts_total, self.points_total);

        // Vector of (match percentage, act_id, act_id)
        let results = self.generate_match_results();
        logvbln!("{:#?}", results);

        // Vector of set of activities that are considered similar
        self.merge_results(&results)
    }

    // PRIVATES
    fn generate_match_results(&self) -> Vec<Telem2TelemMatchResult> {
        let distance_difference_percentage = |src, dest| -> i32 {
            let src_size = self.activity_lengths[src] as i32;
            let dest_size = self.activity_lengths[dest] as i32;

            let size_diff = (src_size - dest_size).abs() as f32;

            (100. * size_diff / (src_size.max(dest_size) as f32)) as i32
        };

        let mut results: Vec<Telem2TelemMatchResult> = Vec::new();

        self.act_to_act_points.iter().for_each(|(src, dest_map)| {
            dest_map.iter().for_each(|(dest, count)| {
                // Match percentage between activity src and dest
                let src_2_dest_match =
                    (100.0 * *count as f32 / self.activity_datapoints_count[src] as f32) as i32;

                // If percentage is higher than threshold and their lengths are also proportional then create result
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

    // Gets the complete list of match percentages between pairs of activities and generates groups by merging all of them
    fn merge_results(&mut self, results: &Vec<Telem2TelemMatchResult>) -> MatchedRoutesResult {
        let mut act_to_group: HashMap<DocumentId, GroupId> = HashMap::new(); // Existing activity to which group it is allocated
        let mut groups: HashMap<GroupId, GroupInfo> = HashMap::new(); // Group composition
        let mut group_id_counter: GroupId = 1;

        let shortest_act_id = |act_id_1: &DocumentId, act_id_2: &DocumentId| -> DocumentId {
            if *act_id_1 == 0 {
                return *act_id_2;
            }

            if *act_id_2 == 0 {
                return *act_id_1;
            }

            if self.activity_lengths[act_id_1] < self.activity_lengths[act_id_2] {
                *act_id_1
            } else {
                *act_id_2
            }
        };

        for two_telem_result in results {
            let match_percentage = two_telem_result.0;
            let src_act_id = two_telem_result.1;
            let dest_act_id = two_telem_result.2;

            let src_group_id = *(act_to_group.get(&src_act_id).unwrap_or(&0));
            let dest_group_id = *(act_to_group.get(&dest_act_id).unwrap_or(&0));

            let src_has_group = src_group_id != 0;
            let dest_has_group = dest_group_id != 0;

            {
                // GROUP creation - when both activities are not present yet
                let mut create_group = |activity_ids: &Vec<DocumentId>| {
                    let is_new_group = !groups.contains_key(&group_id_counter);

                    let group_members = groups.entry(group_id_counter).or_default();

                    activity_ids.iter().for_each(|act_id| {
                        act_to_group.insert(*act_id, group_id_counter);
                        group_members.activities.get_mut().insert(*act_id);

                        let distribution = group_members
                            .match_distribution
                            .get_mut()
                            .entry(match_percentage)
                            .or_default();

                        distribution.0 = shortest_act_id(&distribution.0, act_id);
                        distribution.1 += 1;
                    });

                    if is_new_group {
                        group_id_counter += 1;
                    }
                };

                if match_percentage < MATCH_THRESHOLD {
                    if !src_has_group {
                        create_group(&vec![src_act_id]);
                    }

                    // Already created a group for source activity above
                    if src_act_id != dest_act_id && !dest_has_group {
                        create_group(&vec![dest_act_id]);
                    }

                    continue;
                }

                // First occurence of these 2 activities
                if !src_has_group && !dest_has_group {
                    // None exist - create new group with just the 2 of them
                    create_group(&vec![src_act_id, dest_act_id]);

                    continue;
                }
            }

            let mut insert_in_group = |group_id, activity_ids: &Vec<DocumentId>| {
                if !groups.contains_key(&group_id) {
                    panic!("Groups does not contain any group with id {group_id}");
                }

                activity_ids.iter().for_each(|act_id| {
                    act_to_group.insert(*act_id, group_id);

                    let group_info = groups.entry(group_id).or_default();
                    group_info.activities.get_mut().insert(*act_id);

                    let mut distribution = group_info
                        .match_distribution
                        .get_mut()
                        .entry(match_percentage)
                        .or_default();

                    distribution.0 = shortest_act_id(&distribution.0, act_id);
                    distribution.1 += 1;
                });
            };

            if src_has_group && dest_has_group {
                // Both are in groups
                if src_group_id != dest_group_id {
                    // Move dest content to src group
                    for dest_member_id in
                        groups.get(&dest_group_id).unwrap().activities.take().iter()
                    {
                        groups
                            .get_mut(&src_group_id)
                            .unwrap()
                            .activities
                            .get_mut()
                            .insert(*dest_member_id);

                        act_to_group.remove(&dest_member_id);
                        act_to_group.insert(*dest_member_id, src_group_id);
                    }

                    // Add the match distribution for the current pair
                    let group_distr = groups
                        .get_mut(&src_group_id)
                        .unwrap()
                        .match_distribution
                        .get_mut()
                        .entry(match_percentage)
                        .or_default();

                    group_distr.0 = shortest_act_id(&group_distr.0, &src_act_id);
                    group_distr.1 += 1;

                    // Copy the match distribution from the dest group to src group
                    groups
                        .get_mut(&dest_group_id)
                        .unwrap()
                        .match_distribution
                        .take()
                        .iter()
                        .for_each(|(mp, (act_id, count))| {
                            let group_info = groups
                                .get_mut(&src_group_id)
                                .unwrap()
                                .match_distribution
                                .get_mut()
                                .entry(*mp)
                                .or_default();

                            group_info.0 = *act_id;
                            group_info.1 += count;
                        });

                    groups.remove(&dest_group_id);
                }
            } else if src_has_group {
                insert_in_group(src_group_id, &vec![dest_act_id]);
            } else if dest_has_group {
                insert_in_group(dest_group_id, &vec![src_act_id]);
            }
        }

        // DEBUG purposes
        let mut count = 0;
        let mut resulting_set: HashSet<DocumentId> = HashSet::new();

        let mut resulting_routes: Vec<Route> = Vec::new();

        for (_, group_info) in groups {
            let group = group_info.activities.take();

            if group.len() == 0 {
                continue;
            }

            for item in &group {
                resulting_set.insert(*item);
            }

            self.route_idx += 1;
            count += group.len();

            let mut max_percent = 0;
            let mut last_activity: DocumentId = 0;
            let mut master_act_id: DocumentId = 0;

            group_info
                .match_distribution
                .take()
                .iter()
                .for_each(|(match_perc, (act_id, _))| {
                    // Required for single activity groups which only contain themselves with 100%
                    last_activity = *act_id;

                    if *match_perc >= max_percent && *match_perc != 100 {
                        max_percent = *match_perc;
                        master_act_id = *act_id;
                    }
                });

            if master_act_id == 0 {
                master_act_id = last_activity;
            }

            resulting_routes.push(Route {
                _id: self.route_idx as f64,
                activities: group.into_iter().collect(),
                master_activity_id: master_act_id,

                ..Default::default()
            });
        }

        println!("Allocated activities {}", count);
        println!(
            "  Missing activities {:?}",
            self.acts_unique.difference(&resulting_set)
        );

        resulting_routes
    }
}
