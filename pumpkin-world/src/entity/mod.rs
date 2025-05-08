use std::{
    collections::HashMap,
    sync::atomic::{AtomicI32, Ordering},
};

use enum_dispatch::enum_dispatch;
use living::zombie::ZombieCommon;
use non_living::{exp_orb::ExpOrb, item::Item, projectile::Projectile, tnt::Tnt};
use pumpkin_data::entity::{EffectType, EntityType};
use pumpkin_nbt::{compound::NbtCompound, nbt_int_array};
use pumpkin_util::{
    math::{
        position::BlockPos,
        vector2::Vector2,
        vector3::{Math, Vector3},
    },
    text::TextComponent,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::DeserializeOwned};

use crate::generation::positions::chunk_pos;

pub mod entity_data_flags;
pub mod living;
pub mod non_living;

// Helper methods for converting values to their nbt forms
mod nbt_vector {
    use pumpkin_util::math::vector3::{Math, Vector3};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<T, S>(v: &Vector3<T>, s: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize + Copy,
        S: Serializer,
    {
        let arr = [v.x, v.y, v.z];
        arr.serialize(s)
    }

    pub fn deserialize<'de, T, D>(d: D) -> Result<Vector3<T>, D::Error>
    where
        T: Deserialize<'de> + Math + Copy,
        D: Deserializer<'de>,
    {
        let value: [T; 3] = Deserialize::deserialize(d)?;
        Ok(Vector3::new(value[0], value[1], value[2]))
    }
}

mod nbt_entity_type {
    use pumpkin_data::entity::EntityType;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

    pub fn serialize<S>(v: &EntityType, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        format!("minecraft:{}", v.resource_name).serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<EntityType, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: &str = Deserialize::deserialize(d)?;
        let id = value
            .strip_prefix("minecraft:")
            .ok_or_else(|| de::Error::custom("Invalid minecraft entity resource id"))?;
        EntityType::from_name(id)
            .ok_or_else(|| de::Error::custom(format!("Unknown minecraft entity resource id {id}")))
    }
}

mod nbt_text_component {
    use pumpkin_util::text::TextComponent;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de, ser};

    pub fn serialize<S>(v: &Option<TextComponent>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let Some(v) = v else {
            return Err(ser::Error::custom("Expected some value if serializing"));
        };

        let stringified = serde_json::to_string(v).map_err(|err| {
            ser::Error::custom(format!(
                "Failed to serialize text component as json: {:?}",
                err
            ))
        })?;
        stringified.serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<TextComponent>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let stringified: &str = Deserialize::deserialize(d)?;
        serde_json::from_str(stringified)
            .map_err(|err| {
                de::Error::custom(format!(
                    "Failed to deserialized text component json: {:?}",
                    err
                ))
            })
            .map(Some)
    }
}

mod nbt_rotation {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::Rotation;

    pub fn serialize<S>(v: &Rotation, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let arr = [v.yaw, v.pitch];
        arr.serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Rotation, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: [f32; 2] = Deserialize::deserialize(d)?;
        Ok(Rotation {
            yaw: value[0],
            pitch: value[1],
        })
    }
}

mod nbt_uuid {
    use pumpkin_nbt::nbt_int_array;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(v: &uuid::Uuid, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (hi, lo) = v.as_u64_pair();
        let hi_hi = (hi >> 32) as i32;
        let hi_lo = hi as i32;
        let lo_hi = (lo >> 32) as i32;
        let lo_lo = lo as i32;

        nbt_int_array([hi_hi, hi_lo, lo_hi, lo_lo], s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<uuid::Uuid, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value: [i32; 4] = Deserialize::deserialize(d)?;
        let hi = ((value[0] as u64) << 32) | value[1] as u64;
        let lo = ((value[2] as u64) << 32) | value[3] as u64;

        Ok(uuid::Uuid::from_u64_pair(hi, lo))
    }
}

pub enum Entity {
    Living(LivingEntity),
    NonLiving(NonLivingEntity),
}

impl Entity {
    // TODO: We can probably use serde for this instead of parsing nbt compounds, but that is
    // somewhat difficult due to data being slightly different that what is serialized in NBTs such
    // as UUIDs. This is all doable in serde, but I think that should be a task for another time

