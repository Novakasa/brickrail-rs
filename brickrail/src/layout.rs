use crate::crossing::{LevelCrossing, SetCrossingPositionEvent};
use crate::editor::GenericID;
use crate::layout_primitives::*;
use crate::marker::MarkerKey;
use crate::section::LogicalSection;
use crate::switch::{SetSwitchPositionEvent, Switch};
use crate::switch_motor::MotorPosition;
use crate::track::{TrackLogicalFilter, LAYOUT_SCALE};
use bevy::color::palettes::css::{GOLD, GREEN, ORANGE};
use bevy::ecs::query::{QueryData, QueryFilter, WorldQuery};
use bevy::prelude::*;
use bevy::utils::hashbrown::hash_map::OccupiedError;
use bevy::utils::HashMap;
use petgraph::graphmap::{DiGraphMap, UnGraphMap};
use serde::{Deserialize, Serialize};
use serde_json_any_key::any_key_map;

#[derive(Resource, Default, Clone, PartialEq, Eq)]
pub struct TrackLocks {
    pub locked_tracks: HashMap<TrackID, TrainID>,
    pub locked_switch_motors: HashMap<LayoutDeviceID, (TrainID, MotorPosition)>,
}

impl TrackLocks {
    pub fn can_lock(
        &self,
        train: &TrainID,
        section: &LogicalSection,
        switches: &Query<&Switch>,
        entity_map: &EntityMap,
    ) -> bool {
        for track in section.tracks.iter() {
            if !self.can_lock_track(train, &track.track()) {
                return false;
            }
        }
        for connection in section.connection_iter() {
            if !self.can_lock_connection(train, &connection, switches, entity_map) {
                return false;
            }
        }
        return true;
    }

    pub fn can_lock_track(&self, train: &TrainID, track: &TrackID) -> bool {
        for colliding_track in track.colliding_tracks() {
            if let Some(locked_train) = self.locked_tracks.get(&colliding_track) {
                if locked_train != train {
                    return false;
                }
            }
        }
        return true;
    }

    pub fn can_lock_connection(
        &self,
        _train: &TrainID,
        connection: &LogicalTrackConnectionID,
        switches: &Query<&Switch>,
        entity_map: &EntityMap,
    ) -> bool {
        let directed_connection = connection.to_directed();
        if let Some(switch) = entity_map
            .switches
            .get(&directed_connection.from_track)
            .and_then(|e| switches.get(*e).ok())
        {
            let position = directed_connection.to_track.get_switch_position();
            for (id_option, pos) in switch.iter_motor_positions(&position) {
                if let Some(id) = id_option {
                    if let Some((_, locked_pos)) = self.locked_switch_motors.get(id) {
                        if locked_pos != &pos {
                            return false;
                        }
                    }
                }
            }
        }
        return true;
    }

    pub fn lock(
        &mut self,
        train: &TrainID,
        section: &LogicalSection,
        entity_map: &EntityMap,
        switches: &Query<&Switch>,
        crossings: &Query<&LevelCrossing>,
        set_switch_position: &mut EventWriter<SetSwitchPositionEvent>,
        set_crossing_position: &mut EventWriter<SetCrossingPositionEvent>,
    ) {
        for track in section.tracks.iter() {
            if let Some(locked_train) = self.locked_tracks.get(&track.track()) {
                if locked_train != train {
                    panic!("Track {:?} is already locked by {:?}", track, locked_train);
                }
            }
            self.locked_tracks.insert(track.track(), *train);
        }

        for directed_connection in section.directed_connection_iter() {
            if let Some(entity) = entity_map.switches.get(&directed_connection.from_track) {
                let position = directed_connection.to_track.get_switch_position();
                let switch = switches.get(*entity).unwrap();
                for (id_option, pos) in switch.iter_motor_positions(&position) {
                    if let Some(id) = id_option {
                        match self
                            .locked_switch_motors
                            .try_insert(id.clone(), (*train, pos.clone()))
                        {
                            Ok(_) => {}
                            Err(OccupiedError {
                                entry: _entry,
                                value: (_locked_train, locked_pos),
                            }) => {
                                assert_eq!(locked_pos, pos);
                            }
                        }
                    }
                }
                set_switch_position.send(SetSwitchPositionEvent {
                    id: directed_connection.from_track,
                    position,
                });
            }
            if let Some(entity) = entity_map
                .crossings
                .get(&directed_connection.from_track.track)
            {}
        }
    }

