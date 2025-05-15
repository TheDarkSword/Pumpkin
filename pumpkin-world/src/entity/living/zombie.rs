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

macro_rules! auto_struct {
    ($s:ident) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $s(pub ZombieCommon);
    };
}

macro_rules! auto_trait {
    ($s:ident) => {
        impl EntityPosition for $s {
            fn set_pos(&mut self, pos: Vector3<f64>) {
                self.0.mob_common.non_player_common.common.pos = pos;
            }
        }

        impl EntityBase for $s {
            fn uuid(&self) -> uuid::Uuid {
                self.0.mob_common.non_player_common.common.uuid()
            }

            fn id(&self) -> EntityId {
                self.0.mob_common.non_player_common.common.id()
            }

            fn pos(&self) -> Vector3<f64> {
                self.0.mob_common.non_player_common.common.pos
            }

            fn set_velocity(&mut self, velocity: Vector3<f64>) {
                self.0.mob_common.non_player_common.common.velocity = velocity;
            }

            fn get_velocity(&self) -> Vector3<f64> {
                self.0.mob_common.non_player_common.common.velocity
            }
        }
    };
}

macro_rules! auto_define {
    ($s:ident) => {
        auto_struct!($s);
        auto_trait!($s);
    };
}

auto_define!(Zombie);
auto_define!(Drowned);
