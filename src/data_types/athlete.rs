use std::{collections::HashMap};

use chrono::Utc;
use serde_derive::{Deserialize, Serialize};

use super::common::Identifiable;

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct SegmentVisitedData {
    pub count: i32
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct AthleteData {    
    pub _id: i64,
    pub tokens: AthleteTokens,
    pub before_ts: i64,
    pub after_ts: i64,

    #[serde(default)]    
    pub segments: HashMap<String, SegmentVisitedData>
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct AthleteTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

impl AthleteData {
    pub fn new(id: i64) -> Self {
        Self {   
            _id: id,         
            before_ts: Utc::now().timestamp(),
            after_ts: 0,
            tokens: AthleteTokens {
                access_token: "168f77da961a3d36941a6f63a52570f6c007ebf6".to_string(),
                refresh_token: "1448cdba35873b1db89f6001afae1eeff6dd972a".to_string(),
                expires_at: 0,                
            },
            segments: HashMap::new()
        }
    }

    pub fn incr_visited_segment(&mut self, seg_id: i64) {
        if let Some(ref mut existing_seg) = self.segments.get_mut(&seg_id.to_string()) {
            existing_seg.count += 1;
        } else {
            self.segments.insert(seg_id.to_string(), SegmentVisitedData { count: 1 });
        }
    }
}

impl Identifiable for AthleteData {
    fn as_i64(&self) -> i64 {
        self._id
    }
}