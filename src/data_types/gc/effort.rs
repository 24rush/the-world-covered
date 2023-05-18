use serde_derive::{Deserialize, Serialize};

use crate::data_types::{common::{Identifiable, DocumentId}, strava::athlete::AthleteId};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Effort {
    pub _id: f64,
    pub athlete_id: AthleteId,
    pub segment_id: DocumentId,
    pub activity_id: DocumentId,
    pub moving_time: i32,
    
    pub distance_from_start: i32
}

impl Identifiable for Effort {
    fn as_i64(&self) -> DocumentId {
        self._id as DocumentId
    }
}