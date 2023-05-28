use std::collections::HashMap;

use bevy::prelude::Resource;
use thiserror::Error as TError;

use super::{ConnectionHandle, Enveloppe};

#[derive(TError, Debug)]
pub enum NetworkError {}

#[derive(Resource)]
pub struct NetworkEvents(pub Vec<NetworkEvent>);

#[derive(Debug)]
pub enum NetworkEvent {
    Connected(ConnectionHandle),
    Disconnected(ConnectionHandle),
    Message(ConnectionHandle, Vec<u8>),
    Error(Option<ConnectionHandle>, anyhow::Error),
}

#[derive(Resource)]
pub struct Connections(pub HashMap<String, Vec<(ConnectionHandle, Enveloppe)>>);
