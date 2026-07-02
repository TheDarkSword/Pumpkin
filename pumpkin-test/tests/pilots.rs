//! M1 pilot tests: each drives a real public serverbound handler and asserts
//! the resulting game state, deterministically and without real networking.

use pumpkin_data::Block;
use pumpkin_data::item::Item;
use pumpkin_test::TestServer;
use pumpkin_test_macros::pumpkin_test;
use pumpkin_util::GameMode;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;

/// Breaking a stone block with a pickaxe in survival drops one cobblestone.
#[pumpkin_test(seed = 12345)]
async fn break_stone_drops_cobblestone(mut t: TestServer) {
    t.insert_empty_chunk(0, 0);

    let player = t.add_player("Miner", GameMode::Survival).await;
    player.set_main_hand(&Item::DIAMOND_PICKAXE, 1).await;

    let pos = BlockPos::new(0, 99, 0);
    t.set_block(pos, &Block::STONE).await;
    assert_eq!(
        t.block_at(pos).id,
        Block::STONE.id,
        "stone should be placed"
    );

    player.break_block(pos).await;
    t.tick_n(1).await;

    assert_eq!(t.block_at(pos).id, Block::AIR.id, "block should be broken");

    let drops = t.item_entities_near(Vector3::new(0.5, 99.5, 0.5), 2.0);
    assert_eq!(drops.len(), 1, "expected exactly one dropped item entity");
    let stack = drops[0].stack().await;
    assert_eq!(
        stack.item.id,
        Item::COBBLESTONE.id,
        "should drop cobblestone"
    );
    assert_eq!(stack.item_count, 1);
}

/// Using a block item on the top face of a block places a new block above it.
#[pumpkin_test(seed = 12345)]
async fn place_block_on_top_of_stone(mut t: TestServer) {
    t.insert_empty_chunk(0, 0);

    let player = t.add_player("Builder", GameMode::Survival).await;
    player.set_main_hand(&Item::STONE, 64).await;

    let base = BlockPos::new(2, 99, 0);
    let target = BlockPos::new(2, 100, 0);
    t.set_block(base, &Block::STONE).await;
    t.set_block(target, &Block::AIR).await;

    player.place_block_on_top(base).await;
    t.tick_n(1).await;

    assert_eq!(
        t.block_at(target).id,
        Block::STONE.id,
        "a stone block should be placed on top"
    );
}

/// A position packet moves the player's entity to the requested coordinates.
#[pumpkin_test(seed = 12345)]
async fn player_position_updates(mut t: TestServer) {
    let player = t.add_player("Walker", GameMode::Survival).await;

    let target = Vector3::new(1.0, 100.0, 2.0);
    player.move_to(target).await;

    let pos = player.position();
    assert!((pos.x - 1.0).abs() < 1e-6, "x should update, got {}", pos.x);
    assert!(
        (pos.y - 100.0).abs() < 1e-6,
        "y should update, got {}",
        pos.y
    );
    assert!((pos.z - 2.0).abs() < 1e-6, "z should update, got {}", pos.z);
}

/// Setting a creative hotbar slot updates the player's held item.
#[pumpkin_test(seed = 12345)]
async fn creative_set_slot_updates_held_item(mut t: TestServer) {
    let player = t.add_player("Creator", GameMode::Creative).await;

    // Container slot 36 is the first hotbar slot; the selected slot defaults to 0.
    player
        .set_creative_slot(36, &Item::DIAMOND_PICKAXE, 1)
        .await;

    let held = player.held_item().await;
    assert_eq!(
        held.item.id,
        Item::DIAMOND_PICKAXE.id,
        "held item should be set"
    );
    assert_eq!(held.item_count, 1);
}
