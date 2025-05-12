use std::{
    collections::HashMap,
    sync::atomic::{AtomicI32, Ordering},
};

use enum_dispatch::enum_dispatch;
use living::zombie::ZombieCommon;
use non_living::{exp_orb::ExpOrb, item::Item, projectile::Projectile, tnt::Tnt};
use pumpkin_data::entity::{EffectType, EntityType};
use pumpkin_nbt::deserializer::deserialize_nbt_bool;
use pumpkin_util::{math::vector3::Vector3, text::TextComponent};
use serde::{Deserialize, Serialize};

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
        let value: String = Deserialize::deserialize(d)?;
        let id = value
            .strip_prefix("minecraft:")
            .ok_or_else(|| de::Error::custom(format!("Invalid entity resource id {value}")))?;
        EntityType::from_name(id)
            .ok_or_else(|| de::Error::custom(format!("Unknown entity resource id {id}")))
    }
}

mod nbt_item_stack {
    use pumpkin_data::item::Item;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

    use crate::item::ItemStack;

    fn default_count() -> i32 {
        1
    }

    #[derive(Serialize, Deserialize)]
    struct NbtRepr {
        id: String,
        #[serde(default = "default_count")]
        count: i32,
        // TODO: components
    }

    pub fn serialize<S>(v: &ItemStack, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        NbtRepr {
            id: format!("minecraft:{}", v.item.registry_key),
            count: v.item_count as i32,
        }
        .serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<ItemStack, D::Error>
    where
        D: Deserializer<'de>,
    {
        let data: NbtRepr = Deserialize::deserialize(d)?;
        let id = data
            .id
            .strip_prefix("minecraft:")
            .ok_or_else(|| de::Error::custom(format!("Invalid item resource id {}", data.id)))?;
        let item = Item::from_registry_key(id)
            .ok_or_else(|| de::Error::custom(format!("Unknown item resource id {}", data.id)))?;

        Ok(ItemStack::new(data.count as u8, item))
    }
}

mod nbt_effects {
    use std::collections::HashMap;

    use pumpkin_data::entity::EffectType;
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de};

    use super::EffectData;

    #[derive(Serialize, Deserialize)]
    struct NbtRepr {
        id: String,
        #[serde(flatten)]
        data: EffectData,
    }

    pub fn serialize<S>(v: &HashMap<EffectType, EffectData>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let effects: Vec<_> = v
            .iter()
            .map(|(kind, data)| NbtRepr {
                id: format!("minecraft:{}", kind.to_name()),
                data: data.clone(),
            })
            .collect();

        effects.serialize(s)
    }

    pub fn deserialize<'de, D>(d: D) -> Result<HashMap<EffectType, EffectData>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let effects: Vec<NbtRepr> = Deserialize::deserialize(d)?;
        let mut mappings = Vec::with_capacity(effects.len());

        for effect in effects {
            let id = effect.id.strip_prefix("minecraft:").ok_or_else(|| {
                de::Error::custom(format!("Invalid effect resource key: {}", effect.id))
            })?;
            let kind = EffectType::from_name(id).ok_or_else(|| {
                de::Error::custom(format!("Unknown effect resource key: {}", effect.id))
            })?;

            mappings.push((kind, effect.data));
        }

        Ok(HashMap::from_iter(mappings))
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

/// This should only be used in level due to the need to control how positions are modified
#[enum_dispatch]
pub(crate) trait EntityPosition {
    fn pos(&self) -> Vector3<f64>;

    fn set_pos(&mut self, pos: Vector3<f64>);
}

#[enum_dispatch]
pub trait EntityBase {
    /// Unique identifier for this entity. Persistent.
    fn uuid(&self) -> uuid::Uuid;

    /// Unique identifier for this entity. Transient.
    fn id(&self) -> EntityId;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[enum_dispatch(EntityPosition, EntityBase)]
pub enum Entity {
    Living(LivingEntity),
    NonLiving(NonLivingEntity),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "id")]
#[enum_dispatch(EntityPosition, EntityBase)]
pub enum LivingEntity {
    #[serde(untagged, alias = "minecraft:zombie", alias = "minecraft:drowned")]
    ZombieLike(ZombieCommon),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "id")]
