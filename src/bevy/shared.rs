use bevy::prelude::*;

use crate::shared::{
    ConnectionHandle, Connections, GenericParser, MessageType, NetworkEvent, NetworkEvents, Router,
};

pub(crate) fn handle_network_events(
    mut events: ResMut<NetworkEvents>,
    mut sink: EventWriter<NetworkEvent>,
) {
    for ev in events.0.drain(..) {
        sink.send(ev);
    }
}

pub(crate) fn add_message_consumer<T>(
    key: String,
) -> impl FnMut(ResMut<Connections>, Res<Router>, EventWriter<(ConnectionHandle, T)>)
where
    T: Send + Sync + 'static,
{
    move |mut hmap: ResMut<Connections>,
          router: Res<Router>,
          mut queue: EventWriter<(ConnectionHandle, T)>| {
        if let Some(values) = hmap.0.remove(&*key) {
            for (handle, v) in values {
                let enveloppe = router.0.lock().unwrap().parse_enveloppe(&v);
                match enveloppe {
                    Ok(dat) => match GenericParser::try_into_concrete_type::<T>(dat) {
                        Ok(msg) => {
                            queue.send((handle, msg));
                        }
                        Err(e) => {
                            warn!("failed to downcast : {}", e);
                        }
                    },
                    Err(e) => {
                        warn!("failed to parse type enveloppe : {}", e);
                        continue;
                    }
                };
            }
        }
    }
}

pub trait WsMessageInserter {
    #[deprecated(
        since = "0.1.4",
        note = "Use [`add_message_type`](#method.add_message_type) instead."
    )]
    fn register_message_type<T>(&mut self) -> &mut Self
    where
        T: MessageType + 'static,
    {
        self.add_message_type::<T>()
    }
    fn add_message_type<T>(&mut self) -> &mut Self
    where
        T: MessageType + 'static;
}

impl WsMessageInserter for App {
    fn add_message_type<T>(&mut self) -> &mut Self
    where
        T: MessageType + 'static,
    {
        self.add_event::<(ConnectionHandle, T)>();
        let router = self
            .world
            .get_resource::<Router>()
            .expect("cannot register message before WebSocketServer initialization")
            .0;
        router.lock().unwrap().insert_type::<T>();

        self.add_system(add_message_consumer::<T>(T::message_type().to_string()));
        self
    }
}
