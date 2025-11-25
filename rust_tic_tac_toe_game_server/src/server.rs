use axum::extract::ws::{Message, WebSocket};
use axum::extract::{Path, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Router};
use futures_util::{SinkExt, StreamExt};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

// A simple identifier for each WebSocket connection.
type ConnectionId = u64;

pub struct Room {
    // Map of connection id -> sender channel for that connection
    pub connections: HashMap<ConnectionId, mpsc::UnboundedSender<String>>,
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
        .route("/join/{room_id}", get(join_room))
        .with_state(state);
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
}

pub async fn join_room(
    Path(room_id): Path<String>,
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_join_room(room_id, socket, state))
}

// New helper: centralize dead-connection cleanup to avoid duplication and optionally announce a message
async fn cleanup_dead_connections(
    state: SharedState,
    room_id: &str,
    dead_connections: Vec<ConnectionId>,
    announce: Option<String>,
    exclude: Option<Vec<ConnectionId>>,
) {
    // If there's nothing to remove and nothing to announce, nothing to do.
    if dead_connections.is_empty() && announce.is_none() {
        return;
    }

    // Prepare set of ids to remove (start with provided dead_connections)
    let mut to_remove: Vec<ConnectionId> = dead_connections.into_iter().collect();

    // Snapshot current senders for this room under the lock, then release the lock
    let mut senders: Vec<(ConnectionId, mpsc::UnboundedSender<String>)> = Vec::new();
    {
        let app_state = state.lock().await;
        if let Some(room) = app_state.rooms.get(room_id) {
            for (&cid, tx) in &room.connections {
                senders.push((cid, tx.clone()));
            }
        }
    }

    // Apply exclusion if provided (e.g., don't send the leave announce to the leaving connection)
    if let Some(exclude_ids) = exclude {
        let exclude_set: HashSet<_> = exclude_ids.into_iter().collect();
        senders.retain(|(cid, _)| !exclude_set.contains(cid));
    }

    // If an announcement is requested, send it to the snapshot of senders (after exclusion)
    if let Some(msg) = announce {
        for (cid, tx) in &senders {
            if tx.send(msg.clone()).is_err() {
                to_remove.push(*cid);
            }
        }
    }

    // Remove any connections discovered dead (either initially provided or discovered while announcing)
    if !to_remove.is_empty() {
        let mut app_state = state.lock().await;
        if let Some(room) = app_state.rooms.get_mut(room_id) {
            for cid in to_remove {
                room.connections.remove(&cid);
            }
            if room.connections.is_empty() {
                app_state.rooms.remove(room_id);
            }
        }
    }
}

async fn handle_join_room(room_id: String, socket: WebSocket, state: SharedState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Register this connection in the shared room state and get its connection id.
    let connection_id = {
        let mut app_state = state.lock().await;
        let id = app_state.next_connection_id;
        app_state.next_connection_id = app_state.next_connection_id.wrapping_add(1);

        let room = app_state
            .rooms
            .entry(room_id.clone())
            .or_insert_with(Room::new);

        room.connections.insert(id, tx.clone());

        id
    };

    // Broadcast a join message to everyone in the room.
    {
        let mut dead_connections = Vec::new();
        let mut senders = Vec::new();

        {
            let app_state = state.lock().await;
            if let Some(room) = app_state.rooms.get(&room_id) {
                for (&cid, tx_conn) in &room.connections {
                    senders.push((cid, tx_conn.clone()));
                }
            }
        }

        for (cid, tx_conn) in &senders {
            if tx_conn
                .send("Someone joined the room".to_string())
                .is_err()
            {
                dead_connections.push(*cid);
            }
        }

        if !dead_connections.is_empty() {
            // Use centralized helper (no announce, no exclude)
            cleanup_dead_connections(state.clone(), &room_id, dead_connections, None, None).await;
        }
    }

    let state_for_recv = state.clone();
    let room_id_for_recv = room_id.clone();

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender
                .send(Message::Text(msg.into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if msg.is_err() {
                break;
            }

            match msg {
                Ok(msg_result) => {
                    if let Message::Text(text) = msg_result {
                        // Broadcast this message to all connections in the same room
                        let mut dead_connections = Vec::new();
                        let mut senders = Vec::new();

                        {
                            let app_state = state_for_recv.lock().await;
                            if let Some(room) = app_state.rooms.get(&room_id_for_recv) {
                                for (&cid, tx_conn) in &room.connections {
                                    senders.push((cid, tx_conn.clone()));
                                }
                            }
                        }

                        for (cid, tx_conn) in &senders {
                            if tx_conn.send(text.clone().to_string()).is_err() {
                                dead_connections.push(*cid);
                            }
                        }

                        if !dead_connections.is_empty() {
                            // Use centralized helper (no announce, no exclude)
                            cleanup_dead_connections(state_for_recv.clone(), &room_id_for_recv, dead_connections, None, None).await;
                        }
                    }
                }
                Err(_) => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    // On disconnect, announce to the remaining connections and then remove this connection from the room
    // Announce "Someone left the room" to everyone except the leaving connection.
    cleanup_dead_connections(
        state.clone(),
        &room_id,
        Vec::new(),
        Some("Someone left the room".to_string()),
        Some(vec![connection_id]),
    ).await;

    let mut app_state = state.lock().await;
    if let Some(room) = app_state.rooms.get_mut(&room_id) {
        room.connections.remove(&connection_id);
        if room.connections.is_empty() {
            app_state.rooms.remove(&room_id);
        }
    }
}