#[enum_dispatch(EntityPosition, EntityBase)]
pub enum NonLivingEntity {
    #[serde(untagged, alias = "minecraft:item")]
    Item(Item),
    #[serde(
        untagged,
        alias = "minecraft:arrow",
        alias = "minecraft:breeze_wind_charge",
        alias = "minecraft:dragon_fireball",
        alias = "minecraft:egg",
        alias = "minecraft:ender_pearl",
        alias = "minecraft:experience_bottle",
        alias = "minecraft:eye_of_ender",
        alias = "minecraft:fireball",
        alias = "minecraft:firework_rocket",
        alias = "minecraft:llama_spit",
        alias = "minecraft:potion",
        alias = "minecraft:shulker_bullet",
        alias = "minecraft:small_fireball",
        alias = "minecraft:snowball",
        alias = "minecraft:spectral_arrow",
        alias = "minecraft:trident",
        alias = "minecraft:wind_charge",
        alias = "minecraft:wither_skull"
    )]
    Projectile(Projectile),
    #[serde(untagged, alias = "minecraft:tnt")]
    Tnt(Tnt),
    #[serde(untagged, alias = "minecraft:experience_orb")]
    ExpOrb(ExpOrb),
}

pub type EntityId = i32;
static CURRENT_ID: AtomicI32 = AtomicI32::new(0);

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct EffectData {
    pub duration: i32,
    pub amplifier: i8,
    #[serde(deserialize_with = "deserialize_nbt_bool")]
    pub ambient: bool,
    #[serde(deserialize_with = "deserialize_nbt_bool")]
    pub show_particles: bool,
    #[serde(deserialize_with = "deserialize_nbt_bool")]
    pub show_icon: bool,
    #[serde(default, deserialize_with = "deserialize_nbt_bool")]
    pub blend: bool,
    // TODO: Hidden effect
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobCommon {
    #[serde(flatten)]
    pub living_common: LivingCommon,
    #[serde(flatten)]
    pub non_player_common: NonPlayerCommon,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LivingCommon {
    #[serde(rename = "AbsorptionAmount")]
    pub absorption: f32,
    #[serde(
        with = "nbt_effects",
        skip_serializing_if = "HashMap::is_empty",
        default
    )]
    pub active_effects: HashMap<EffectType, EffectData>,
    // TODO: Attributes
    #[serde(rename = "CanPickUpLoot", deserialize_with = "deserialize_nbt_bool")]
    pub can_pick_up_items: bool,
    #[serde(rename = "DeathTime")]
    pub death_ticks: i16,
    // TODO: Drop chances
    // TODO: Equipment
    #[serde(rename = "FallFlying", deserialize_with = "deserialize_nbt_bool")]
    pub fall_flying: bool,
    #[serde(rename = "Health")]
    pub health: f32,
    // TODO: Others
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(
        rename = "CustomNameVisible",
        default,
        deserialize_with = "deserialize_nbt_bool"
    )]
    pub custom_name_visible: bool,
}

impl NonPlayerCommon {
    pub fn kind(&self) -> EntityType {
        self.kind
    }
}

fn assign_entity_id() -> EntityId {
    CURRENT_ID.fetch_add(1, Ordering::Relaxed)
}

#[derive(Debug, Clone)]
pub struct Rotation {
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    #[serde(rename = "Glowing", default, deserialize_with = "deserialize_nbt_bool")]
    pub glowing: bool,
    #[serde(
        rename = "HasVisualFire",
        default,
        deserialize_with = "deserialize_nbt_bool"
    )]
    pub visual_fire: bool,
    #[serde(rename = "Invulnerable", deserialize_with = "deserialize_nbt_bool")]
    pub invulnerable: bool,
    #[serde(rename = "Motion", with = "nbt_vector")]
    pub velocity: Vector3<f64>,
    #[serde(
        rename = "NoGravity",
        default,
        deserialize_with = "deserialize_nbt_bool"
    )]
    pub no_gravity: bool,
    #[serde(rename = "OnGround", deserialize_with = "deserialize_nbt_bool")]
    pub on_ground: bool,
    //TODO: passengers
    #[serde(rename = "PortalCooldown")]
    pub portal_cooldown_ticks: i32,
    #[serde(rename = "Pos", with = "nbt_vector")]
    pos: Vector3<f64>,
    #[serde(rename = "Rotation", with = "nbt_rotation")]
    pub rotation: Rotation,
    #[serde(default, deserialize_with = "deserialize_nbt_bool")]
    pub silent: bool,
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
