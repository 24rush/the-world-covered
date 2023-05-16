use serde_derive::Deserialize;

use crate::data_types::common::DocumentId;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Map {
    pub polyline: String,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct ResourceId {
    pub id: DocumentId,
}