use serde_derive::{Deserialize, Serialize};
use crate::data_types::{common::{Identifiable, DocumentId}, strava::athlete::AthleteId};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Gradient {
    pub start: usize,
    pub end: usize,
    pub gradient: f32
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Route {
    pub _id: f64,
    pub master_activity_id: DocumentId,     
    pub r#type: String,

    pub athlete_id: AthleteId,
    
    pub activities: Vec<DocumentId>,

    pub distance: f32,
    pub average_speed: f32,
    pub total_elevation_gain: f32,

    pub description: Option<String>,
    pub location_city: Option<String>,
    pub location_country: String,
    pub polyline: String,    

    pub gradients: Vec<Gradient>    
}

impl Identifiable for Route {
    fn as_i64(&self) -> DocumentId {
        self._id as DocumentId
    }
}