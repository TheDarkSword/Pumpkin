use pumpkin_util::math::vector3::Vector3;
use serde::{Deserialize, Serialize};

use crate::{
    entity::{EntityBase, EntityId, EntityPosition, NonPlayerCommon, nbt_item_stack},
    item::ItemStack,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    #[serde(flatten)]
    pub common: NonPlayerCommon,
    #[serde(rename = "Age")]
    pub age_ticks: i16,
    #[serde(rename = "PickupDelay")]
    pub pickup_delay_ticks: i16,
    #[serde(rename = "Health")]
    pub health: i16,
    #[serde(rename = "Item", with = "nbt_item_stack")]
    pub stack: ItemStack,
    // TODO: Owners
}

impl EntityPosition for Item {
    fn set_pos(&mut self, pos: Vector3<f64>) {
        self.common.common.pos = pos;
    }
}

impl EntityBase for Item {
    fn uuid(&self) -> uuid::Uuid {
        self.common.common.uuid()
    }

    fn id(&self) -> EntityId {
        self.common.common.id()
    }

    fn pos(&self) -> Vector3<f64> {
        self.common.common.pos
    }

    fn set_velocity(&mut self, velocity: Vector3<f64>) {
        self.common.common.velocity = velocity;
    }

    fn get_velocity(&self) -> Vector3<f64> {
        self.common.common.velocity
    }
}
