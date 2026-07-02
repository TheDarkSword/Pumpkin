//! Data-driven pilots: assert known vanilla facts against Pumpkin's compiled
//! data (recipes, tags, block loot). These are pure static queries — no server,
//! no world — so the expected values act as an independent vanilla oracle.

use pumpkin::world::loot::{LootContextParameters, LootTableExt};
use pumpkin_data::Block;
use pumpkin_data::item::Item;
use pumpkin_data::recipes::{CraftingRecipeTypes, RECIPES_CRAFTING};
use pumpkin_data::tag::{self, Taggable};

/// One oak log crafts into four oak planks (a shapeless vanilla recipe).
#[test]
fn oak_log_makes_four_oak_planks() {
    let result = RECIPES_CRAFTING
        .iter()
        .find_map(|recipe| match recipe {
            CraftingRecipeTypes::CraftingShapeless {
                ingredients,
                result,
                ..
            } if ingredients.iter().any(|i| i.match_item(&Item::OAK_LOG)) => Some(result),
            _ => None,
        })
        .expect("a shapeless recipe consuming an oak log should exist");

    assert_eq!(
        Item::from_registry_key(result.id).map(|item| item.id),
        Some(Item::OAK_PLANKS.id)
    );
    assert_eq!(result.count, 4);
}

/// Oak planks belong to the `minecraft:planks` item tag.
#[test]
fn oak_planks_are_planks() {
    assert_eq!(
        Item::OAK_PLANKS.is_tagged_with("minecraft:planks"),
        Some(true)
    );
    assert!(Item::OAK_PLANKS.has_tag(&tag::Item::MINECRAFT_PLANKS));
}

/// Stone is mineable with a pickaxe.
#[test]
fn stone_is_mineable_with_pickaxe() {
    assert_eq!(
        Block::STONE.is_tagged_with("minecraft:mineable/pickaxe"),
        Some(true)
    );
    assert!(Block::STONE.has_tag(&tag::Block::MINECRAFT_MINEABLE_PICKAXE));
}

/// Breaking stone with no tool (no silk touch) yields exactly one cobblestone.
#[test]
fn stone_loot_is_one_cobblestone() {
    let table = Block::STONE.loot_table.expect("stone has a loot table");
    let drops = table.get_loot(LootContextParameters::default());

    assert_eq!(drops.len(), 1, "stone should drop a single item");
    assert_eq!(drops[0].item.id, Item::COBBLESTONE.id);
    assert_eq!(drops[0].item_count, 1);
}
