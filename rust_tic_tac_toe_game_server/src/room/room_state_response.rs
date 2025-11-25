use serde::Serialize;

#[derive(Serialize)]
pub struct RoomStateResponse{
    pub room_id: String,
    pub num_connections: usize,
    pub message: String,
}