extern crate bevy_websocket_adapter;
use ::bevy::prelude::*;
use bevy::{diagnostic::DiagnosticsPlugin, log::LogPlugin};
use bevy_websocket_adapter::{
    bevy::{WebSocketServer, WsMessageInserter},
    impl_message_type,
    server::Server,
    shared::ConnectionHandle,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DummyEvent {
    a: u32,
}
impl_message_type!(DummyEvent, "dummy");

fn start_listen(mut ws: ResMut<Server>) {
    ws.listen("0.0.0.0:9023")
        .expect("failed to start websocket server");
}

fn listen_for_dummy(mut evs: EventReader<(ConnectionHandle, DummyEvent)>) {
    for (handle, ev) in evs.iter() {
        info!("received DummyEvent from {:?} : {:?}", handle, ev);
    }
}

fn main() {
    App::new()
        .add_plugin(LogPlugin {
            level: bevy::log::Level::DEBUG,
            ..Default::default()
        })
        .add_plugin(DiagnosticsPlugin)
        .add_plugins(MinimalPlugins)
        .add_plugin(WebSocketServer::default())
        .add_startup_system(start_listen)
        .add_message_type::<DummyEvent>()
        .add_system(listen_for_dummy)
        .run();
}
