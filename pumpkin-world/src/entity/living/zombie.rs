use pumpkin_nbt::compound::NbtCompound;

use crate::entity::MobCommon;

pub struct ZombieCommon {
    pub mob_common: MobCommon,
    pub is_baby: bool,
    // TODO: Others
}

impl ZombieCommon {
    pub(crate) fn from_nbt(nbt: &NbtCompound) -> Option<Self> {
        let mob_common = MobCommon::from_nbt(nbt)?;
        let is_baby = nbt.get_bool("IsBaby").unwrap_or(false);

        Some(Self {
            mob_common,
            is_baby,
        })
    }
}
