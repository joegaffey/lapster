use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::broadcast;

use crate::protocol::{valid_state, PlayerInfo, ServerMessage, StateInfo};

const BROADCAST_CAPACITY: usize = 64;

pub struct Hub {
    rooms: Mutex<HashMap<String, Room>>,
    next_player_id: AtomicU16,
    max_players: usize,
}

struct Room {
    password: Option<String>,
    car: String,
    track: String,
    players: HashMap<u16, Player>,
    tx: broadcast::Sender<Arc<String>>,
}

struct Player {
    slot: usize,
    name: String,
    seq: u64,
    tick: u64,
    state: Vec<f32>,
}

pub struct JoinRequest {
    pub room: String,
    pub password: String,
    pub name: String,
    pub car: String,
    pub track: String,
}

pub struct JoinResult {
    pub player_id: u16,
    pub slot: usize,
    pub receiver: broadcast::Receiver<Arc<String>>,
    pub welcome: ServerMessage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinError {
    InvalidRoom,
    InvalidName,
    InvalidAsset,
    RoomFull,
    WrongPassword,
    AssetMismatch,
}

impl fmt::Display for JoinError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRoom => "invalid room code",
            Self::InvalidName => "invalid player name",
            Self::InvalidAsset => "invalid car or track",
            Self::RoomFull => "room is full",
            Self::WrongPassword => "wrong room password",
            Self::AssetMismatch => "room is using a different car or track",
        })
    }
}

impl Hub {
    pub fn new(max_players: usize) -> Self {
        Self {
            rooms: Mutex::new(HashMap::new()),
            next_player_id: AtomicU16::new(1),
            max_players: max_players.clamp(2, 10),
        }
    }

    pub fn join(&self, request: JoinRequest) -> Result<JoinResult, JoinError> {
        let room_code = normalize_room(&request.room)?;
        let name = normalize_name(&request.name)?;
        let car = normalize_asset(&request.car)?;
        let track = normalize_asset(&request.track)?;
        let password = normalize_password(&request.password);

        let mut rooms = self.rooms.lock().expect("room lock poisoned");
        if !rooms.contains_key(&room_code) {
            let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
            rooms.insert(
                room_code.clone(),
                Room {
                    password: password.clone(),
                    car: car.clone(),
                    track: track.clone(),
                    players: HashMap::new(),
                    tx,
                },
            );
        }
        let room = rooms.get_mut(&room_code).expect("room was just inserted");

        if room.password.is_some() && room.password.as_deref() != password.as_deref() {
            return Err(JoinError::WrongPassword);
        }
        if room.car != car || room.track != track {
            return Err(JoinError::AssetMismatch);
        }
        if room.players.len() >= self.max_players {
            return Err(JoinError::RoomFull);
        }
        let slot = (0..self.max_players)
            .find(|slot| !room.players.values().any(|player| player.slot == *slot))
            .ok_or(JoinError::RoomFull)?;
        let mut player_id = self.next_player_id.fetch_add(1, Ordering::Relaxed);
        if player_id == 0 {
            player_id = self.next_player_id.fetch_add(1, Ordering::Relaxed);
        }

        room.players.insert(
            player_id,
            Player {
                slot,
                name,
                seq: 0,
                tick: 0,
                state: Vec::new(),
            },
        );

        let players = room.player_list();
        let states = room.state_list();
        let receiver = room.tx.subscribe();
        let welcome = ServerMessage::Welcome {
            player_id,
            slot,
            room: room_code,
            car: room.car.clone(),
            track: room.track.clone(),
            max_players: self.max_players,
            players: players.clone(),
            states,
        };
        broadcast_message(&room.tx, &ServerMessage::Roster { players });

        Ok(JoinResult {
            player_id,
            slot,
            receiver,
            welcome,
        })
    }

    pub fn update_state(
        &self,
        room_code: &str,
        player_id: u16,
        seq: u64,
        tick: u64,
        state: Vec<f32>,
    ) -> bool {
        if !valid_state(&state) {
            return false;
        }
        let mut rooms = self.rooms.lock().expect("room lock poisoned");
        let Some(room) = rooms.get_mut(room_code) else {
            return false;
        };
        let Some(player) = room.players.get_mut(&player_id) else {
            return false;
        };
        if seq <= player.seq {
            return false;
        }
        player.seq = seq;
        player.tick = tick;
        player.state.clone_from(&state);
        broadcast_message(
            &room.tx,
            &ServerMessage::State {
                player_id,
                seq,
                tick,
                state,
            },
        );
        true
    }

    pub fn leave(&self, room_code: &str, player_id: u16) {
        let mut rooms = self.rooms.lock().expect("room lock poisoned");
        let Some(room) = rooms.get_mut(room_code) else {
            return;
        };
        room.players.remove(&player_id);
        if room.players.is_empty() {
            rooms.remove(room_code);
            return;
        }
        let players = room.player_list();
        broadcast_message(
            &room.tx,
            &ServerMessage::Roster {
                players: players.clone(),
            },
        );
        broadcast_message(&room.tx, &ServerMessage::PlayerLeft { player_id });
    }
}

impl Room {
    fn player_list(&self) -> Vec<PlayerInfo> {
        let mut players = self
            .players
            .iter()
            .map(|(id, player)| PlayerInfo {
                id: *id,
                slot: player.slot,
                name: player.name.clone(),
            })
            .collect::<Vec<_>>();
        players.sort_by_key(|player| player.slot);
        players
    }

