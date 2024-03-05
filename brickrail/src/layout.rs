use crate::editor::GenericID;
use crate::layout_primitives::*;
use crate::marker::MarkerKey;
use crate::section::LogicalSection;
use crate::track::LAYOUT_SCALE;
use bevy::utils::HashMap;
use bevy::{prelude::*, utils::HashSet};
use petgraph::graphmap::DiGraphMap;
use serde::{Deserialize, Serialize};
use serde_json_any_key::any_key_map;

#[derive(Resource, Default)]
pub struct TrackLocks {
    pub locked_tracks: HashMap<TrackID, TrainID>,
    pub clean_trains: HashSet<TrainID>,
}

impl TrackLocks {
    pub fn can_lock(&self, train: &TrainID, section: &LogicalSection) -> bool {
        for track in section.tracks.iter() {
            if let Some(locked_train) = self.locked_tracks.get(&track.track()) {
                if locked_train != train {
                    return false;
                }
            }
        }
        return true;
    }

    pub fn can_lock_track(&self, train: &TrainID, track: &TrackID) -> bool {
        if let Some(locked_train) = self.locked_tracks.get(track) {
            if locked_train != train {
                return false;
            }
        }
        return true;
    }

    pub fn mark_clean(&mut self, train: &TrainID) {
        self.clean_trains.insert(train.clone());
    }

    pub fn is_clean(&self, train: &TrainID) -> bool {
        self.clean_trains.contains(train)
    }

    pub fn lock(&mut self, train: &TrainID, section: &LogicalSection) {
        for track in section.tracks.iter() {
            self.locked_tracks.insert(track.track(), *train);
        }
        self.clean_trains = HashSet::new();
    }

    pub fn unlock_all(&mut self, train: &TrainID) {
        self.locked_tracks
            .retain(|_, locked_train| locked_train != train);
        self.clean_trains = HashSet::new();
    }
}

#[derive(Resource, Default)]
pub struct EntityMap {
    pub tracks: HashMap<TrackID, Entity>,
    pub connections: HashMap<TrackConnectionID, Entity>,
    pub connections_outer: HashMap<TrackConnectionID, Entity>,
    pub connections_inner: HashMap<TrackConnectionID, Entity>,
    pub switches: HashMap<DirectedTrackID, Entity>,
    pub markers: HashMap<TrackID, Entity>,
    pub blocks: HashMap<BlockID, Entity>,
    pub trains: HashMap<TrainID, Entity>,
    pub wagons: HashMap<WagonID, Entity>,
    pub hubs: HashMap<HubID, Entity>,
    pub names: HashMap<GenericID, String>,
    pub layout_devices: HashMap<LayoutDeviceID, Entity>,
}

impl EntityMap {
    pub fn get_entity(&self, id: &GenericID) -> Option<Entity> {
        match id {
            GenericID::Track(track_id) => self.tracks.get(track_id).copied(),
            GenericID::Switch(switch_id) => self.switches.get(switch_id).copied(),
            GenericID::Block(block_id) => self.blocks.get(block_id).copied(),
            GenericID::Train(train_id) => self.trains.get(train_id).copied(),
            GenericID::Marker(track_id) => self.markers.get(track_id).copied(),
            GenericID::Hub(hub_id) => self.hubs.get(hub_id).copied(),
            _ => panic!("generic id get entity not implemented for {:?}", id),
        }
    }

    pub fn add_track(&mut self, track: TrackID, entity: Entity) {
        self.tracks.try_insert(track, entity).unwrap();
    }

    pub fn add_switch(&mut self, switch: DirectedTrackID, entity: Entity) {
        self.switches.try_insert(switch, entity).unwrap();
    }

    pub fn add_block(&mut self, block: BlockID, entity: Entity) {
        self.blocks.try_insert(block, entity).unwrap();
    }

    pub fn add_train(&mut self, train: TrainID, entity: Entity) {
        self.trains.try_insert(train, entity).unwrap();
    }

    pub fn add_marker(&mut self, track: TrackID, entity: Entity) {
        // println!("Adding marker {:?} to {:?}", track, entity);
        self.markers.try_insert(track, entity).unwrap();
    }

    pub fn add_hub(&mut self, hub: HubID, entity: Entity) {
        self.hubs.try_insert(hub, entity).unwrap();
    }

    pub fn remove_marker(&mut self, track: TrackID) {
        self.markers.remove(&track);
    }

    pub fn remove_block(&mut self, block: BlockID) {
        self.blocks.remove(&block);
    }

    pub fn add_connection(
        &mut self,
        connection: TrackConnectionID,
        entity: Entity,
        outer_entity: Entity,
        inner_entity: Entity,
    ) {
        self.connections.try_insert(connection, entity).unwrap();
        self.connections_outer
            .try_insert(connection, outer_entity)
            .unwrap();
        self.connections_inner
            .try_insert(connection, inner_entity)
            .unwrap();
    }

    pub fn new_train_id(&self) -> TrainID {
        let mut id = 0;
        while self.trains.contains_key(&TrainID::new(id)) {
            id += 1;
        }
        return TrainID::new(id);
    }

    pub fn new_hub_id(&self, kind: HubType) -> HubID {
        let mut id = 0;
        while self.hubs.contains_key(&HubID::new(id, kind)) {
            id += 1;
        }
        return HubID::new(id, kind);
    }

