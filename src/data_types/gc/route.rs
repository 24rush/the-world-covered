use serde_derive::{Deserialize, Serialize};

use crate::data_types::{common::{Identifiable, DocumentId}, strava::athlete::AthleteId};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Route {
    pub _id: f64,
    pub athlete_id: AthleteId,
    pub activities: Vec<DocumentId>,
    pub master_activity_id: DocumentId,
    pub polyline: String,
    pub segment_ids: Vec<DocumentId>,
    
    pub climb_per_km: f32
}

impl Identifiable for Route {
    fn as_i64(&self) -> DocumentId {
        self._id as DocumentId
    }
}