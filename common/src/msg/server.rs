use super::{ClientState, EcsCompPacket};
use crate::{
    character::CharacterItem,
    comp,
    outcome::Outcome,
    recipe::RecipeBook,
    state, sync,
    sync::Uid,
    terrain::{Block, TerrainChunk},
};
use authc::AuthClientError;
use hashbrown::HashMap;
use serde::{Deserialize, Serialize};
use vek::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub description: String,
    pub git_hash: String,
    pub git_date: String,
    pub auth_provider: Option<String>,
}

/// Inform the client of updates to the player list.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlayerListUpdate {
    Init(HashMap<Uid, PlayerInfo>),
    Add(Uid, PlayerInfo),
    SelectedCharacter(Uid, CharacterInfo),
    LevelChange(Uid, u32),
    Admin(Uid, bool),
    Remove(Uid),
    Alias(Uid, String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub is_admin: bool,
    pub is_online: bool,
    pub player_alias: String,
    pub character: Option<CharacterInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterInfo {
    pub name: String,
    pub level: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InviteAnswer {
    Accepted,
    Declined,
    TimedOut,
}
    
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStats {
    pub player_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Notification {
    WaypointSaved,
}

/// Messages sent from the server to the client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMsg {
    InitialSync {
        entity_package: sync::EntityPackage<EcsCompPacket>,
        server_info: ServerInfo,
        time_of_day: state::TimeOfDay,
        max_group_size: u32,
        world_map: (Vec2<u32>, Vec<u32>),
        recipe_book: RecipeBook,
    },
    /// An error occurred while loading character data
    CharacterDataLoadError(String),
    /// A list of characters belonging to the a authenticated player was sent
    CharacterListUpdate(Vec<CharacterItem>),
    /// An error occured while creating or deleting a character
    CharacterActionError(String),
    PlayerListUpdate(PlayerListUpdate),
    GroupUpdate(comp::group::ChangeNotification<sync::Uid>),
    // Indicate to the client that they are invited to join a group
    GroupInvite {
        inviter: sync::Uid,
        timeout: std::time::Duration,
    },
    // Indicate to the client that their sent invite was not invalid and is currently pending
    InvitePending(sync::Uid),
    // Note: this could potentially include all the failure cases such as inviting yourself in
    // which case the `InvitePending` message could be removed and the client could consider their
    // invite pending until they receive this message
    // Indicate to the client the result of their invite
    InviteComplete {
        target: sync::Uid,
        answer: InviteAnswer,
    },
    StateAnswer(Result<ClientState, (RequestStateError, ClientState)>),
    /// Trigger cleanup for when the client goes back to the `Registered` state
    /// from an ingame state
    ExitIngameCleanup,
    Ping,
    Pong,
    /// A message to go into the client chat box. The client is responsible for
    /// formatting the message and turning it into a speech bubble.
    ChatMsg(comp::ChatMsg),
    SetPlayerEntity(Uid),
    TimeOfDay(state::TimeOfDay),
    EntitySync(sync::EntitySyncPackage),
    CompSync(sync::CompSyncPackage<EcsCompPacket>),
    CreateEntity(sync::EntityPackage<EcsCompPacket>),
    DeleteEntity(Uid),
    InventoryUpdate(comp::Inventory, comp::InventoryUpdateEvent),
    TerrainChunkUpdate {
        key: Vec2<i32>,
        chunk: Result<Box<TerrainChunk>, ()>,
    },
    TerrainBlockUpdates(HashMap<Vec3<i32>, Block>),
    Disconnect,
    Shutdown,
    TooManyPlayers,
    /// Send a popup notification such as "Waypoint Saved"
    Notification(Notification),
    SetViewDistance(u32),
    Outcomes(Vec<Outcome>),
    ServerStats(ServerStats),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RequestStateError {
    RegisterDenied(RegisterError),
    Denied,
    Already,
    Impossible,
    WrongMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RegisterError {
    AlreadyLoggedIn,
    AuthError(String),
    InvalidCharacter,
    NotOnWhitelist,
    //TODO: InvalidAlias,
}

impl From<AuthClientError> for RegisterError {
    fn from(err: AuthClientError) -> Self { Self::AuthError(err.to_string()) }
}

impl From<comp::ChatMsg> for ServerMsg {
    fn from(v: comp::ChatMsg) -> Self { ServerMsg::ChatMsg(v) }
}
