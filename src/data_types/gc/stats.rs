use serde_derive::{Deserialize, Serialize};

use crate::data_types::common::DocumentId;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct YearlyStats {
    pub year: u32,
    pub rides_with_friends: u32,
    pub runs: u32,
    pub rides: u32,
    pub avg_power: f32,

    pub total_elevation_gain: u32,
    pub total_km: u32,

    pub hours_per_week: u32,

    //TODO distribution per days of week
    pub countries_visited: u32,
    pub states_visited: u32,

    // TODO pictures from rides
    pub total_kudos: u32,

    pub hardest_ride_id: DocumentId,
    pub longest_ride_id: DocumentId
    
}

pub struct OverallStats {
    pub total_km_ridden: u32,
    pub year_explored_most: u32,
    pub year_most_rides_with_friends: u32,
    pub year_hardest_rides: u32,

    pub days_with_most_rides: u8
}