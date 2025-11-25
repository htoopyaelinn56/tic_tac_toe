use serde::Serialize;
use serde_json::Value;

#[derive(Serialize, Clone, Copy, Debug)]
pub enum PlayerMark {
    X,
    O,
}

impl PlayerMark {
    pub fn to_string(&self) -> String {
        match self {
            PlayerMark::X => "x".to_string(),
            PlayerMark::O => "o".to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct RoomStateResponse {
    pub room_id: String,
    pub num_connections: usize,
    pub message: String,
    pub success: bool,
    pub my_mark: String,
}

impl RoomStateResponse {
    pub fn to_json_value(&self) -> Value {
        serde_json::to_value(self).unwrap()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseType {
    RoomState,
}

#[derive(Serialize)]
pub struct RoomResponse {
    pub response_type: ResponseType,
    pub response: Value,
}
