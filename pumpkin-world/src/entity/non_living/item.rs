use crate::{entity::NonPlayerCommon, item::ItemStack};

pub struct Item {
    pub common: NonPlayerCommon,
    pub age_ticks: i16,
    pub pickup_delay_ticks: i16,
    pub health: i16,
    pub stack: ItemStack,
    // TODO: Owners
}
