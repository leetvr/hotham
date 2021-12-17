use approx::relative_eq;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugFrame {
    pub id: Uuid,
    pub frame_number: u64,
    pub session_id: Uuid,
    pub entities: Vec<DebugEntity>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugEntity {
    pub name: String,
    pub id: String,
    pub entity_id: u64,
    pub transform: Option<DebugTransform>,
    pub collider: Option<DebugCollider>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugTransform {
    pub translation: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}

impl PartialEq for DebugTransform {
    fn eq(&self, other: &Self) -> bool {
        relative_eq!(self.translation[0], other.translation[0])
            && relative_eq!(self.translation[1], other.translation[1])
            && relative_eq!(self.translation[2], other.translation[2])
            && relative_eq!(self.scale[0], other.scale[0])
            && relative_eq!(self.scale[1], other.scale[1])
            && relative_eq!(self.scale[2], other.scale[2])
            && relative_eq!(self.rotation[0], other.rotation[0])
            && relative_eq!(self.rotation[1], other.rotation[1])
            && relative_eq!(self.rotation[2], other.rotation[2])
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DebugCollider {
    pub collider_type: String,
    pub geometry: Vec<f32>,
    pub translation: [f32; 3],
}

// impl PartialEq for DebugCollider {
//     fn eq(&self, other: &Self) -> bool {
//         self.collider_type == other.collider_type && self.geometry == other.geometry
//     }
// }
