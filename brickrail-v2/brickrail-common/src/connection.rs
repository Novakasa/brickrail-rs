use std::marker::PhantomData;

use bevy::prelude::*;
use petgraph::graphmap::UnGraphMap;

use crate::layout_primitives::{TrackConnectionID, TrackID};
use crate::lifecycle::{DespawnElement, ElementId, LayoutElement, LayoutType};

/// Marker type for the connection element kind.
#[derive(Clone, Debug)]
pub struct Connection;

/// Layout data for a connection. Currently empty — the TrackConnectionID
/// already encodes all structural info (including portal vs continuous).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ConnectionData;

impl LayoutElement for Connection {
    type ID = TrackConnectionID;
    type Data = ConnectionData;

    fn build_lifecycle<L: LayoutType>(app: &mut App) {
        app.add_plugins(ConnectionGraphPlugin::<L>::new());
    }
}

/// Undirected structural graph of track connections.
#[derive(Resource)]
pub struct ConnectionGraph<L: LayoutType> {
    pub graph: UnGraphMap<TrackID, TrackConnectionID>,
    _marker: PhantomData<L>,
}

impl<L: LayoutType> Default for ConnectionGraph<L> {
    fn default() -> Self {
        Self {
            graph: UnGraphMap::new(),
            _marker: PhantomData,
        }
    }
}

impl<L: LayoutType> ConnectionGraph<L> {
    /// Returns all connections from a given track.
    pub fn connections_from(&self, track: TrackID) -> Vec<TrackConnectionID> {
        self.graph
            .edges(track)
            .map(|(_, _, conn)| *conn)
            .collect()
    }
}

/// Plugin that maintains the `ConnectionGraph` resource, updating it
/// reactively as connection entities are added or removed.
pub struct ConnectionGraphPlugin<L: LayoutType>(PhantomData<L>);

impl<L: LayoutType> Default for ConnectionGraphPlugin<L> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<L: LayoutType> ConnectionGraphPlugin<L> {
    pub fn new() -> Self {
        Self::default()
    }
}

fn on_connection_added<L: LayoutType>(
    trigger: On<Add, ElementId<Connection>>,
    query: Query<&ElementId<Connection>>,
    mut graph: ResMut<ConnectionGraph<L>>,
) {
    if let Ok(id) = query.get(trigger.event().entity) {
        let conn = id.0;
        graph.graph.add_edge(conn.track_a.track, conn.track_b.track, conn);
    }
}

fn on_connection_removed<L: LayoutType>(
    trigger: On<DespawnElement>,
    query: Query<&ElementId<Connection>>,
    mut graph: ResMut<ConnectionGraph<L>>,
) {
    if let Ok(id) = query.get(trigger.event().entity) {
        let conn = id.0;
        graph.graph.remove_edge(conn.track_a.track, conn.track_b.track);
    }
}

impl<L: LayoutType> Plugin for ConnectionGraphPlugin<L> {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConnectionGraph<L>>();
        app.add_observer(on_connection_added::<L>);
        app.add_observer(on_connection_removed::<L>);
    }
}
