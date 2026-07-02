//! L2 protocol-conformance pilots: drive serverbound packets as real bytes
//! through `handle_play_packet` (the actual decode + dispatch), then assert on
//! decoded server state or on captured clientbound frames.

use pumpkin_data::Block;
use pumpkin_data::item::Item;
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_protocol::java::client::play::CPingResponse;
use pumpkin_protocol::java::server::play::{SPlayPingRequest, SPlayerAction, SPlayerPosition};
use pumpkin_test::TestServer;
use pumpkin_test_macros::pumpkin_test;
use pumpkin_util::GameMode;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector3::Vector3;

/// A ping request decoded from bytes produces a matching pong, captured as a
/// real clientbound frame.
#[pumpkin_test(seed = 12345)]
async fn ping_request_gets_a_pong(mut t: TestServer) {
    let player = t.add_player("Pinger", GameMode::Survival).await;

    player
        .send_serverbound(&SPlayPingRequest { payload: 987_654 })
        .await;

    player.assert_clientbound_sent(&CPingResponse::new(987_654));
}

/// A position packet decoded from bytes moves the player.
#[pumpkin_test(seed = 12345)]
async fn position_packet_moves_player(mut t: TestServer) {
    let player = t.add_player("Walker", GameMode::Survival).await;

    player
        .send_serverbound(&SPlayerPosition {
            position: Vector3::new(3.0, 100.0, 4.0),
            collision: 1,
        })
        .await;

    let pos = player.position();
    assert!((pos.x - 3.0).abs() < 1e-6, "x should update, got {}", pos.x);
    assert!(
        (pos.y - 100.0).abs() < 1e-6,
        "y should update, got {}",
        pos.y
    );
    assert!((pos.z - 4.0).abs() < 1e-6, "z should update, got {}", pos.z);
}

/// A player-action packet decoded from bytes breaks a block. `SPlayerAction` is
/// serializable only under the `test-harness` feature, which is how a
/// deserialize-only serverbound packet becomes drivable from a test.
#[pumpkin_test(seed = 12345)]
async fn player_action_breaks_block(mut t: TestServer) {
    t.insert_empty_chunk(0, 0);
    let player = t.add_player("Miner", GameMode::Survival).await;
    player.set_main_hand(&Item::DIAMOND_PICKAXE, 1).await;

    let pos = BlockPos::new(0, 99, 0);
    t.set_block(pos, &Block::STONE).await;

    player
        .send_serverbound(&SPlayerAction {
            status: VarInt(2), // FinishedDigging
            position: pos,
            face: 1,
            sequence: VarInt(0),
        })
        .await;
    t.tick_n(1).await;

    assert_eq!(t.block_at(pos).id, Block::AIR.id, "block should be broken");
}
