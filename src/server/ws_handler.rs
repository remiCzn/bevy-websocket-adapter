use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use log::{trace, debug, warn};
use tokio::{
    runtime::Runtime,
    task::JoinHandle,
    net::{TcpListener, ToSocketAddrs},
};
use crossbeam_channel::{unbounded, Receiver};
use uuid::Uuid;
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};
use thiserror::{Error as TError};

#[derive(Debug, Clone, Default)]
pub struct ConnectionHandle {
    pub uuid: Uuid,
}

impl ConnectionHandle {
    pub fn new() -> ConnectionHandle {
        ConnectionHandle{
            uuid: Uuid::new_v4(),
        }
    }

    pub fn id(&self) -> Uuid {
        self.uuid
    }
}

#[derive(TError, Debug)]
pub enum ServerConfigError {

}

#[derive(TError, Debug)]
pub enum NetworkError {
}

#[derive(Debug)]
pub enum NetworkEvent {
    Connected(ConnectionHandle),
    Disconnected(ConnectionHandle),
    Message(ConnectionHandle, Vec<u8>),
    Error(Option<ConnectionHandle>, anyhow::Error)
}


pub struct Server {
    rt: Arc<Runtime>,
    server_handle: Option<JoinHandle<()>>,
    sessions_events: Arc<Mutex<HashMap<Uuid, Arc<Receiver<NetworkEvent>>>>>,
    sessions_handles: Arc<Mutex<HashMap<Uuid, JoinHandle<()>>>>,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

impl Server {
    pub fn new() -> Server {
        Server{
            rt: Arc::new(tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Could not build tokio runtime")),
            server_handle: None,
            sessions_events: Arc::new(Mutex::new(HashMap::<Uuid, Arc<Receiver<NetworkEvent>>>::new())),
            sessions_handles: Arc::new(Mutex::new(HashMap::<Uuid, JoinHandle<()>>::new()))
        }
    }

    pub fn is_running(&self) -> bool {
        self.server_handle.is_some()
    }

    pub fn listen(&mut self, addr: impl ToSocketAddrs + Send + 'static) -> Result<(), ServerConfigError> {
        self.start_listen_loop(addr)?;
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(conn) = self.server_handle.take() {
            debug!("stopping WS accept loop");
            conn.abort();
        }
        for (k, conn) in self.sessions_handles.lock().unwrap().drain() {
            debug!("aborting session {}", k);
            conn.abort();
        }
    }

    pub fn recv(&mut self) -> Option<NetworkEvent> {
        let mut sel = crossbeam_channel::Select::new();
        let mut ids = Vec::<Uuid>::new();
        let mut receivers = Vec::new();
        {
            let chs = self.sessions_events.lock().unwrap();
            for (i, rx) in chs.iter() {
                ids.push(*i);
                receivers.push(rx.clone());
            }
        }
        if receivers.is_empty() {
            return None;
        }
        for rx in receivers.iter() {
            sel.recv(&**rx);
        }
        let mut r = None;
        while r.is_none() {
            let msg;
            let index = sel.try_ready();
            if index.is_err() {
                return None;
            }
            {
                let chs = self.sessions_events.lock().unwrap();
                let res = chs.get(&ids[index.unwrap()]);
                res?;
                msg = Some(res.unwrap().recv());
            }
            let sess_id = ids[index.unwrap()];
            r = match msg {
                Some(Err(_e)) => {
                    self.sessions_events.lock().unwrap().remove(&sess_id);
                    self.sessions_handles.lock().unwrap().remove(&sess_id);
                    debug!("connection closed for handle {}", sess_id);
                    None
                },
                Some(Ok(m)) => {
                    match m {
                        NetworkEvent::Error(_, _) => {
                            self.sessions_events.lock().unwrap().remove(&sess_id);
                            self.sessions_handles.lock().unwrap().remove(&sess_id);
                            Some(m)
                        },
                        _ => {
                            Some(m)
                        }
                    }
                },
                None => {
                    panic!("none message, this should never happens.")
                }
            };
        }
        r
    }

    fn start_listen_loop(&mut self, addr: impl ToSocketAddrs + Send + 'static) -> Result<(), ServerConfigError> {
        let rt = self.rt.clone();
        let sessions_events = self.sessions_events.clone();
        let sessions_handles = self.sessions_handles.clone();

        let listen_loop = async move {
            let try_socket = TcpListener::bind(addr).await;
            let listener = try_socket.expect("Failed to bind");
            while let Ok((socket, addr)) = listener.accept().await {
                debug!("new connection from {:?}", addr);
                let client_handle = ConnectionHandle::new();
                let handle_id = client_handle.id();
                let (ev_tx, ev_rx) = unbounded();

                let handle = async move {
                        let ws_stream = tokio_tungstenite::accept_async(socket)
                            .await
                            .expect("Error during the websocket handshake occurred");
                        ev_tx.send(NetworkEvent::Connected(client_handle.clone())).expect("failed to send network event");
                        let (_outgoing, incoming) = ws_stream.split();

                        let handle_incoming = incoming.try_for_each(|msg| {
                            match msg {
                                tokio_tungstenite::tungstenite::Message::Binary(bts) => {
                                    ev_tx.send(NetworkEvent::Message(client_handle.clone(), bts)).expect("failed to send network event");
                                },
                                tokio_tungstenite::tungstenite::Message::Close(_) => {
                                    ev_tx.send(NetworkEvent::Disconnected(client_handle.clone())).expect("failed to send network event");
                                },
                                _ => {
                                    warn!("unsupported format for message: {:?}", msg);
                                }
                            }

                            future::ok(())
                        });
                        pin_mut!(handle_incoming);
                        if let Err(e) = handle_incoming.await {
                            ev_tx.send(NetworkEvent::Error(None, e.into())).expect("failed to send network event");
                        }
                };
                let session_handle = rt.spawn(handle);
                sessions_handles.lock().unwrap().insert(handle_id, session_handle);
                sessions_events.lock().unwrap().insert(handle_id, Arc::new(ev_rx));

            }
        };

        trace!("WS server started listening");

        self.server_handle = Some(self.rt.spawn(listen_loop));

        Ok(())
    }

}