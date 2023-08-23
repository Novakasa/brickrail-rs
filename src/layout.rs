use crate::layout_primitives::*;
use bevy::{prelude::*, utils::HashMap};
use petgraph::graphmap::DiGraphMap;

#[derive(Resource)]
pub struct Layout {
    directed_graph: DiGraphMap<DirectedTrackID, DirectedTrackConnection>,
    logical_graph: DiGraphMap<LogicalTrackID, ()>,
    markers: HashMap<TrackID, Marker>,
    pub scale: f32,
}

impl Layout {
    pub fn add_track(&mut self, track: TrackID) {
        if self
            .directed_graph
            .contains_node(track.get_directed(TrackDirection::Aligned))
        {
            println!("track {:?} already exists", track);
            return;
        }

        for dirtrack in track.dirtracks() {
            self.directed_graph.add_node(dirtrack);
            for logical_track in dirtrack.logical_tracks() {
                self.logical_graph.add_node(logical_track);
            }
        }
    }

    pub fn connect_tracks(&mut self, connection: TrackConnection) {
        let directed = connection.to_directed(ConnectionDirection::Forward);
        if self
            .directed_graph
            .contains_edge(directed.from_track, directed.to_track)
        {
            println!("Connection already exists");
            return;
        }
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
        for dirtrack in marker.track.dirtracks() {
            for logical_track in dirtrack.logical_tracks() {
                if let Some(logical_marker) = marker
                    .logicals
                    .get(&(dirtrack.direction, logical_track.facing))
                {
                    if logical_marker.key == MarkerKey::In {
                        self.logical_graph
                            .add_edge(logical_track, logical_track.reversed(), ());
                    }
                }
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
            directed_graph: DiGraphMap::new(),
            logical_graph: DiGraphMap::new(),
            markers: HashMap::new(),
            scale: 40.0,
        });
        app.add_systems(Startup, print_sizes);
        app.add_systems(Update, draw_tracks);
    }
}
