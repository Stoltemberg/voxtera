pub mod client;
pub mod compression;
pub mod ecs_packet;
pub mod server;
pub mod world_msg;

// Reexports
pub use self::{
    client::{ClientGeneral, ClientMsg, ClientRegister, ClientType},
    compression::{
        CompressedData, GridLtrPacking, PackingFormula, QuadPngEncoding, TriPngEncoding,
        VoxelImageEncoding, WidePacking, WireChonk,
    },
    ecs_packet::EcsCompPacket,
    server::{
        CharacterInfo, ChatTypeContext, DisconnectReason, InviteAnswer, Notification, PlayerInfo,
        PlayerListUpdate, RegisterError, SerializedTerrainChunk, ServerGeneral, ServerInfo,
        ServerInit, ServerMsg, ServerRegisterAnswer,
    },
    world_msg::WorldMapMsg,
};
use serde::{Deserialize, Serialize};

/// Relationship state shown by the social panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FriendStatus {
    PendingOutgoing,
    PendingIncoming,
    Accepted,
}

/// Friend entry sent to the client UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FriendInfo {
    pub alias: String,
    pub status: FriendStatus,
    pub online: bool,
}

/// Actions initiated by the visual friends panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FriendAction {
    RequestList,
    Add(String),
    Accept(String),
    Reject(String),
    Remove(String),
}

/// Actions initiated by the admin panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdminAction {
    TeleportTo(common::uid::Uid),
    TeleportHere(common::uid::Uid),
    Kick(common::uid::Uid),
    Mute(common::uid::Uid, std::time::Duration),
    Unmute(common::uid::Uid),
    GiveItem(String, u16),
    GodMode,
    TogglePvp,
    Announce(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PingMsg {
    Ping,
    Pong,
}
