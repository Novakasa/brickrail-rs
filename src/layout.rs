use std::cell::Cell;

use crate::layout_primitives::*;
use bevy::prelude::*;
use petgraph::graphmap::{DiGraphMap, UnGraphMap};

#[derive(Resource)]
pub struct Layout {
    // track_graph: UnGraphMap<TrackID, TrackConnection>,
    directed_graph: DiGraphMap<DirectedTrackID, DirectedTrackConnection>,
}

impl Layout {
    fn add_track(&mut self, track: TrackID) {
        self.directed_graph
            .add_node(track.get_directed(TrackDirection::Forward));
        self.directed_graph
            .add_node(track.get_directed(TrackDirection::Backward));
    }
}

fn draw_tracks(mut gizmos: Gizmos, layout: Res<Layout>) {
    let scale = 50.0;
    for track in layout.directed_graph.nodes() {
        gizmos.line_2d(
            track.from_slot().get_vec2() * scale,
            track.to_slot().get_vec2() * scale,
            Color::GOLD,
        );
    }
}

fn spawn_tracks(mut layout: ResMut<Layout>) {
    layout.add_track(TrackID::new(CellID::new(3, 3, 0), Orientation::SW));
    layout.add_track(TrackID::new(CellID::new(3, 3, 0), Orientation::EW));
    layout.add_track(TrackID::new(CellID::new(2, 3, 0), Orientation::SE));
}

pub struct LayoutPlugin {}

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Layout {
            directed_graph: DiGraphMap::new(),
        });
        app.add_systems(Startup, spawn_tracks);
        app.add_systems(Update, draw_tracks);
    }
}
