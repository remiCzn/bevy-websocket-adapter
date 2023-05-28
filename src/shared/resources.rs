use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use bevy::prelude::Resource;

use super::{ConnectionHandle, Enveloppe, GenericParser, NetworkEvent};

#[derive(Resource)]
pub struct Connections(pub HashMap<String, Vec<(ConnectionHandle, Enveloppe)>>);

#[derive(Resource)]
pub struct NetworkEvents(pub Vec<NetworkEvent>);

#[derive(Resource)]
pub struct Router(pub Arc<Mutex<GenericParser>>);
