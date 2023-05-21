use serde_derive::{Deserialize, Serialize};

use crate::data_types::common::DocumentId;

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Map {
    pub polyline: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ResourceId {
    pub id: DocumentId,
}
