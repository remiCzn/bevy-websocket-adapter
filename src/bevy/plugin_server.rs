use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bevy::prelude::*;

use crate::{
    server::Server,
    shared::{
        ConnectionHandle, Connections, Enveloppe, GenericParser, NetworkEvent, NetworkEvents,
        Router,
    },
};

#[derive(Default, Debug)]
pub struct WebSocketServer {}

impl Plugin for WebSocketServer {
    fn build(&self, app: &mut App) {
        let server = Server::new();
        let router = Arc::new(Mutex::new(GenericParser::new()));
        let map = HashMap::<String, Vec<(ConnectionHandle, Enveloppe)>>::new();
        let network_events = Vec::<NetworkEvent>::new();
        app.insert_resource(server)
            .insert_resource(Router(router))
            .insert_resource(Connections(map))
            .insert_resource(NetworkEvents(network_events))
            .add_event::<NetworkEvent>()
            .add_stage_before(CoreStage::First, "network", SystemStage::single_threaded())
            .add_system_to_stage("network", consume_messages)
            .add_system_to_stage("network", super::shared::handle_network_events);
    }
}

fn consume_messages(
    server: Res<Server>,
    mut hmap: ResMut<Connections>,
    mut network_events: ResMut<NetworkEvents>,
) {
    if !server.is_running() {
        return;
    }

    while let Some(ev) = server.recv() {
        match ev {
            NetworkEvent::Message(handle, raw_ev) => {
                trace!("consuming message from {:?}", handle);
                if let Ok(enveloppe) = serde_json::from_reader::<std::io::Cursor<Vec<u8>>, Enveloppe>(
                    std::io::Cursor::new(raw_ev),
                ) {
                    let tp = enveloppe.message_type.to_string();
                    let mut v = if let Some(x) = hmap.0.remove(&tp) {
                        x
                    } else {
                        Vec::new()
                    };
                    v.push((handle, enveloppe.clone()));
                    hmap.0.insert(tp, v);
                } else {
                    warn!("failed to deserialize message from {:?}", handle);
                    continue;
                }
            }
            other => {
                trace!("received network event: {:?}", other);
                network_events.0.push(other);
            }
        }
    }
}
