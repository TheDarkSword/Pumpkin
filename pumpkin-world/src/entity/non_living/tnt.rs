use crate::entity::NonPlayerCommon;

pub struct Tnt {
    pub common: NonPlayerCommon,
    pub fuse_ticks: i16,
    pub explosion_power: f32,
    // TODO: block state
}
