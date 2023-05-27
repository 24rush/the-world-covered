use serde_derive::{Deserialize, Serialize};

use crate::data_types::common::DocumentId;

use super::common::{Map, ResourceId};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Segment {
    pub id: DocumentId,
    pub average_grade: f32,
    pub distance: f32,
    pub city: Option<String>,
    pub country: Option<String>
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Effort {
    pub id: DocumentId,
    pub athlete: ResourceId,
    pub activity: ResourceId,
    pub name: String,
    pub segment: Segment,

    pub moving_time: i32,
    pub start_index: i32,
    pub end_index: i32,
    pub start_date_local: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Activity {
    pub _id: f64,
    pub distance: f32,
    pub average_speed: f32,
    pub segment_efforts: Vec<Effort>,
    pub r#type: String,

    pub map: Map,
    pub elapsed_time: i32,
    pub total_elevation_gain: f32,
    
    pub athlete_count: u8,
    pub description: Option<String>,
    pub location_city: Option<String>,
    pub location_country: String,
    pub start_date_local: String,
}

impl crate::data_types::common::Identifiable for Activity {
    fn as_i64(&self) -> i64 {
        self._id as i64
    }
}