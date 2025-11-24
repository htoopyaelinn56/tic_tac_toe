use godot::prelude::*;
use rust_udp_multicast_test::multicast_service;

use std::sync::{mpsc, OnceLock};
use tokio::runtime::Runtime;

static TOKIO_RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn get_runtime() -> &'static Runtime {
    TOKIO_RUNTIME.get_or_init(|| Runtime::new().expect("Failed to create Tokio runtime"))
}

pub fn spawn<F>(future: F)
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    get_runtime().spawn(future);
}

struct MyExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MyExtension {}

#[derive(GodotClass)]
#[class(base=Node)]
struct RustNode {
    base: Base<Node>,
}

#[godot_api]
impl INode for RustNode {
    fn init(base: Base<Node>) -> Self {
        RustNode { base }
    }
}

#[godot_api]
impl RustNode {
    // This function will be callable from Godot
    #[func]
    fn hello_world(&self) {
        godot_print!("Hello from Rust!");
    }

    #[func]
    fn start_discovery_service(&self, player_name: GString) {
        spawn(async move {
            multicast_service::start_service(player_name.to_string()).await;
        })
    }

    #[func]
    fn discover_peers(&self) -> GString {
        let (tx, rx) = mpsc::channel();
        spawn(async move {
            let peers = multicast_service::get_peers().await;
            let _ = tx.send(peers);
        });
        let peers = rx.recv().unwrap_or_default();
        peers.to_godot()
    }

    #[func]
    fn stop_discovery_service(&self) {
        spawn(async move { multicast_service::stop_service().await })
    }

    #[func]
    fn get_local_ip(&self) -> GString {
        let local_ip = multicast_service::get_local_ipv4_in_string();
        local_ip.to_godot()
    }
}