    pub fn unlock_all(&mut self, train: &TrainID) {
        self.locked_tracks
            .retain(|_, locked_train| locked_train != train);
        self.locked_switch_motors
            .retain(|_, (locked_train, _)| locked_train != train);
    }
}

#[derive(Resource, Default)]
pub struct EntityMap {
    pub tracks: HashMap<TrackID, Entity>,
    pub connections_outer: HashMap<DirectedTrackConnectionID, Entity>,
    pub connections_inner: HashMap<DirectedTrackConnectionID, Entity>,
    pub connections_path: HashMap<DirectedTrackConnectionID, Entity>,
    pub switches: HashMap<DirectedTrackID, Entity>,
    pub markers: HashMap<TrackID, Entity>,
    pub blocks: HashMap<BlockID, Entity>,
    pub trains: HashMap<TrainID, Entity>,
    pub wagons: HashMap<WagonID, Entity>,
    pub hubs: HashMap<HubID, Entity>,
    pub layout_devices: HashMap<LayoutDeviceID, Entity>,
    pub destinations: HashMap<DestinationID, Entity>,
    pub schedules: HashMap<ScheduleID, Entity>,
    pub crossings: HashMap<TrackID, Entity>,
}

impl EntityMap {
    pub fn iter_all_entities(&self) -> impl Iterator<Item = &Entity> + '_ {
        self.tracks
            .values()
            .chain(self.switches.values())
            .chain(self.blocks.values())
            .chain(self.trains.values())
            .chain(self.markers.values())
            .chain(self.hubs.values())
            .chain(self.layout_devices.values())
            .chain(self.connections_outer.values())
            .chain(self.connections_inner.values())
            .chain(self.connections_path.values())
            .chain(self.wagons.values())
            .chain(self.destinations.values())
            .chain(self.schedules.values())
            .chain(self.crossings.values())
    }

    pub fn get_entity(&self, id: &GenericID) -> Option<Entity> {
        match id {
            GenericID::Track(track_id) => self.tracks.get(track_id).copied(),
            GenericID::Switch(switch_id) => self.switches.get(switch_id).copied(),
            GenericID::Block(block_id) => self.blocks.get(block_id).copied(),
            GenericID::Train(train_id) => self.trains.get(train_id).copied(),
            GenericID::Marker(track_id) => self.markers.get(track_id).copied(),
            GenericID::Hub(hub_id) => self.hubs.get(hub_id).copied(),
            GenericID::Destination(dest_id) => self.destinations.get(dest_id).copied(),
            GenericID::Schedule(schedule_id) => self.schedules.get(schedule_id).copied(),
            GenericID::Crossing(track_id) => self.crossings.get(track_id).copied(),
            _ => panic!("generic id get entity not implemented for {:?}", id),
        }
    }

    pub fn query_get<'a, D: QueryData, F: QueryFilter>(
        &'a self,
        query: &'a Query<D, F>,
        id: &GenericID,
    ) -> Option<<<D as QueryData>::ReadOnly as WorldQuery>::Item<'a>> {
        let entity = self.get_entity(id)?;
        query.get(entity).ok()
    }

    pub fn query_get_mut<'a, D: QueryData, F: QueryFilter>(
        &'a self,
        query: &'a mut Query<D, F>,
        id: &GenericID,
    ) -> Option<<D as WorldQuery>::Item<'a>> {
        let entity = self.get_entity(id)?;
        query.get_mut(entity).ok()
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

    pub fn add_wagon(&mut self, wagon: WagonID, entity: Entity) {
        self.wagons.try_insert(wagon, entity).unwrap();
    }

    pub fn add_marker(&mut self, track: TrackID, entity: Entity) {
        // println!("Adding marker {:?} to {:?}", track, entity);
        self.markers.try_insert(track, entity).unwrap();
    }

    pub fn add_hub(&mut self, hub: HubID, entity: Entity) {
        self.hubs.try_insert(hub, entity).unwrap();
    }

    pub fn add_destination(&mut self, dest: DestinationID, entity: Entity) {
        self.destinations.try_insert(dest, entity).unwrap();
    }

    pub fn add_schedule(&mut self, schedule: ScheduleID, entity: Entity) {
        self.schedules.try_insert(schedule, entity).unwrap();
    }

    pub fn add_crossing(&mut self, crossing: TrackID, entity: Entity) {
        self.crossings.try_insert(crossing, entity).unwrap();
    }

    pub fn remove_track(&mut self, track: TrackID) {
        self.tracks.remove(&track);
    }

    pub fn remove_connection(&mut self, connection: DirectedTrackConnectionID) {
        self.connections_outer.remove(&connection);
        self.connections_inner.remove(&connection);
        self.connections_path.remove(&connection);
    }

    pub fn remove_marker(&mut self, track: TrackID) {
        self.markers.remove(&track);
    }

    pub fn remove_block(&mut self, block: BlockID) {
        self.blocks.remove(&block);
    }

    pub fn remove_switch(&mut self, switch: DirectedTrackID) {
        self.switches.remove(&switch);
    }

    pub fn remove_train(&mut self, train: TrainID) {
        self.trains.remove(&train);
    }

    pub fn remove_hub(&mut self, hub: HubID) {
        self.hubs.remove(&hub);
    }

    pub fn remove_layout_device(&mut self, device: LayoutDeviceID) {
        self.layout_devices.remove(&device);
    }

    pub fn remove_crossing(&mut self, crossing: TrackID) {
        self.crossings.remove(&crossing);
    }

    pub fn add_connection(
        &mut self,
        connection: DirectedTrackConnectionID,
        outer_entity: Entity,
        inner_entity: Entity,
        path_entity: Entity,
    ) {
        self.connections_outer
            .try_insert(connection, outer_entity)
            .unwrap();
        self.connections_inner
            .try_insert(connection, inner_entity)
            .unwrap();
        self.connections_path
            .try_insert(connection, path_entity)
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

    pub fn new_destination_id(&self) -> DestinationID {
        let mut id = 0;
        while self.destinations.contains_key(&DestinationID::Specific(id)) {
            id += 1;
        }
        return DestinationID::Specific(id);
    }

    pub fn new_schedule_id(&self) -> ScheduleID {
        let mut id = 0;
        while self.schedules.contains_key(&ScheduleID::new(id)) {
            id += 1;
        }
        return ScheduleID::new(id);
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
    pub logical_graph: DiGraphMap<LogicalTrackID, ()>,
    pub connection_graph: UnGraphMap<TrackID, TrackConnectionID>,
}

impl Connections {
    pub fn has_track(&self, track: TrackID) -> bool {
        self.connection_graph.contains_node(track)
    }

    pub fn get_unconnected_dirtrack(&self, track: TrackID) -> Option<DirectedTrackID> {
        let mut unconnected = track.dirtracks().to_vec();
        for (_, _, connection) in self.connection_graph.edges(track) {
            let dirtrack = if connection.track_a().track == track {
                connection.track_a
            } else {
                connection.track_b
            };
            unconnected.retain(|dir| *dir != dirtrack);
        }
        if unconnected.len() == 1 {
            return Some(unconnected[0]);
        }
        return None;
    }

    pub fn add_filtered_track(&mut self, track: TrackID, logical_filter: &TrackLogicalFilter) {
        self.connection_graph.add_node(track);
        for dirtrack in track.dirtracks() {
            for logical_track in dirtrack.logical_tracks() {
                if !self.logical_graph.contains_node(logical_track) {
                    self.logical_graph.add_node(logical_track);
                }
            }
        }

        let connections = self
            .connection_graph
            .edges(track)
            .map(|(_, _, c)| c)
            .cloned()
            .collect::<Vec<_>>();

        // this just makes sure all possible logical connections are present
        // the ones not matching the filter will be removed below
        for connection in connections {
            println!("Reconnecting {:?}", connection);
            self.connect_tracks_simple(&connection);
        }

        for dirtrack in track.dirtracks() {
            for logical_track in dirtrack.logical_tracks() {
                if !logical_filter
                    .filters
                    .get(&logical_track.discriminator())
                    .copied()
                    .unwrap_or(true)
                {
                    // this should automatically remove logical connections as well
                    println!("Removing track {:?}", logical_track);
                    self.logical_graph.remove_node(logical_track);
                }
            }
        }
    }

    pub fn remove_track(&mut self, track: TrackID) {
        self.connection_graph.remove_node(track);
        for dirtrack in track.dirtracks() {
            for logical_track in dirtrack.logical_tracks() {
                self.logical_graph.remove_node(logical_track);
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
    ) -> impl Iterator<Item = LogicalTrackID> + 'a {
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
        println!("Connecting {:?}", connection);
        assert!(self
            .connection_graph
            .contains_node(connection.track_a().track));
        assert!(self
            .connection_graph
            .contains_node(connection.track_b().track));
        self.connection_graph.add_edge(
            connection.track_a().track,
            connection.track_b().track,
            connection.clone(),
        );
        for logical in connection.logical_connections() {
            if self.logical_graph.contains_node(logical.from_track)
                && self.logical_graph.contains_node(logical.to_track)
            {
                if !self
                    .logical_graph
                    .contains_edge(logical.from_track, logical.to_track)
                {
                    self.logical_graph
                        .add_edge(logical.from_track, logical.to_track, ());
                }
            }
        }
    }

    pub fn connect_tracks(&mut self, track_a: &LogicalTrackID, track_b: &LogicalTrackID) {
        assert!(
            self.logical_graph.contains_node(track_a.clone())
                && self.logical_graph.contains_node(track_b.clone())
        );
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

    pub fn dijkstra(
        &self,
        start: LogicalBlockID,
        targets: &[LogicalBlockID],
        avoid_locked: Option<(&TrainID, &TrackLocks, &Query<&Switch>, &EntityMap)>,
        prefer_facing: Option<Facing>,
    ) -> HashMap<LogicalBlockID, f32> {
        let start_node = start.default_in_marker_track();
        let result =
            petgraph::algo::dijkstra(&self.logical_graph, start_node, None, |(a, b, _)| {
                edge_cost(a, b, avoid_locked, prefer_facing)
            });
        let target_nodes = targets
            .iter()
            .map(|target| (target.default_in_marker_track(), target))
            .collect::<HashMap<_, _>>();
        let mut filtered_result = HashMap::new();
        for (track, cost) in result.iter() {
            if let Some(block) = target_nodes.get(track) {
                filtered_result.insert(**block, *cost);
            }
        }
        filtered_result
    }

    pub fn find_route_section(
        &self,
        start: LogicalBlockID,
        target: LogicalBlockID,
        avoid_locked: Option<(&TrainID, &TrackLocks, &Query<&Switch>, &EntityMap)>,
        prefer_facing: Option<Facing>,
    ) -> Option<LogicalSection> {
        let start_track = start.default_in_marker_track();
        let target_track = target.default_in_marker_track();
        match petgraph::algo::astar(
            &self.logical_graph,
            start_track,
            |track| track == target_track,
            |(a, b, _)| edge_cost(a, b, avoid_locked, prefer_facing),
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

fn edge_cost(
    a: LogicalTrackID,
    b: LogicalTrackID,
    avoid_locked: Option<(&TrainID, &TrackLocks, &Query<&Switch>, &EntityMap)>,
    prefer_facing: Option<Facing>,
) -> f32 {
    let mut cost = 1.0;
    if let Some((train, locks, switches, entity_map)) = avoid_locked {
        if !locks.can_lock_track(train, &b.track())
            || !locks.can_lock_connection(
                train,
                &LogicalTrackConnectionID::new(a, b),
                switches,
                entity_map,
            )
        {
            cost += f32::INFINITY;
        }
    }
    if let Some(facing) = prefer_facing {
        if b.facing != facing {
            cost += 10000.0;
        }
    }
    cost
}

fn draw_layout_graph(mut gizmos: Gizmos, connections: Res<Connections>, time: Res<Time>) {
    let dist = time.elapsed_secs() % 1.0;
    for track in connections.logical_graph.nodes() {
        track
            .dirtrack
            .draw_with_gizmos(&mut gizmos, LAYOUT_SCALE, Color::from(GOLD));
    }

    for (from_track, to_track, _) in connections.logical_graph.all_edges() {
        let connection = LogicalTrackConnectionID {
            from_track,
            to_track,
        }
        .to_directed();
        connection.draw_with_gizmos(&mut gizmos, LAYOUT_SCALE, Color::from(GOLD));
        let pos = connection.interpolate_pos(dist * connection.connection_length());
        gizmos.circle_2d(pos * LAYOUT_SCALE, 0.05 * LAYOUT_SCALE, Color::from(GREEN));
        let pos_unnormalized = connection.interpolate_pos(dist);
        gizmos.circle_2d(
            pos_unnormalized * LAYOUT_SCALE,
            0.05 * LAYOUT_SCALE,
            Color::from(ORANGE),
        );
    }
}

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(EntityMap::default());
        app.insert_resource(TrackLocks::default());
        app.insert_resource(Connections::default());
        app.insert_resource(MarkerMap::default());
        // app.add_systems(Update, draw_layout_graph);
    }
}
