use serde_derive::{Deserialize, Serialize};
use crate::data_types::{common::{Identifiable, DocumentId}, strava::athlete::AthleteId};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Gradient {
    pub start: usize,
    pub end: usize,
    pub gradient: f32
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Route {
    pub _id: f64,
    pub athlete_id: AthleteId,
    pub activities: Vec<DocumentId>,
    pub master_activity_id: DocumentId,
    pub polyline: String,
    pub segment_ids: Vec<DocumentId>,
    
    pub meters_climbed_per_km: f32,
    pub gradients: Vec<Gradient>    
}

impl Identifiable for Route {
    fn as_i64(&self) -> DocumentId {
        self._id as DocumentId
    }
}