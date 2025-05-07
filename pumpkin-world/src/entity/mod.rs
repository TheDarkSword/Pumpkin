use std::sync::atomic::{AtomicI32, Ordering};

use pumpkin_data::entity::EntityType;
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::{
    math::{position::BlockPos, vector2::Vector2, vector3::Vector3},
    text::TextComponent,
};

use crate::generation::positions::chunk_pos;

pub mod entity_data_flags;

pub enum Entity {
    Living(LivingEntity),
    NonLiving(NonLivingEntity),
}

pub enum LivingEntity {}

pub enum NonLivingEntity {}

pub type EntityId = i32;
static CURRENT_ID: AtomicI32 = AtomicI32::new(0);

struct MobCommon {
    // TODO
}

struct NonPlayerEntityCommon {
    kind: EntityType,
    common: EntityCommon,
    custom_name: Option<TextComponent>,
    custom_name_visible: Option<bool>,
}

impl NonPlayerEntityCommon {
    fn from_nbt(nbt: &NbtCompound) -> Option<Self> {
        let id = nbt.get_string("id")?;
        let kind = EntityType::from_name(&id.replace("minecraft:", ""))?;
        let custom_name: Option<TextComponent> = {
            let raw_json = nbt.get_string("CustomName")?;
            serde_json::from_str(raw_json).ok()
        };
        let custom_name_visible = nbt.get_bool("CustomNameVisible");

        let common = EntityCommon::from_nbt(nbt)?;

        Some(NonPlayerEntityCommon {
            kind,
            common,
            custom_name,
            custom_name_visible,
        })
    }
}

struct EntityCommon {
    id: EntityId,
    uuid: uuid::Uuid,
    air_ticks: i16,
    fall_distance: f64,
    fire_ticks: i16,
    glowing: bool,
    visual_fire: bool,
    invulnerable: bool,
    velocity: Vector3<f64>,
    gravity: bool,
    on_ground: bool,
    //TODO: passengers
    portal_cooldown_ticks: i32,
    pos: Vector3<f64>,
    yaw: f32,
    pitch: f32,
    silent: Option<bool>,
    //TODO: scoreboard tags
}

impl EntityCommon {
    fn from_nbt(nbt: &NbtCompound) -> Option<Self> {
        let id = CURRENT_ID.fetch_add(1, Ordering::Relaxed);

        let raw_uuid = nbt.get_int_array("UUID")?;
        #[allow(clippy::get_first)]
        let uuid = uuid::Uuid::from_u64_pair(
            (*raw_uuid.get(0)? as u64) << 32 | (*raw_uuid.get(1)? as u64),
            (*raw_uuid.get(2)? as u64) << 32 | (*raw_uuid.get(3)? as u64),
        );

        let air_ticks = nbt.get_short("Air")?;
        let fall_distance = nbt.get_double("fall_distance")?;
        let fire_ticks = nbt.get_short("Fire")?;
        let glowing = nbt.get_bool("Glowing")?;
        let visual_fire = nbt.get_bool("HasVisualFire")?;
        let invulnerable = nbt.get_bool("Invulnerable")?;

        let motion = nbt.get_list("Motion")?;
        #[allow(clippy::get_first)]
        let velocity = Vector3::new(
            motion.get(0)?.extract_double()?,
            motion.get(1)?.extract_double()?,
            motion.get(2)?.extract_double()?,
        );

        let gravity = !nbt.get_bool("NoGravity")?;
        let on_ground = nbt.get_bool("OnGround")?;

        let portal_cooldown_ticks = nbt.get_int("PortalCooldown")?;

        let pos_arr = nbt.get_list("Pos")?;
        #[allow(clippy::get_first)]
        let pos = Vector3::new(
            pos_arr.get(0)?.extract_double()?,
            pos_arr.get(1)?.extract_double()?,
            pos_arr.get(2)?.extract_double()?,
        );

        let rotation = nbt.get_list("Rotation")?;
        #[allow(clippy::get_first)]
        let yaw = rotation.get(0)?.extract_float()?;
        let pitch = rotation.get(1)?.extract_float()?;

        let silent = nbt.get_bool("Silent");

        Some(EntityCommon {
            id,
            uuid,
            air_ticks,
            fall_distance,
            fire_ticks,
            glowing,
            visual_fire,
            invulnerable,
            velocity,
            gravity,
            on_ground,
            portal_cooldown_ticks,
            pos,
            yaw,
            pitch,
            silent,
        })
    }
}
