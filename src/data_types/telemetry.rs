use serde_derive::Deserialize;

use super::common::Identifiable;

pub type TelemetryId = i64;

type LatLng = [f32; 2];

#[derive(Debug, Deserialize, Clone, Default)]
pub struct LatLngData {
    pub data: Vec<LatLng>
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct F32data {
    pub data: Vec<f32>
}

#[derive(Debug, Deserialize, Clone)]
pub struct Telemetry {
    pub _id: f64,
    pub r#type: String,

    #[serde(default)]   
    pub latlng: LatLngData,

    #[serde(default)] 
    pub velocity_smooth: F32data,

    #[serde(default)]  
    pub grade_smooth: F32data,
    
    #[serde(default)] 
    pub distance: F32data,

    #[serde(default)] 
    pub altitude: F32data,

    #[serde(default)] 
    pub time    : F32data,
}

impl Identifiable for Telemetry {
    fn as_i64(&self) -> i64 {
        self._id as i64
    }
}