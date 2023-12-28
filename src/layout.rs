use crate::editor::GenericID;
use crate::layout_primitives::*;
use crate::marker::MarkerKey;
use crate::section::LogicalSection;
use crate::track::LAYOUT_SCALE;
use bevy::prelude::*;
use bevy::utils::HashMap;
use petgraph::graphmap::DiGraphMap;
use serde::{Deserialize, Serialize};

#[derive(Resource, Default)]
pub struct EntityMap {
    pub tracks: HashMap<TrackID, Entity>,
    pub markers: HashMap<TrackID, Entity>,
    pub blocks: HashMap<BlockID, Entity>,
    pub trains: HashMap<TrainID, Entity>,
    pub wagons: HashMap<WagonID, Entity>,
}

#[derive(Resource, Default, Serialize, Deserialize)]
pub struct MarkerMap {
    pub in_markers: HashMap<LogicalTrackID, LogicalBlockID>,
    pub enter_markers: HashMap<LogicalTrackID, LogicalBlockID>,
}

#[derive(Resource, Default, Serialize, Deserialize)]
pub struct Connections {
    logical_graph: DiGraphMap<LogicalTrackID, ()>,
}

#[derive(Resource, Default)]
struct TrackLocks {
    locked_tracks: HashMap<TrackID, TrainID>,
}

impl EntityMap {
    pub fn get_entity(&self, id: &GenericID) -> Option<Entity> {
        match id {
            GenericID::Track(track_id) => self.tracks.get(track_id).copied(),
            GenericID::Block(block_id) => self.blocks.get(block_id).copied(),
            GenericID::Train(train_id) => self.trains.get(train_id).copied(),
            _ => None,
        }
    }

    pub fn add_track(&mut self, track: TrackID, entity: Entity) {
        self.tracks.try_insert(track, entity).unwrap();
    }

    pub fn add_block(&mut self, block: BlockID, entity: Entity) {
        self.blocks.try_insert(block, entity).unwrap();
    }

    pub fn add_train(&mut self, train: TrainID, entity: Entity) {
        self.trains.try_insert(train, entity).unwrap();
    }

    pub fn add_marker(&mut self, track: TrackID, entity: Entity) {
        self.markers.try_insert(track, entity).unwrap();
    }
}

impl MarkerMap {
    pub fn get_marker_key(
        &self,
        logical_track: &LogicalTrackID,
        target_block: &LogicalBlockID,
    ) -> MarkerKey {
        if self.in_markers.get(logical_track) == Some(target_block) {
            MarkerKey::In
        } else if self.enter_markers.get(logical_track) == Some(target_block) {
            MarkerKey::Enter
        } else {
            MarkerKey::None
        }
    }
}

impl Connections {
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

    pub fn find_route_section(
        &self,
        start: LogicalBlockID,
        target: LogicalBlockID,
    ) -> Option<LogicalSection> {
        let start_track = start.default_in_marker_track();
        let target_track = target.default_in_marker_track();
        match petgraph::algo::astar(
            &self.logical_graph,
            start_track,
            |track| track == target_track,
            |_| 1.0,
            |track| {
                let delta = track.cell().get_delta_vec(&target_track.cell());
                delta.x.abs() + delta.y.abs()
            },
        ) {
            Some((_, path)) => Some(LogicalSection { tracks: path }),
            None => None,
        }
    }
}

fn draw_layout_graph(mut gizmos: Gizmos, connections: Res<Connections>, time: Res<Time>) {
    let dist = time.elapsed_seconds() % 1.0;
    for track in connections.logical_graph.nodes() {
        track
            .dirtrack
            .draw_with_gizmos(&mut gizmos, LAYOUT_SCALE, Color::GOLD);
    }

    for (from_track, to_track, _) in connections.logical_graph.all_edges() {
        let connection = LogicalTrackConnectionID {
            from_track,
            to_track,
        }
        .to_directed();
        connection.draw_with_gizmos(&mut gizmos, LAYOUT_SCALE, Color::GOLD);
        let pos = connection.interpolate_pos(dist * connection.connection_length());
        gizmos.circle_2d(pos * LAYOUT_SCALE, 0.05 * LAYOUT_SCALE, Color::GREEN);
    }
}

fn print_sizes() {
    println!("{:?}", std::mem::size_of::<CellID>());
    println!("{:?}", std::mem::size_of::<TrackID>());
    println!("{:?}", std::mem::size_of::<DirectedTrackID>());
    println!("{:?}", std::mem::size_of::<DirectedTrackConnectionID>());
}

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EntityMap::default());
        app.insert_resource(TrackLocks::default());
        app.insert_resource(Connections::default());
        app.insert_resource(MarkerMap::default());
        app.add_systems(Startup, print_sizes);
        // app.add_systems(Update, draw_layout_graph);
    }
}
