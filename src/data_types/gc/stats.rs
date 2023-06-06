use serde_derive::{Deserialize, Serialize};

use crate::data_types::common::DocumentId;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct YearlyStats {
    pub year: u32,
    pub rides_with_friends: u32,
    pub runs: u32,
    pub rides: u32,
    pub total_elevation_gain: u32,
    pub total_km_rides: u32,
    pub total_km_runs: u32,

    pub hours_per_week_rides: u32,
    pub hours_per_week_runs: u32,

    pub total_kudos: u32,
    pub most_kudos_activity: DocumentId
    /*TODO distribution per days of week
    pub countries_visited: u32,
    pub states_visited: u32,    
    pub hardest_ride_id: DocumentId,
    pub longest_ride_id: DocumentId    */
}

pub struct OverallStats {
    pub total_km_ridden: u32,
    pub year_explored_most: u32,
    pub year_most_rides_with_friends: u32,
    pub year_hardest_rides: u32,

    pub days_with_most_rides: u8
}