    fn state_list(&self) -> Vec<StateInfo> {
        self.players
            .iter()
            .filter(|(_, player)| !player.state.is_empty())
            .map(|(id, player)| StateInfo {
                player_id: *id,
                seq: player.seq,
                tick: player.tick,
                state: player.state.clone(),
            })
            .collect()
    }
}

fn broadcast_message(sender: &broadcast::Sender<Arc<String>>, message: &ServerMessage) {
    if let Ok(text) = serde_json::to_string(message) {
        let _ = sender.send(Arc::new(text));
    }
}

fn normalize_room(value: &str) -> Result<String, JoinError> {
    let room = value.trim().to_ascii_uppercase();
    if room.is_empty()
        || room.len() > 32
        || !room
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        return Err(JoinError::InvalidRoom);
    }
    Ok(room)
}

fn normalize_name(value: &str) -> Result<String, JoinError> {
    let name = value.trim();
    if name.is_empty() {
        return Err(JoinError::InvalidName);
    }
    Ok(name.chars().take(24).collect())
}

fn normalize_asset(value: &str) -> Result<String, JoinError> {
    let asset = value.trim();
    if asset.is_empty() || asset.len() > 128 {
        return Err(JoinError::InvalidAsset);
    }
    Ok(asset.to_string())
}

fn normalize_password(value: &str) -> Option<String> {
    let password = value.trim();
    if password.is_empty() {
        None
    } else {
        Some(password.chars().take(64).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_message(text: &str) -> ServerMessage {
        serde_json::from_str(text).unwrap()
    }

    fn request(room: &str, password: &str, name: &str) -> JoinRequest {
        JoinRequest {
            room: room.to_string(),
            password: password.to_string(),
            name: name.to_string(),
            car: "mazda-rx7".to_string(),
            track: "skidpad05".to_string(),
        }
    }

    #[test]
    fn joins_assign_slots_and_reject_bad_passwords() {
        let hub = Hub::new(4);
        let first = hub.join(request("abc123", "secret", "One")).unwrap();
        assert_eq!(first.slot, 0);
        assert!(matches!(
            hub.join(request("ABC123", "wrong", "Two")),
            Err(JoinError::WrongPassword)
        ));
        let second = hub.join(request("abc123", "secret", "Two")).unwrap();
        assert_eq!(second.slot, 1);
    }

    #[test]
    fn room_assets_are_fixed_by_the_first_join() {
        let hub = Hub::new(4);
        hub.join(request("ROOM", "", "One")).unwrap();
        let mut mismatch = request("ROOM", "", "Two");
        mismatch.car = "sc-deckard-conejo".to_string();
        assert!(matches!(hub.join(mismatch), Err(JoinError::AssetMismatch)));
    }

    #[test]
    fn room_capacity_is_enforced() {
        let hub = Hub::new(2);
        hub.join(request("ROOM", "", "One")).unwrap();
        hub.join(request("ROOM", "", "Two")).unwrap();
        assert!(matches!(
            hub.join(request("ROOM", "", "Three")),
            Err(JoinError::RoomFull)
        ));
    }

    #[tokio::test]
    async fn state_is_relayed_and_stale_sequences_are_ignored() {
        let hub = Hub::new(4);
        let first = hub.join(request("ROOM", "", "One")).unwrap();
        let second = hub.join(request("ROOM", "", "Two")).unwrap();
        let mut first_rx = second.receiver;
        let mut second_rx = first.receiver;

        while !matches!(
            first_rx.try_recv().map(|text| parse_message(&text)),
            Ok(ServerMessage::Roster { .. })
        ) {}
        while !matches!(
            second_rx.try_recv().map(|text| parse_message(&text)),
            Ok(ServerMessage::Roster { .. })
        ) {}
        assert!(hub.update_state("ROOM", first.player_id, 1, 10, vec![1.0; 64]));
        assert!(!hub.update_state("ROOM", first.player_id, 1, 11, vec![2.0; 64]));
        let message = loop {
            let text = second_rx.recv().await.unwrap();
            let message = parse_message(&text);
            if matches!(message, ServerMessage::State { .. }) {
                break message;
            }
        };
        match message {
            ServerMessage::State { player_id, seq, .. } => {
                assert_eq!(player_id, first.player_id);
                assert_eq!(seq, 1);
            }
            other => panic!("unexpected message: {other:?}"),
        }
    }

    #[test]
    fn invalid_state_is_rejected() {
        let hub = Hub::new(4);
        let joined = hub.join(request("ROOM", "", "One")).unwrap();
        assert!(!hub.update_state("ROOM", joined.player_id, 1, 0, vec![1.0; 63]));
        assert!(!hub.update_state("ROOM", joined.player_id, 1, 0, vec![f32::NAN; 64]));
    }

    #[test]
    fn leaving_removes_empty_rooms() {
        let hub = Hub::new(4);
        let joined = hub.join(request("ROOM", "", "One")).unwrap();
        hub.leave("ROOM", joined.player_id);
        let again = hub.join(request("ROOM", "new", "Two")).unwrap();
        assert_eq!(again.slot, 0);
    }
}
