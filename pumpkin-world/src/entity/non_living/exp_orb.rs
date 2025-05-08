use pumpkin_nbt::compound::NbtCompound;

use crate::entity::NonPlayerCommon;

pub struct ExpOrb {
    pub common: NonPlayerCommon,
    pub age_ticks: i16,
    pub merge_count: i32,
    pub health: i16,
    pub value: i16,
}

impl ExpOrb {
    pub(crate) fn from_nbt(nbt: &NbtCompound) -> Option<Self> {
        let common = NonPlayerCommon::from_nbt(nbt)?;
        let age_ticks = nbt.get_short("Age")?;
        let merge_count = nbt.get_int("Count")?;
        let health = nbt.get_short("Health")?;
        let value = nbt.get_short("Value")?;

        Some(Self {
            common,
            age_ticks,
            merge_count,
            health,
            value,
        })
    }
}
