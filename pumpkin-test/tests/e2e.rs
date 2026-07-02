//! L3 end-to-end pilot: a headless bot speaks the real Java protocol over a
//! real loopback TCP socket against a running server, and completes the
//! server-list status exchange (handshake -> status -> ping/pong).

use pumpkin_data::packet::CURRENT_MC_VERSION;
use pumpkin_protocol::codec::var_int::VarInt;
use pumpkin_protocol::java::client::status::{CPingResponse, CStatusResponse};
use pumpkin_protocol::java::server::handshake::SHandShake;
use pumpkin_protocol::java::server::status::{SStatusPingRequest, SStatusRequest};
use pumpkin_protocol::{ConnectionState, MultiVersionJavaPacket, ServerPacket};
use pumpkin_test::{TestBot, TestServer};
use pumpkin_test_macros::pumpkin_test;

/// A headless bot completes the status exchange with a real server over TCP.
#[pumpkin_test]
async fn status_ping_roundtrips(t: TestServer) {
    let addr = t.spawn_java_listener().await;
    let mut bot = TestBot::connect(addr).await;
    let version = CURRENT_MC_VERSION;

    // Handshake requesting the Status state. The handshake is always id 0.
    bot.send_raw(
        0,
        &SHandShake {
            protocol_version: VarInt(version.protocol_version()),
            server_address: "127.0.0.1".into(),
            server_port: addr.port(),
            next_state: ConnectionState::Status,
        },
    )
    .await;

    // Status request -> status response carrying the server-list JSON.
    bot.send(&SStatusRequest {}).await;
    let raw = bot.recv().await;
    assert_eq!(raw.id, CStatusResponse::to_id(version));
    let status = CStatusResponse::read(&raw.payload[..], &version).expect("decode status response");
    assert!(
        status.json_response.contains("version"),
        "status json should describe the version: {}",
        status.json_response
    );

    // Ping -> pong echoing the same payload.
    bot.send(&SStatusPingRequest { payload: 424_242 }).await;
    let raw = bot.recv().await;
    assert_eq!(raw.id, CPingResponse::to_id(version));
    let pong = CPingResponse::read(&raw.payload[..], &version).expect("decode pong");
    assert_eq!(pong.payload, 424_242);
}
