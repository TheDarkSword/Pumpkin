//! In-memory test harness for the Pumpkin server (level L1).
//!
//! Builds a real [`Server`] on a temporary directory with a fixed world seed,
//! attaches players backed by a real `JavaClient` over a loopback socket, and
//! advances game logic deterministically by driving `Server` ticks directly
//! (bypassing the sleeping `Ticker`). No production code paths are altered; the
//! server exposes only additive helpers behind its `test-harness` feature.

use std::net::SocketAddr;
use std::sync::Arc;

use arc_swap::ArcSwap;
use bytes::Bytes;
use pumpkin::data::VanillaData;
use pumpkin::entity::player::Player;
use pumpkin::net::java::JavaClient;
use pumpkin::net::{ClientPlatform, GameProfile, PlayerConfig, offline_uuid};
use pumpkin::server::Server;
use pumpkin_config::{AdvancedConfiguration, BasicConfiguration};
use pumpkin_data::dimension::Dimension;
use pumpkin_protocol::ConnectionState;
use pumpkin_util::GameMode;
use pumpkin_util::world_seed::Seed;
use tempfile::TempDir;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::UnboundedReceiver;

/// A running in-memory server plus the temporary world directory backing it.
pub struct TestServer {
    /// The real server instance under test.
    pub server: Arc<Server>,
    /// Kept alive so the temporary world directory is removed on drop.
    _temp_dir: TempDir,
    /// Monotonic id handed to each mock client.
    next_client_id: u64,
}

impl TestServer {
    /// Boots a server on a fresh temporary directory with the given world seed,
    /// in offline mode and with only the overworld dimension enabled.
    pub async fn new(seed: u64) -> Self {
        let temp_dir = tempfile::tempdir().expect("failed to create temp world dir");
        let basic = BasicConfiguration {
            seed: Seed(seed),
            online_mode: false,
            encryption: false,
            allow_chat_reports: false,
            allow_nether: false,
            allow_end: false,
            java_edition: false,
            bedrock_edition: false,
            default_level_name: temp_dir.path().to_string_lossy().into_owned(),
            ..Default::default()
        };
        let server = Server::new(
            basic,
            AdvancedConfiguration::default(),
            VanillaData::for_test(),
        )
        .await;
        Self {
            server,
            _temp_dir: temp_dir,
            next_client_id: 0,
        }
    }

    /// Advances the full server tick (world logic, then player/network flush)
    /// `n` times. This replaces the sleeping `Ticker` loop with deterministic,
    /// synchronous stepping.
    pub async fn tick_n(&self, n: u32) {
        for _ in 0..n {
            self.server.tick_worlds().await;
            self.server.tick_players_and_network().await;
        }
    }

    /// Attaches a new player to the overworld, backed by a real `JavaClient`
    /// over a loopback socket whose clientbound frames are captured instead of
    /// being written to the wire.
    pub async fn add_player(&mut self, name: &str, gamemode: GameMode) -> TestPlayer {
        let (server_stream, client_stream, peer) = loopback_pair().await;

        let id = self.next_client_id;
        self.next_client_id += 1;

        let mut java = JavaClient::new(server_stream, peer, id);
        java.connection_state.store(ConnectionState::Play);
        let clientbound = java.start_test_capture();

        let client = Arc::new(ClientPlatform::Java(java));

        let profile = GameProfile {
            id: offline_uuid(name).expect("valid offline uuid"),
            name: name.to_owned(),
            properties: ArcSwap::new(Arc::new(Vec::new())),
            profile_actions: None,
        };

        let world = self.server.get_world_from_dimension(&Dimension::OVERWORLD);
        let player = Arc::new(
            Player::new(
                client.clone(),
                profile,
                PlayerConfig::default(),
                world.clone(),
                gamemode,
            )
            .await,
        );

        // Link the client back to its player, mirroring the real login flow.
        if let Some(java) = client.java() {
            *java.player.lock().await = Some(player.clone());
        }
        world.add_player(&player).expect("add player to world");
        player.set_client_loaded(true);

        TestPlayer {
            player,
            clientbound,
            _client_stream: client_stream,
        }
    }
}

/// A player attached to a [`TestServer`].
pub struct TestPlayer {
    /// The live player entity.
    pub player: Arc<Player>,
    /// Raw serialized clientbound frames the server tried to send this player.
    pub clientbound: UnboundedReceiver<Bytes>,
    /// The client end of the loopback socket, kept open for the connection's life.
    _client_stream: TcpStream,
}

/// Creates a connected loopback TCP pair, returning the server half, the client
/// half, and the server-perceived peer address.
async fn loopback_pair() -> (TcpStream, TcpStream, SocketAddr) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback listener");
    let addr = listener.local_addr().expect("listener local addr");
    let (connect_res, accept_res) = tokio::join!(TcpStream::connect(addr), listener.accept());
    let client_stream = connect_res.expect("connect loopback client");
    let (server_stream, peer) = accept_res.expect("accept loopback server");
    (server_stream, client_stream, peer)
}
