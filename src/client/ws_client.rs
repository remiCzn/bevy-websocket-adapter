use std::{borrow::Cow, sync::Arc};

use bevy::{log::warn, prelude::Resource};
use crossbeam_channel::{unbounded, Receiver, Sender, TryRecvError};
use futures::{join, SinkExt, StreamExt};
use serde::Serialize;
use tokio::{runtime::Runtime, task::JoinHandle};
use tokio_tungstenite::connect_async;
use tracing::error;

use crate::shared::{ConnectionHandle, MessageType, NetworkEvent, SendEnveloppe};

#[derive(Resource)]
pub struct Client {
    rt: Arc<Runtime>,
    handle: Option<JoinHandle<()>>,
    rx: Option<Arc<Receiver<NetworkEvent>>>,
    tx: Option<Arc<Sender<tokio_tungstenite::tungstenite::Message>>>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    pub fn is_running(&self) -> bool {
        self.handle.is_some() && self.rx.is_some() && self.tx.is_some()
    }

    pub fn new() -> Client {
        Client {
            rt: Arc::new(
                tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                    .expect("Could not build tokio runtime"),
            ),
            handle: None,
            rx: None,
            tx: None,
        }
    }

    pub fn disconnect(&mut self) {
        if let Some(h) = &self.handle {
            self.send_raw_message(tungstenite::Message::Close(Some(
                tungstenite::protocol::CloseFrame {
                    code: tungstenite::protocol::frame::coding::CloseCode::Normal,
                    reason: Cow::Borrowed(""),
                },
            )));
            h.abort();
        }
        self.handle = None;
    }

    pub fn connect(&mut self, endpoint: String) {
        let (ev_tx, ev_rx) = unbounded();
        let (from_handler_tx, from_handler_rx) = unbounded();

        let event_loop = async move {
            let s = url::Url::parse(&endpoint).expect("failed to parse connect URL");
            let (ws_stream, _) = connect_async(s).await.expect("Failed to connect");
            let (mut write, read) = ws_stream.split();
            ev_tx
                .send(NetworkEvent::Connected(ConnectionHandle {
                    uuid: uuid::Uuid::nil(),
                }))
                .expect("failed to send network event");
            let read_handle = async move {
                read.for_each(|msg| async {
                    match msg {
                        Err(e) => {
                            error!("failed to receive message: {:?}", e);
                        }
                        Ok(tokio_tungstenite::tungstenite::Message::Binary(bts)) => {
                            ev_tx
                                .send(NetworkEvent::Message(
                                    ConnectionHandle {
                                        uuid: uuid::Uuid::nil(),
                                    },
                                    bts,
                                ))
                                .expect("failed to forward network message");
                        }
                        Ok(m) => {
                            warn!("unsupported message: {:?}", m);
                        }
                    }
                })
                .await;
            };

            let write_handle = async move {
                loop {
                    let req = from_handler_rx.try_recv();
                    match req {
                        Err(TryRecvError::Empty) => {
                            // TODO: REPLACE SPINLOCK !
                            continue;
                        }
                        Err(TryRecvError::Disconnected) => {
                            return;
                        }
                        Ok(ev) => {
                            if let Err(e) = write.send(ev).await {
                                warn!("failed to send message to server: {}", e);
                            }
                        }
                    }
                }
            };
            join!(read_handle, write_handle);
        };
        self.handle = Some(self.rt.spawn(event_loop));
        self.rx = Some(Arc::new(ev_rx));
        self.tx = Some(Arc::new(from_handler_tx));
    }

    pub fn try_recv(&self) -> Option<NetworkEvent> {
        if let Some(channel) = &self.rx {
            match channel.try_recv() {
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => None,
                Ok(ev) => Some(ev),
            }
        } else {
            warn!("trying to receive message with an uninitialized client");
            None
        }
    }

    pub fn send_message<T: MessageType + Serialize + Clone>(&self, msg: &T) {
        let sev = SendEnveloppe {
            message_type: T::message_type().to_string(),
            payload: msg.clone(),
        };
        let payload =
            tokio_tungstenite::tungstenite::Message::Binary(serde_json::to_vec(&sev).unwrap());
        self.send_raw_message(payload)
    }

    pub fn send_raw_message(&self, msg: tokio_tungstenite::tungstenite::Message) {
        if let Some(channel) = &self.tx {
            if let Err(e) = channel.send(msg) {
                warn!("failed to forward message, sink: {:?}", e);
            }
        } else {
            warn!("trying to send message with an uninitialized client",);
        }
    }
}
