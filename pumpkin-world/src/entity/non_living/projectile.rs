use pumpkin_util::math::vector3::Vector3;
use serde::{Deserialize, Serialize};

use crate::entity::{EntityBase, EntityId, EntityPosition, NonPlayerCommon};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Projectile {
    #[serde(flatten)]
    pub common: NonPlayerCommon,
}

impl EntityPosition for Projectile {
    fn pos(&self) -> Vector3<f64> {
        self.common.common.pos
    }

    fn set_pos(&mut self, pos: Vector3<f64>) {
        self.common.common.pos = pos;
    }
}

impl EntityBase for Projectile {
    fn uuid(&self) -> uuid::Uuid {
        self.common.common.uuid()
    }

    fn id(&self) -> EntityId {
        self.common.common.id()
    }
}
