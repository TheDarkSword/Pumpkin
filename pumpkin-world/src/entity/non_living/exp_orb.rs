use pumpkin_util::math::vector3::Vector3;
use serde::{Deserialize, Serialize};

use crate::entity::{EntityBase, EntityId, EntityPosition, NonPlayerCommon};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpOrb {
    #[serde(flatten)]
    pub common: NonPlayerCommon,
    #[serde(rename = "Age")]
    pub age_ticks: i16,
    #[serde(rename = "Count")]
    pub merge_count: i32,
    #[serde(rename = "Health")]
    pub health: i16,
    #[serde(rename = "Value")]
    pub value: i16,
}

impl EntityPosition for ExpOrb {
    fn pos(&self) -> Vector3<f64> {
        self.common.common.pos
    }

    fn set_pos(&mut self, pos: Vector3<f64>) {
        self.common.common.pos = pos;
    }
}

impl EntityBase for ExpOrb {
    fn uuid(&self) -> uuid::Uuid {
        self.common.common.uuid()
    }

    fn id(&self) -> EntityId {
        self.common.common.id()
    }
}
