use crate::layout_primitives::*;
use crate::marker::*;
use bevy::utils::HashSet;
use bevy::{prelude::*, utils::HashMap};
use petgraph::graphmap::DiGraphMap;

#[derive(Resource, Default)]
pub struct Layout {
    directed_graph: DiGraphMap<DirectedTrackID, DirectedTrackConnection>,
    logical_graph: DiGraphMap<LogicalTrackID, ()>,
    markers: HashMap<TrackID, Marker>,
    blocks: HashSet<BlockID>,
    pub scale: f32,
}

impl Layout {
    pub fn add_track(&mut self, track: TrackID) {
        for dirtrack in track.dirtracks() {
            self.directed_graph.add_node(dirtrack);
            for logical_track in dirtrack.logical_tracks() {
                self.logical_graph.add_node(logical_track);
            }
        }
    }

    pub fn connect_tracks(&mut self, connection: TrackConnection) {
        for directed in connection.directed_connections() {
            self.directed_graph
                .add_edge(directed.from_track, directed.to_track, directed);
            for facing in [Facing::Forward, Facing::Backward] {
                self.logical_graph.add_edge(
                    directed.from_track.get_logical(facing),
                    directed.to_track.get_logical(facing),
                    (),
                );
            }
        }
    }

    pub fn add_marker(&mut self, marker: Marker) {
        for logical_track in marker.track.logical_tracks() {
            let logical_marker = marker.collapse_logical(logical_track).unwrap();
            if let MarkerKey::In(_) = logical_marker.key {
                self.logical_graph
                    .add_edge(logical_track, logical_track.reversed(), ());
            }
        }
        self.markers.insert(marker.track, marker);
    }
}

fn draw_tracks(mut gizmos: Gizmos, layout: Res<Layout>, time: Res<Time>) {
    let scale = layout.scale;

    let dist = time.elapsed_seconds() % 1.0;
    for track in layout.directed_graph.nodes() {
        track.draw_with_gizmos(&mut gizmos, scale, Color::GOLD);
    }

    for (_, _, connection) in layout.directed_graph.all_edges() {
        if connection.from_track < connection.to_track {
            continue;
        }
        connection.draw_with_gizmos(&mut gizmos, scale, Color::GOLD);
        let pos = connection.interpolate_pos(dist * connection.connection_length());
        gizmos.circle_2d(pos * scale, 0.05 * scale, Color::GREEN);
    }
}

fn print_sizes() {
    println!("{:?}", std::mem::size_of::<CellID>());
    println!("{:?}", std::mem::size_of::<TrackID>());
    println!("{:?}", std::mem::size_of::<DirectedTrackID>());
    println!("{:?}", std::mem::size_of::<DirectedTrackConnection>());
}

pub struct LayoutPlugin {}

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Layout {
            scale: 40.0,
            ..Default::default()
        });
        app.add_systems(Startup, print_sizes);
        app.add_systems(Update, draw_tracks);
    }
}
