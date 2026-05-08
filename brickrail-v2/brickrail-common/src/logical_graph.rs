use std::marker::PhantomData;

use bevy::prelude::*;
use petgraph::graphmap::DiGraphMap;

use crate::block::Block;
use crate::connection::ConnectionGraph;
use crate::layout_primitives::*;
use crate::lifecycle::{ElementData, LayoutType, Registry};

/// Directed graph of logical tracks (DirectedTrackID + Facing).
/// Used for pathfinding that respects facing constraints.
///
/// Contains two kinds of edges:
/// - **Normal edges**: physical connections, preserving facing (4 per physical connection).
/// - **Flip edges**: at block enter markers, connecting a logical track to its reversed
///   variant (2 per block).
#[derive(Resource)]
pub struct LogicalGraph<L: LayoutType> {
    pub graph: DiGraphMap<LogicalTrackID, ()>,
    _marker: PhantomData<L>,
}

impl<L: LayoutType> Default for LogicalGraph<L> {
    fn default() -> Self {
        Self {
            graph: DiGraphMap::new(),
            _marker: PhantomData,
        }
    }
}

/// Rebuilds the logical graph from the connection graph and block data.
pub fn build_logical_graph<L: LayoutType>(
    conn_graph: &ConnectionGraph<L>,
    block_registry: &Registry<Block, L>,
    block_data: &Query<&ElementData<Block>>,
) -> DiGraphMap<LogicalTrackID, ()> {
    let mut graph = DiGraphMap::new();

    // Normal edges: for each physical connection, add edges for both facings.
    // A connection has track_a and track_b pointing toward each other.
    // Traveling A→B: the train is on track_a's direction, after crossing it's on
    // track_b.opposite() (heading away from A).
    for (a, b, conn) in conn_graph.graph.all_edges() {
        let _ = (a, b); // nodes from petgraph, we use the edge data instead
        for facing in [Facing::Forward, Facing::Backward] {
            // A → B direction
            let from = LogicalTrackID::new(conn.track_a, facing);
            let to = LogicalTrackID::new(conn.track_b.opposite(), facing);
            graph.add_edge(from, to, ());

            // B → A direction
            let from = LogicalTrackID::new(conn.track_b, facing);
            let to = LogicalTrackID::new(conn.track_a.opposite(), facing);
            graph.add_edge(from, to, ());
        }
    }

    // Flip edges: at each block's enter markers.
    // Each block contributes 2 flip edges (one per endpoint marker).
    //
    // For a block section [t0, ..., tn]:
    //   At tn: (tn, Forward) ↔ (tn, Forward).reversed()
    //     — connects (Aligned,Forward) enter to (Against,Backward) enter
    //   At t0: (t0.opposite(), Forward) ↔ (t0.opposite(), Forward).reversed()
    //     — connects (Against,Forward) enter to (Aligned,Backward) enter
    for (_id, &entity) in block_registry.iter() {
        let Ok(data) = block_data.get(entity) else {
            continue;
        };
        let section = &data.section;
        if section.is_empty() {
            continue;
        }

        let t0 = section.first().unwrap();
        let tn = section.last().unwrap();

        // Flip at last track (tn): connects (Aligned,Forward) ↔ (Against,Backward)
        let logical_tn = LogicalTrackID::new(*tn, Facing::Forward);
        graph.add_edge(logical_tn, logical_tn.reversed(), ());
        graph.add_edge(logical_tn.reversed(), logical_tn, ());

        // Flip at first track (t0): connects (Against,Forward) ↔ (Aligned,Backward)
        // When traveling against, t0 is the far end, so the directed track is t0.opposite()
        let logical_t0 = LogicalTrackID::new(t0.opposite(), Facing::Forward);
        graph.add_edge(logical_t0, logical_t0.reversed(), ());
        graph.add_edge(logical_t0.reversed(), logical_t0, ());
    }

    graph
}

/// Plugin that maintains the `LogicalGraph` resource.
pub struct LogicalGraphPlugin<L: LayoutType>(PhantomData<L>);

impl<L: LayoutType> Default for LogicalGraphPlugin<L> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<L: LayoutType> LogicalGraphPlugin<L> {
    pub fn new() -> Self {
        Self::default()
    }
}

/// System that rebuilds the logical graph. Run after layout elements are spawned.
fn rebuild_logical_graph<L: LayoutType>(
    conn_graph: Res<ConnectionGraph<L>>,
    block_registry: Res<Registry<Block, L>>,
    block_data: Query<&ElementData<Block>>,
    mut logical_graph: ResMut<LogicalGraph<L>>,
) {
    logical_graph.graph = build_logical_graph(&conn_graph, &block_registry, &block_data);
}

impl<L: LayoutType> Plugin for LogicalGraphPlugin<L> {
    fn build(&self, app: &mut App) {
        app.init_resource::<LogicalGraph<L>>();
        // Rebuild in Last schedule to ensure ConnectionGraph and blocks are up to date.
        app.add_systems(Last, rebuild_logical_graph::<L>);
    }
}
