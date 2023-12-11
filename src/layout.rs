use crate::layout_primitives::*;
use bevy::prelude::*;
use bevy::utils::HashMap;
use petgraph::graphmap::DiGraphMap;

#[derive(Resource, Default)]
pub struct Layout {
    logical_graph: DiGraphMap<LogicalTrackID, ()>,
    markers: HashMap<TrackID, Entity>,
    blocks: HashMap<BlockID, Entity>,
    logical_blocks: HashMap<LogicalBlockID, Entity>,
    trains: HashMap<TrainID, Entity>,
    pub scale: f32,
}

impl Layout {
    pub fn has_track(&self, track: TrackID) -> bool {
        for logical_track in track.logical_tracks() {
            if self.logical_graph.contains_node(logical_track) {
                return true;
            }
        }
        return false;
    }

    pub fn add_track(&mut self, track: TrackID) {
        for dirtrack in track.dirtracks() {
            for logical_track in dirtrack.logical_tracks() {
                self.logical_graph.add_node(logical_track);
            }
        }
    }

    pub fn has_connection(&self, connection: &TrackConnectionID) -> bool {
        for logical in connection.logical_connections() {
            if self
                .logical_graph
                .contains_edge(logical.from_track, logical.to_track)
            {
                return true;
            }
        }
        return false;
    }

    pub fn has_directed_connection(&self, connection: &DirectedTrackConnectionID) -> bool {
        for facing in [Facing::Forward, Facing::Backward].iter() {
            if self.has_logical_connection(&connection.to_logical(*facing)) {
                return true;
            }
        }
        return false;
    }

    pub fn has_logical_connection(&self, connection: &LogicalTrackConnectionID) -> bool {
        self.logical_graph
            .contains_edge(connection.from_track, connection.to_track)
    }

    pub fn connect_tracks_simple(&mut self, connection: &TrackConnectionID) {
        for logical in connection.logical_connections() {
            self.logical_graph
                .add_edge(logical.from_track, logical.to_track, ());
        }
    }

    pub fn add_marker(&mut self, track: TrackID, entity: Entity) {
        self.markers.insert(track, entity);
    }
}

fn draw_layout_graph(mut gizmos: Gizmos, layout: Res<Layout>, time: Res<Time>) {
    let scale = layout.scale;

    let dist = time.elapsed_seconds() % 1.0;
    for track in layout.logical_graph.nodes() {
        track
            .dirtrack
            .draw_with_gizmos(&mut gizmos, scale, Color::GOLD);
    }

    for (from_track, to_track, _) in layout.logical_graph.all_edges() {
        let connection = LogicalTrackConnectionID {
            from_track,
            to_track,
        }
        .to_directed();
        connection.draw_with_gizmos(&mut gizmos, scale, Color::GOLD);
        let pos = connection.interpolate_pos(dist * connection.connection_length());
        gizmos.circle_2d(pos * scale, 0.05 * scale, Color::GREEN);
    }
}

fn print_sizes() {
    println!("{:?}", std::mem::size_of::<CellID>());
    println!("{:?}", std::mem::size_of::<TrackID>());
    println!("{:?}", std::mem::size_of::<DirectedTrackID>());
    println!("{:?}", std::mem::size_of::<DirectedTrackConnectionID>());
}

pub struct LayoutPlugin {}

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Layout {
            scale: 40.0,
            ..Default::default()
        });
        app.add_systems(Startup, print_sizes);
        // app.add_systems(Update, draw_layout_graph);
    }
}
