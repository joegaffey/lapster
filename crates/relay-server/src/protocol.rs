use serde::{Deserialize, Serialize};

pub const STATE_FLOATS: usize = 64;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Join {
        room: String,
        #[serde(default)]
        password: String,
        name: String,
        car: String,
        track: String,
    },
    State {
        seq: u64,
        tick: u64,
        state: Vec<f32>,
    },
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome {
        player_id: u16,
        slot: usize,
        room: String,
        car: String,
        track: String,
        max_players: usize,
        players: Vec<PlayerInfo>,
        states: Vec<StateInfo>,
    },
    Roster {
        players: Vec<PlayerInfo>,
    },
    State {
        player_id: u16,
        seq: u64,
        tick: u64,
        state: Vec<f32>,
    },
    PlayerLeft {
        player_id: u16,
    },
    Error {
        message: String,
    },
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub id: u16,
    pub slot: usize,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateInfo {
    pub player_id: u16,
    pub seq: u64,
    pub tick: u64,
    pub state: Vec<f32>,
}

impl ClientMessage {
    pub fn parse(text: &str) -> Result<Self, String> {
        serde_json::from_str(text).map_err(|error| format!("invalid message: {error}"))
    }
}

pub fn valid_state(state: &[f32]) -> bool {
    state.len() == STATE_FLOATS && state.iter().all(|value| value.is_finite())
}
