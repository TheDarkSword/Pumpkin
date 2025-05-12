use pumpkin_nbt::deserializer::deserialize_nbt_bool;
use pumpkin_util::math::vector3::Vector3;
use serde::{Deserialize, Serialize};

use crate::entity::{EntityBase, EntityId, EntityPosition, MobCommon};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZombieCommon {
    #[serde(flatten)]
    pub mob_common: MobCommon,
    #[serde(rename = "IsBaby", default, deserialize_with = "deserialize_nbt_bool")]
    pub is_baby: bool,
    // TODO: Others
}

impl EntityPosition for ZombieCommon {
    fn pos(&self) -> Vector3<f64> {
        self.mob_common.non_player_common.common.pos
    }

    fn set_pos(&mut self, pos: Vector3<f64>) {
        self.mob_common.non_player_common.common.pos = pos;
    }
}

impl EntityBase for ZombieCommon {
    fn uuid(&self) -> uuid::Uuid {
        self.mob_common.non_player_common.common.uuid()
    }

    fn id(&self) -> EntityId {
        self.mob_common.non_player_common.common.id()
    }
}
