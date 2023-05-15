use serde_derive::Deserialize;

use super::common::Identifiable;

pub type ActivityId = i64;

#[derive(Debug, Deserialize)]
pub struct Segment {
    pub id: i64,
}

#[derive(Debug, Deserialize)]
pub struct SegmentEffort {
    pub segment: Segment
}

#[derive(Debug, Deserialize)]
pub struct Activity {
    pub _id: f64,
    pub segment_efforts: Vec<SegmentEffort>,
    pub r#type: String,
}

impl Identifiable for Activity {
    fn as_i64(&self) -> i64 {
        self._id as i64
    }
}