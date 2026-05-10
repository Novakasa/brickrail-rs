use bevy::prelude::*;
use petgraph::graphmap::UnGraphMap;

use crate::layout_primitives::{TrackConnectionID, TrackID};
use crate::lifecycle::{DespawnElement, ElementId, LayoutElement};

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

    fn build_lifecycle(app: &mut App) {
        app.add_plugins(ConnectionGraphPlugin);
    }
}

/// Undirected structural graph of track connections.
#[derive(Resource, Default)]
pub struct ConnectionGraph {
    pub graph: UnGraphMap<TrackID, TrackConnectionID>,
}

impl ConnectionGraph {
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
pub struct ConnectionGraphPlugin;

fn on_connection_added(
    trigger: On<Add, ElementId<Connection>>,
    query: Query<&ElementId<Connection>>,
    mut graph: ResMut<ConnectionGraph>,
) {
    if let Ok(id) = query.get(trigger.event().entity) {
        let conn = id.0;
        graph.graph.add_edge(conn.track_a.track, conn.track_b.track, conn);
    }
}

fn on_connection_removed(
    trigger: On<DespawnElement>,
    query: Query<&ElementId<Connection>>,
    mut graph: ResMut<ConnectionGraph>,
) {
    if let Ok(id) = query.get(trigger.event().entity) {
        let conn = id.0;
        graph.graph.remove_edge(conn.track_a.track, conn.track_b.track);
    }
}

impl Plugin for ConnectionGraphPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConnectionGraph>();
        app.add_observer(on_connection_added);
        app.add_observer(on_connection_removed);
    }
}
