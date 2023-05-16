use serde_derive::Deserialize;

use crate::data_types::common::DocumentId;

use super::common::Map;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Xoms {
    pub kom: String,
    pub qom: String
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Segment {
    pub _id: DocumentId,
    pub name: String,
    pub xoms: Xoms,
    pub map: Map,
}
