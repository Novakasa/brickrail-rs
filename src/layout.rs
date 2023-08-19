use crate::layout_primitives::*;
use bevy::prelude::*;
use petgraph::graphmap::DiGraphMap;

#[derive(Resource)]
pub struct Layout {
    // track_graph: UnGraphMap<TrackID, TrackConnection>,
    directed_graph: DiGraphMap<DirectedTrackID, DirectedTrackConnection>,
    pub scale: f32,
}

impl Layout {
    pub fn add_track(&mut self, track: TrackID) {
        if self
            .directed_graph
            .contains_node(track.get_directed(TrackDirection::Forward))
        {
            println!("track {:?} already exists", track);
            return;
        }
        self.directed_graph
            .add_node(track.get_directed(TrackDirection::Forward));
        self.directed_graph
            .add_node(track.get_directed(TrackDirection::Backward));
    }

    pub fn connect_tracks(&mut self, connection: TrackConnection) {
        let connect_a = connection.to_directed(ConnectionDirection::Forward);
        self.directed_graph
            .add_edge(connect_a.from_track, connect_a.to_track, connect_a);
        let connect_b = connection.to_directed(ConnectionDirection::Backward);
        self.directed_graph
            .add_edge(connect_b.from_track, connect_b.to_track, connect_b);
    }
}

fn draw_tracks(mut gizmos: Gizmos, layout: Res<Layout>) {
    let scale = layout.scale;
    for track in layout.directed_graph.nodes() {
        let center_pos = track.get_center_vec2();
        let to_pos = track.to_slot().get_vec2();
        let end_pos = center_pos + 0.7 * (to_pos - center_pos);
        // println!("{:?} {:?}", center_pos, end_pos);
        gizmos.line_2d(center_pos * scale, end_pos * scale, Color::GOLD);
    }
}

fn spawn_tracks(mut layout: ResMut<Layout>) {
    // layout.add_track(TrackID::new(CellID::new(3, 3, 0), Orientation::SW));
    // layout.add_track(TrackID::new(CellID::new(3, 3, 0), Orientation::EW));
    // layout.add_track(TrackID::new(CellID::new(2, 3, 0), Orientation::SE));
}

pub struct LayoutPlugin {}

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Layout {
            directed_graph: DiGraphMap::new(),
            scale: 40.0,
        });
        app.add_systems(Startup, spawn_tracks);
        app.add_systems(Update, draw_tracks);
    }
}
