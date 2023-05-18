use std::{collections::HashMap};
use serde_derive::{Deserialize, Serialize};

pub type AthleteId = i64;

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct SegmentVisitedData {
    pub count: i32
}

#[derive(Deserialize, Debug, Serialize, Clone, Default)]
pub struct AthleteData {    
    pub _id: AthleteId,
    pub tokens: AthleteTokens,
    pub before_ts: i64,
    pub after_ts: i64,

    #[serde(default)]    
    pub segments: HashMap<String, SegmentVisitedData>
}

#[derive(Deserialize, Debug, Serialize, Clone, Default)]
pub struct AthleteTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

impl AthleteData {
    pub fn incr_visited_segment(&mut self, seg_id: i64) {
        if let Some(ref mut existing_seg) = self.segments.get_mut(&seg_id.to_string()) {
            existing_seg.count += 1;
        } else {
            self.segments.insert(seg_id.to_string(), SegmentVisitedData { count: 1 });
        }
    }
}

impl crate::data_types::common::Identifiable for AthleteData {
    fn as_i64(&self) -> i64 {
        self._id
    }
}