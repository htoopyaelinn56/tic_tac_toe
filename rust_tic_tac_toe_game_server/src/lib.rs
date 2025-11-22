use godot::prelude::*;
use rust_udp_multicast_test::multicast_service;

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
    fn start_discovery_service(&self, player_name: GString)  {
        multicast_service::start_service_sync(player_name.to_string());
    }

    #[func] 
    fn discover_peers() -> GString {
        let peers =  multicast_service::get_peers_sync();
        peers.to_godot()
    }
}
