use serde_derive::{Deserialize, Serialize};

use crate::data_types::common::DocumentId;

use super::common::{Map, ResourceId};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Effort {
    pub id: DocumentId,
    pub athlete: ResourceId,
    pub activity: ResourceId,
    pub segment: ResourceId,

    pub moving_time: i32,
    pub start_index: i32,
    pub end_index: i32,    
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Activity {
    pub _id: f64,
    pub distance: f32,
    pub segment_efforts: Vec<Effort>,
    pub r#type: String,

    pub map: Map,
    pub total_elevation_gain: f32,
}

impl crate::data_types::common::Identifiable for Activity {
    fn as_i64(&self) -> i64 {
        self._id as i64
    }
}