    pub fn new_layout_device_id(&self, kind: LayoutDeviceType) -> LayoutDeviceID {
        let mut id = 0;
        while self
            .layout_devices
            .contains_key(&LayoutDeviceID::new(id, kind))
        {
            id += 1;
        }
        return LayoutDeviceID::new(id, kind);
    }
}

#[derive(Resource, Default, Serialize, Deserialize, Clone)]
pub struct MarkerMap {
    #[serde(with = "any_key_map")]
    pub in_markers: HashMap<LogicalTrackID, LogicalBlockID>,
    pub enter_markers: HashMap<LogicalTrackID, LogicalBlockID>,
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

    pub fn register_marker(
        &mut self,
        logical_track: LogicalTrackID,
        marker_key: MarkerKey,
        logical_block: LogicalBlockID,
    ) {
        match marker_key {
            MarkerKey::In => {
                self.in_markers.insert(logical_track, logical_block);
            }
            MarkerKey::Enter => {
                self.enter_markers.insert(logical_track, logical_block);
            }
            MarkerKey::None => {}
        }
    }

    pub fn remove_marker(&mut self, track: TrackID) {
        for logical_track in track.logical_tracks() {
            self.in_markers.remove(&logical_track);
            self.enter_markers.remove(&logical_track);
        }
    }

    pub fn remove_block(&mut self, block: BlockID) {
        for logical_block in block.logical_block_ids() {
            self.in_markers.retain(|_, v| v != &logical_block);
            self.enter_markers.retain(|_, v| v != &logical_block);
        }
    }
}

struct ConnectionIterator<'a> {
    current_track: LogicalTrackID,
    continue_at_fork: bool,
    connections: &'a Connections,
}

impl<'a> Iterator for ConnectionIterator<'a> {
    type Item = LogicalTrackID;

    fn next(&mut self) -> Option<Self::Item> {
        let next_tracks = self
            .connections
            .iter_next_tracks(self.current_track)
            .collect::<Vec<LogicalTrackID>>();
        match next_tracks.len() {
            0 => {
                return None;
            }
            1 => {
                self.current_track = next_tracks[0];
                return Some(self.current_track);
            }
            _ => {
                if self.continue_at_fork {
                    self.current_track = next_tracks[0];
                    return Some(self.current_track);
                } else {
                    return None;
                }
            }
        }
    }
}

#[derive(Resource, Default, Serialize, Deserialize, Clone)]
pub struct Connections {
    logical_graph: DiGraphMap<LogicalTrackID, ()>,
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

    pub fn iter_logical_tracks(&self) -> impl Iterator<Item = LogicalTrackID> + '_ {
        self.logical_graph.nodes()
    }

    pub fn iter_tracks(&self) -> impl Iterator<Item = TrackID> + '_ {
        self.logical_graph
            .nodes()
            .map(|logical_track| logical_track.track())
    }

    pub fn iter_logical_connections(&self) -> impl Iterator<Item = LogicalTrackConnectionID> + '_ {
        self.logical_graph
            .all_edges()
            .map(|(from_track, to_track, _)| LogicalTrackConnectionID {
                from_track,
                to_track,
            })
    }

    pub fn iter_connections(&self) -> impl Iterator<Item = TrackConnectionID> + '_ {
        self.iter_logical_connections()
            .map(|logical_connection| logical_connection.to_directed().to_connection())
    }

    pub fn iter_next_tracks(
        &self,
        track: LogicalTrackID,
    ) -> impl Iterator<Item = LogicalTrackID> + '_ {
        self.logical_graph.neighbors(track)
    }

    pub fn iter_from_track<'a>(
        &'a self,
        track: LogicalTrackID,
    ) -> impl Iterator<Item = LogicalTrackID> + '_ {
        ConnectionIterator {
            current_track: track,
            continue_at_fork: false,
            connections: self,
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

    pub fn connect_tracks(&mut self, track_a: &LogicalTrackID, track_b: &LogicalTrackID) {
        self.logical_graph
            .add_edge(track_a.clone(), track_b.clone(), ());
    }

    pub fn disconnect_tracks(&mut self, track_a: &LogicalTrackID, track_b: &LogicalTrackID) {
        self.logical_graph
            .remove_edge(track_a.clone(), track_b.clone());
    }

    pub fn get_directed_connections_from(
        &self,
        track: DirectedTrackID,
    ) -> Vec<DirectedTrackConnectionID> {
        let mut connections = Vec::new();
        for logical in track.logical_tracks() {
            for next in self.iter_next_tracks(logical) {
                let directed_connection =
                    LogicalTrackConnectionID::new(logical, next).to_directed();
                if !connections.contains(&directed_connection) {
                    connections.push(directed_connection);
                }
            }
        }

        connections
    }

    pub fn find_route_section(
        &self,
        start: LogicalBlockID,
        target: LogicalBlockID,
        avoid_locked: Option<(&TrainID, &TrackLocks)>,
        prefer_facing: Option<Facing>,
    ) -> Option<LogicalSection> {
        let start_track = start.default_in_marker_track();
        let target_track = target.default_in_marker_track();
        match petgraph::algo::astar(
            &self.logical_graph,
            start_track,
            |track| track == target_track,
            |(_a, b, _)| {
                let mut cost = 1.0;
                if let Some((train, locks)) = avoid_locked {
                    if !locks.can_lock_track(train, &b.track()) {
                        cost += f32::INFINITY;
                    }
                }
                if let Some(facing) = prefer_facing {
                    if b.facing != facing {
                        cost += 10000.0;
                    }
                }
                return cost;
            },
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
