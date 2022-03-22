extern crate bevy_websocket_adapter;
use ::bevy::prelude::*;
use bevy_websocket_adapter::{bevy::WebSocketServer, server::Server, shared::NetworkEvent};

fn start_listen(mut ws: ResMut<Server>) {
    ws.listen("0.0.0.0:12345")
        .expect("failed to start websocket server");
}

fn listen_for_events(mut evs: EventReader<NetworkEvent>) {
    for ev in evs.iter() {
        info!("received NetworkEvent : {:?}", ev);
    }
}

fn main() {
    App::new()
        .insert_resource(bevy::log::LogSettings {
            level: bevy::log::Level::DEBUG,
            ..Default::default()
        })
        .add_plugin(bevy::log::LogPlugin)
        .add_plugins(MinimalPlugins)
        .add_plugin(WebSocketServer::default())
        .add_startup_system(start_listen)
        .add_system(listen_for_events)
        .run();
}
