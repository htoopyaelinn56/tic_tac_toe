use godot::prelude::*;
use rust_udp_multicast_test::multicast_service;

use std::sync::OnceLock;
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
    fn discover_peers()  {
        spawn(async move {
            let peers = multicast_service::get_peers().await;
            godot_print!("Discovered peer: {}", peers);
        });
    }
}
