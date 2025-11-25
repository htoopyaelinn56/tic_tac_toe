use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug)]
pub enum PlayerMark {
    X,
    O,
}

#[derive(Serialize)]
pub struct RoomStateResponse{
    pub room_id: String,
    pub num_connections: usize,
    pub message: String,
    pub success: bool,
    pub my_mark: PlayerMark,
}