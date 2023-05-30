use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bevy::prelude::*;

use crate::{
    client::Client,
    shared::{
        ConnectionHandle, Enveloppe, GenericParser, MessageBuffer, NetworkEvent, NetworkEvents,
        Router,
    },
};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
#[system_set(base)]
pub struct NetworkStage;

#[derive(Default, Debug)]
pub struct WebSocketClient {}

impl Plugin for WebSocketClient {
    fn build(&self, app: &mut App) {
        let client = Client::new();
        let router = Arc::new(Mutex::new(GenericParser::new()));
        let map = HashMap::<String, Vec<(ConnectionHandle, Enveloppe)>>::new();
        let network_events = Vec::<NetworkEvent>::new();
        app.insert_resource(client)
            .insert_resource(Router(router))
            .insert_resource(MessageBuffer(map))
            .insert_resource(NetworkEvents(network_events))
            .add_event::<NetworkEvent>()
            .configure_set(NetworkStage.before(CoreSet::First))
            .add_systems(
                (consume_messages, super::shared::handle_network_events).in_base_set(NetworkStage),
            );
    }
}

fn consume_messages(
    client: Res<Client>,
    mut hmap: ResMut<MessageBuffer>,
    mut network_events: ResMut<NetworkEvents>,
) {
    if !client.is_running() {
        return;
    }

    while let Some(ev) = client.try_recv() {
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
