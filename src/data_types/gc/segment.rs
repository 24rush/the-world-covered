use serde_derive::{Deserialize, Serialize};

use crate::data_types::{common::{Identifiable, DocumentId}};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Segment {
    pub _id: f64,
    pub start_index: i32,
    pub end_index: i32,

    pub polyline: String,
    pub kom: String,
    pub qom: String,
    pub time_to_xom: i32, // Distance to best effort
}

impl Identifiable for Segment {
    fn as_i64(&self) -> DocumentId {
        self._id as DocumentId
    }
}