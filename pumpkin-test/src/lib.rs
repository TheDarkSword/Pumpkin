//! In-memory test harness for the Pumpkin server (level L1).
//!
//! Builds a real [`Server`] on a temporary directory with a fixed world seed,
//! attaches players backed by a real `JavaClient` over a loopback socket, and
//! advances game logic deterministically by driving `Server` ticks directly
//! (bypassing the sleeping `Ticker`). Player actions are pushed through the
//! real public serverbound handlers, not through shortcuts. No production code
//! paths are altered; the server exposes only additive helpers behind its
//! `test-harness` feature.

use std::net::SocketAddr;
use std::num::NonZeroU8;
use std::sync::Arc;

use arc_swap::ArcSwap;
use bytes::Bytes;
use pumpkin::data::VanillaData;
use pumpkin::entity::EntityBase;
use pumpkin::entity::item::ItemEntity;
use pumpkin::entity::player::Player;
use pumpkin::net::java::{ClientboundCapture, JavaClient};
use pumpkin::net::{ClientPlatform, GameProfile, PlayerConfig, offline_uuid};
use pumpkin::server::Server;
use pumpkin::world::World;
use pumpkin_config::lighting::LightingEngineConfig;
use pumpkin_config::world::LevelConfig;
use pumpkin_config::{AdvancedConfiguration, BasicConfiguration};
use pumpkin_data::Block;
use pumpkin_data::dimension::Dimension;
use pumpkin_data::item::Item;
use pumpkin_data::item_stack::ItemStack;
use pumpkin_protocol::codec::item_stack_seralizer::ItemStackSerializer;
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_protocol::java::server::play::{
    SPlayerAction, SPlayerPosition, SSetCreativeSlot, SUseItemOn,
};
use pumpkin_protocol::{ClientPacket, ConnectionState, RawPacket};
use pumpkin_util::GameMode;
use pumpkin_util::math::position::BlockPos;
use pumpkin_util::math::vector2::Vector2;
use pumpkin_util::math::vector3::Vector3;
use pumpkin_util::world_seed::Seed;
use pumpkin_world::chunk::ChunkData;
use pumpkin_world::world::BlockFlags;
use tempfile::TempDir;
use tokio::net::{TcpListener, TcpStream};

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
    /// in offline mode, with only the overworld enabled and a small view
    /// distance to keep on-demand chunk generation cheap.
    pub async fn new(seed: i64) -> Self {
        let temp_dir = tempfile::tempdir().expect("failed to create temp world dir");
        let basic = BasicConfiguration {
            // Minecraft world seeds are signed (Java i64); Pumpkin stores the
            // same bits as u64 and casts back with `seed.0 as i64`.
            seed: Seed(seed as u64),
            online_mode: false,
            encryption: false,
            allow_chat_reports: false,
            allow_nether: false,
            allow_end: false,
            java_edition: false,
            bedrock_edition: false,
            view_distance: NonZeroU8::new(2).expect("nonzero"),
            simulation_distance: NonZeroU8::new(2).expect("nonzero"),
            default_level_name: temp_dir.path().to_string_lossy().into_owned(),
            ..Default::default()
        };
        let advanced = AdvancedConfiguration {
            world: LevelConfig {
                // Skip sky/block light propagation: it keeps block edits O(1)
                // and avoids spinning on the empty test chunks, whose light
                // arrays are not populated.
                lighting: LightingEngineConfig::Full,
                ..Default::default()
            },
            ..Default::default()
        };
        let server = Server::new(basic, advanced, VanillaData::for_test()).await;
        Self {
            server,
            _temp_dir: temp_dir,
            next_client_id: 0,
        }
    }

    /// The overworld world.
    #[must_use]
    pub fn world(&self) -> Arc<World> {
        self.server.get_world_from_dimension(&Dimension::OVERWORLD)
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

    /// Runs real vanilla world generation for the chunk at the given
    /// coordinates and loads it. Slow (terrain plus structures); use it only
    /// when a test actually needs generated terrain. For tests that only rely
    /// on blocks they set themselves, prefer [`Self::insert_empty_chunk`].
    pub async fn load_chunk(&self, chunk_x: i32, chunk_z: i32) {
        self.world()
            .level
            .get_or_fetch_chunk(Vector2::new(chunk_x, chunk_z), |_| ())
            .await;
    }

    /// Inserts an empty (all-air) chunk at the given coordinates without running
    /// world generation. This is the fast path: no terrain, no structures, no
    /// generation threads. The test sets whatever blocks it needs afterwards.
    pub fn insert_empty_chunk(&self, chunk_x: i32, chunk_z: i32) {
        self.world().level.loaded_chunks.insert(
            Vector2::new(chunk_x, chunk_z),
            Arc::new(ChunkData::empty_overworld(chunk_x, chunk_z)),
        );
    }

    /// Sets the block state at `pos` to `block`'s default state.
    pub async fn set_block(&self, pos: BlockPos, block: &'static Block) {
        self.world()
            .set_block_state(&pos, block.default_state.id, BlockFlags::NOTIFY_ALL)
            .await;
    }

    /// Reads the block at `pos`.
    #[must_use]
    pub fn block_at(&self, pos: BlockPos) -> &'static Block {
        self.world().get_block(&pos)
    }

    /// Collects the item entities within `radius` of `center`.
    #[must_use]
    pub fn item_entities_near(&self, center: Vector3<f64>, radius: f64) -> Vec<Arc<ItemEntity>> {
        self.world()
            .get_nearby_entities(center, radius)
            .into_values()
            .filter_map(EntityBase::get_item_entity)
            .collect()
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

        let world = self.world();
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
            server: self.server.clone(),
            clientbound,
            _client_stream: client_stream,
        }
    }
}

