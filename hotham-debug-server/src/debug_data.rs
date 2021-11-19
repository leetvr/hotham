use serde::{Deserialize, Serialize};
use std::collections::HashMap;

type Entities = HashMap<u64, DebugEntity>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugData {
    pub id: i64,
    pub entities: Entities,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugEntity {
    pub name: String,
    pub id: i64,
    pub mesh: Option<String>,
    pub material: Option<String>,
    pub transform: Option<DebugTransform>,
    pub collider: Option<DebugCollider>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugTransform {
    pub translation: [i64; 3],
    pub rotation: [i64; 3],
    pub scale: [i64; 3],
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugCollider {
    pub collider_type: String,
    pub geometry: Vec<u64>,
}
