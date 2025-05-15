use pumpkin_util::math::vector3::Vector3;
use serde::{Deserialize, Serialize};

use crate::entity::{EntityBase, EntityId, EntityPosition, NonPlayerCommon};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectileCommon {
    #[serde(flatten)]
    pub common: NonPlayerCommon,
}

macro_rules! auto_struct {
    ($s:ident) => {
        #[derive(Debug, Clone, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $s(pub ProjectileCommon);
    };
}

macro_rules! auto_trait {
    ($s:ident) => {
        impl EntityPosition for $s {
            fn set_pos(&mut self, pos: Vector3<f64>) {
                self.0.common.common.pos = pos;
            }
        }

        impl EntityBase for $s {
            fn uuid(&self) -> uuid::Uuid {
                self.0.common.common.uuid()
            }

            fn id(&self) -> EntityId {
                self.0.common.common.id()
            }

            fn pos(&self) -> Vector3<f64> {
                self.0.common.common.pos
            }

            fn set_velocity(&mut self, velocity: Vector3<f64>) {
                self.0.common.common.velocity = velocity;
            }

            fn get_velocity(&self) -> Vector3<f64> {
                self.0.common.common.velocity
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

auto_define!(Arrow);
auto_define!(BreezeWindCharge);
auto_define!(DragonFireball);
auto_define!(Egg);
auto_define!(EnderPearl);
auto_define!(ExpBottle);
auto_define!(EnderEye);
auto_define!(Fireball);
auto_define!(Firework);
auto_define!(LlamaSpit);
auto_define!(Potion);
auto_define!(ShulkerBullet);
auto_define!(SmallFireball);
auto_define!(Snowball);
auto_define!(SpectralArrow);
auto_define!(Trident);
auto_define!(WindCharge);
auto_define!(WitherSkull);