    fn from_nbt(nbt: &NbtCompound) -> Option<Self> {
        let id = nbt.get_string("id")?;
        let kind = EntityType::from_name(&id.replace("minecraft:", ""))?;
        match kind {
            EntityType::ZOMBIE | EntityType::DROWNED => Some(Entity::Living(
                LivingEntity::ZombieLike(ZombieCommon::from_nbt(nbt)?),
            )),
            EntityType::EXPERIENCE_ORB => Some(Entity::NonLiving(NonLivingEntity::ExpOrb(
                ExpOrb::from_nbt(nbt)?,
            ))),
            _ => None,
        }
    }
}

pub enum LivingEntity {
    ZombieLike(ZombieCommon),
}

pub enum NonLivingEntity {
    Item(Item),
    Projectile(Projectile),
    Tnt(Tnt),
    ExpOrb(ExpOrb),
}

pub type EntityId = i32;
static CURRENT_ID: AtomicI32 = AtomicI32::new(0);

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub struct EffectData {
    pub duration: i32,
    pub amplifier: u8,
    pub ambient: bool,
    pub show_particles: bool,
    pub show_icon: bool,
    pub blend: bool,
    // TODO: Hidden effect
}

pub struct MobCommon {
    pub living_common: LivingCommon,
    pub non_player_common: NonPlayerCommon,
}

impl MobCommon {
    fn from_nbt(nbt: &NbtCompound) -> Option<Self> {
        let living_common = LivingCommon::from_nbt(nbt)?;
        let non_player_common = NonPlayerCommon::from_nbt(nbt)?;

        Some(Self {
            living_common,
            non_player_common,
        })
    }
}

pub struct LivingCommon {
    pub absorption: f32,
    pub active_effects: HashMap<EffectType, EffectData>,
    // TODO: Attributes
    pub can_pick_up_items: bool,
    pub death_ticks: i16,
    // TODO: Drop chances
    // TODO: Equipment
    pub fall_flying: bool,
    pub health: f32,
    // TODO: Others
}

impl LivingCommon {
    fn from_nbt(nbt: &NbtCompound) -> Option<Self> {
        let absorption = nbt.get_float("AbsorptionAmount")?;
        let mut active_effects = HashMap::new();

        if let Some(effects) = nbt.get_list("active_effects") {
            for effect_nbt in effects {
                if let Some(effect_nbt) = effect_nbt.extract_compound() {
                    let ambient = effect_nbt.get_bool("ambient")?;
                    let amplifier = effect_nbt.get_byte("amplifier")? as u8;
                    let duration = effect_nbt.get_int("duration")?;
                    let show_icon = effect_nbt.get_bool("show_icon")?;
                    let show_particles = effect_nbt.get_bool("show_particles")?;

                    let id = effect_nbt.get_string("id")?;
                    let kind = EffectType::from_name(&id.replace("minecraft:", ""))?;

                    active_effects.insert(
                        kind,
                        EffectData {
                            duration,
                            amplifier,
                            ambient,
                            show_particles,
                            show_icon,
                            // TODO: What is this?
                            blend: true,
                        },
                    );
                }
            }
        }

        let can_pick_up_items = nbt.get_bool("CanPickUpLoot")?;
        let death_ticks = nbt.get_short("DeathTime")?;
        let fall_flying = nbt.get_bool("FallFlying")?;
        let health = nbt.get_float("Health")?;

        Some(Self {
            absorption,
            active_effects,
            can_pick_up_items,
            death_ticks,
            fall_flying,
            health,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub struct NonPlayerCommon {
    #[serde(rename = "id", with = "nbt_entity_type")]
    kind: EntityType,
    #[serde(flatten)]
    pub common: EntityCommon,
    #[serde(
        rename = "CustomName",
        with = "nbt_text_component",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub custom_name: Option<TextComponent>,
    #[serde(rename = "CustomNameVisible", default)]
    pub custom_name_visible: Option<bool>,
}

impl NonPlayerCommon {
    pub fn kind(&self) -> EntityType {
        self.kind
    }
}

fn assign_entity_id() -> EntityId {
    CURRENT_ID.fetch_add(1, Ordering::Relaxed)
}

pub struct Rotation {
    yaw: f32,
    pitch: f32,
}

#[derive(Serialize, Deserialize)]
pub struct EntityCommon {
    #[serde(skip, default = "assign_entity_id")]
    id: EntityId,
    #[serde(rename = "UUID", with = "nbt_uuid")]
    uuid: uuid::Uuid,
    #[serde(rename = "Air")]
    pub air_ticks: i16,
    pub fall_distance: f64,
    #[serde(rename = "Fire")]
    pub fire_ticks: i16,
    #[serde(rename = "Glowing")]
    pub glowing: bool,
    #[serde(rename = "HasVisualFire")]
    pub visual_fire: bool,
    #[serde(rename = "Invulnerable")]
    pub invulnerable: bool,
    #[serde(rename = "Motion", with = "nbt_vector")]
    pub velocity: Vector3<f64>,
    #[serde(rename = "NoGravity")]
    pub no_gravity: bool,
    #[serde(rename = "OnGround")]
    pub on_ground: bool,
    //TODO: passengers
    #[serde(rename = "PortalCooldown")]
    pub portal_cooldown_ticks: i32,
    #[serde(rename = "Pos", with = "nbt_vector")]
    pub pos: Vector3<f64>,
    #[serde(rename = "Rotation", with = "nbt_rotation")]
    pub rotation: Rotation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub silent: Option<bool>,
    //TODO: scoreboard tags
}

impl EntityCommon {
    pub fn id(&self) -> EntityId {
        self.id
    }

    pub fn uuid(&self) -> uuid::Uuid {
        self.uuid
    }
}