/// A player attached to a [`TestServer`].
pub struct TestPlayer {
    /// The live player entity.
    pub player: Arc<Player>,
    /// The server the player belongs to (needed to invoke real handlers).
    server: Arc<Server>,
    /// Captures the clientbound frames the server queued for this player.
    clientbound: ClientboundCapture,
    /// The client end of the loopback socket, kept open for the connection's life.
    _client_stream: TcpStream,
}

impl TestPlayer {
    /// The underlying mock Java client.
    fn client(&self) -> &JavaClient {
        self.player.client.java().expect("java test client")
    }

    /// Directly sets the item in the player's selected hotbar slot (test setup).
    pub async fn set_main_hand(&self, item: &'static Item, count: u8) {
        *self.player.inventory().held_item().lock().await = ItemStack::new(count, item);
    }

    /// Returns a clone of the item in the selected hotbar slot.
    pub async fn held_item(&self) -> ItemStack {
        self.player.inventory().held_item().lock().await.clone()
    }

    /// The player's current position.
    #[must_use]
    pub fn position(&self) -> Vector3<f64> {
        self.player.get_entity().pos.load()
    }

    /// Breaks the block at `pos` by sending a `FinishedDigging` action through
    /// the real `handle_player_action` pipeline.
    pub async fn break_block(&self, pos: BlockPos) {
        let action = SPlayerAction {
            status: VarInt(2), // FinishedDigging
            position: pos,
            face: 1, // Up
            sequence: VarInt(0),
        };
        self.client()
            .handle_player_action(&self.player, action, self.server.as_ref())
            .await;
    }

    /// Places the held block against the top face of the block at `pos`,
    /// through the real `handle_use_item_on` pipeline.
    pub async fn place_block_on_top(&self, pos: BlockPos) {
        let packet = SUseItemOn {
            hand: VarInt(0), // main hand
            position: pos,
            face: VarInt(1), // Up
            cursor_pos: Vector3::new(0.5f32, 1.0, 0.5),
            inside_block: false,
            is_against_world_border: false,
            sequence: VarInt(0),
        };
        let _ = self
            .client()
            .handle_use_item_on(&self.player, packet, &self.server)
            .await;
    }

    /// Moves the player to `pos` through the real `handle_position` pipeline.
    pub async fn move_to(&self, pos: Vector3<f64>) {
        let packet = SPlayerPosition {
            position: pos,
            collision: 1, // on-ground flag
        };
        self.client()
            .handle_position(&self.player, &self.server, packet)
            .await;
    }

    /// Sets a creative-mode container slot to the given item, through the real
    /// `handle_set_creative_slot` pipeline (player must be in creative mode).
    pub async fn set_creative_slot(&self, slot: i16, item: &'static Item, count: u8) {
        let packet = SSetCreativeSlot {
            slot,
            clicked_item: ItemStackSerializer::from(ItemStack::new(count, item)),
        };
        self.client()
            .handle_set_creative_slot(&self.player, packet)
            .await
            .expect("creative slot set");
    }

    /// Encodes `packet` and feeds it through the real serverbound byte pipeline
    /// (`handle_play_packet`), so the actual decode and dispatch run — unlike the
    /// typed action helpers above, which call the handlers directly. The packet
    /// must be encodable (a `ClientPacket`, i.e. it derives `Serialize`).
    pub async fn send_serverbound<P: ClientPacket>(&self, packet: &P) {
        let version = self.client().version.load();
        let mut body = Vec::new();
        packet
            .write_packet_data(&mut body, &version)
            .expect("encode serverbound packet");
        let raw = RawPacket {
            id: P::to_id(version),
            payload: body.into(),
        };
        if let Err(error) = self
            .client()
            .handle_play_packet(&self.player, &self.server, &raw)
            .await
        {
            panic!("handle_play_packet returned an error: {error}");
        }
    }

    /// Returns the raw clientbound frames the server has queued for this player
    /// so far, in send order.
    #[must_use]
    pub fn clientbound_frames(&self) -> Vec<Bytes> {
        self.clientbound.drain()
    }

    /// Asserts the server sent a clientbound packet byte-identical to `expected`
    /// (re-serialized with the client's protocol version).
    pub fn assert_clientbound_sent<P: ClientPacket>(&self, expected: &P) {
        let version = self.client().version.load();
        let expected_bytes = JavaClient::serialize_packet_for_version(expected, version)
            .expect("serialize expected clientbound packet");
        let frames = self.clientbound_frames();
        assert!(
            frames.iter().any(|frame| frame == &expected_bytes),
            "expected clientbound packet was not among the {} captured frame(s)",
            frames.len()
        );
    }
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
