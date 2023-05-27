use serde::Deserialize;

use crate::data_types::common::DocumentId;

use super::common::Map;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Xoms {
    #[serde(deserialize_with = "null_to_default")]
    pub kom: String,
    #[serde(deserialize_with = "null_to_default")]
    pub qom: String
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Segment {
    pub _id: DocumentId,
    pub distance: f32,
    pub name: String,
    pub xoms: Xoms,
    pub map: Map,
}

fn null_to_default<'de, D, T>(de: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    let key = Option::<T>::deserialize(de)?;
    Ok(key.unwrap_or_default())
}