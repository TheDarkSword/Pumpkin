//! M0 smoke test: boot a server on a temp dir, attach a player, and tick.

use pumpkin_test::TestServer;
use pumpkin_test_macros::pumpkin_test;
use pumpkin_util::GameMode;

#[pumpkin_test(seed = -22837152948219)]
async fn create_world_add_player_and_tick(mut t: TestServer) {
    let player = t.add_player("Tester", GameMode::Survival).await;
    assert_eq!(player.player.gameprofile.name, "Tester");
    assert!(player.player.has_client_loaded());

    // Advancing ticks must not panic and must leave the player registered.
    t.tick_n(3).await;

    let worlds = t.server.worlds.load();
    let player_count: usize = worlds.iter().map(|w| w.players.load().len()).sum();
    assert_eq!(player_count, 1);
}
