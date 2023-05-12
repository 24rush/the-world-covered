use serde_derive::Deserialize;

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
    pub _id: i64,
    pub segment_efforts: Vec<SegmentEffort>,
}