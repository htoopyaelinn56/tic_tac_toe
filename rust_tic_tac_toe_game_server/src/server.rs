use axum::routing::get;
use axum::Router;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use crate::room;

// A simple identifier for each WebSocket connection.
pub(crate) type ConnectionId = u64;

pub struct Room {
    // Map of connection id -> (sender channel for that connection, assigned PlayerMark)
    pub connections: HashMap<ConnectionId, (mpsc::UnboundedSender<String>, crate::room::PlayerMark)>,
}

impl Room {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
        }
    }
}

pub struct AppState {
    pub rooms: HashMap<String, Room>,
    // Counter to assign unique connection ids
    pub next_connection_id: ConnectionId,
}

pub type SharedState = Arc<Mutex<AppState>>;

pub async fn start_server() {
    let state: SharedState = Arc::new(Mutex::new(AppState {
        rooms: HashMap::new(),
        next_connection_id: 0,
    }));

    let app = Router::new()
        .route("/join/{room_id}", get(room::join_room))
        .with_state(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}