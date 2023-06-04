use serde_derive::{Deserialize, Serialize};
use crate::data_types::{common::{Identifiable, DocumentId}, strava::athlete::AthleteId};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Gradient {
    pub start_index: usize,
    pub end_index: usize,
    pub length: f32,
    pub avg_gradient: f32,
    pub max_gradient: f32,
    pub elevation_gain: f32,
    pub location_city: Option<String>,
    pub location_country: Option<String>,
    pub gradient: f32,
    pub altitude: Vec<i16>,
    pub distance: Vec<i16>
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

    pub gradients: Vec<Gradient>,
    pub dist_from_capital: i32,
    pub center_coord: geo_types::Coord,
}

impl Identifiable for Route {
    fn as_i64(&self) -> DocumentId {
        self._id as DocumentId
    }
}