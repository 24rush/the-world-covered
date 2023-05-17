use serde_derive::Deserialize;

use crate::data_types::common::DocumentId;

use super::common::{Map, ResourceId};

#[derive(Debug, Deserialize, Clone)]
pub struct Effort {
    pub id: DocumentId,
    pub athlete: ResourceId,
    pub activity: ResourceId,
    pub segment: ResourceId,

    pub moving_time: i32,
    pub start_index: i32,
    pub end_index: i32
}

#[derive(Debug, Deserialize, Clone)]
pub struct Activity {
    pub _id: f64,
    pub distance: f32,
    pub segment_efforts: Vec<Effort>,
    pub r#type: String,

    pub map: Map,
}

impl crate::data_types::common::Identifiable for Activity {
    fn as_i64(&self) -> i64 {
        self._id as i64
    }
}