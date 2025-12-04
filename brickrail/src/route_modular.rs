use bevy::{
    color::palettes::{css::YELLOW, tailwind::LIME_100},
    prelude::*,
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{InTrack, InTrackOf, LogicalBlock, LogicalBlockSection},
    editor::GenericID,
    layout::EntityMap,
    layout_primitives::Facing,
    marker::{Marker, MarkerKey, Markers},
    route::RouteMarkerData,
    section::LogicalSection,
    track::LAYOUT_SCALE,
    train::MarkerAdvanceMessage,
};

#[derive(Component, Debug)]
#[relationship(relationship_target=RouteAssignedTo)]
pub struct AssignedRoute(pub Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=AssignedRoute)]
pub struct RouteAssignedTo(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target=RouteLegAssignedTo)]
pub struct AssignedRouteLeg(pub Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=AssignedRouteLeg)]
pub struct RouteLegAssignedTo(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target=RouteLegs)]
pub struct RouteLegOf(pub Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=RouteLegOf)]
pub struct RouteLegs(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target=LegTargetOf)]
pub struct LegTarget(pub Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=LegTarget)]
pub struct LegTargetOf(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target=LegStartOf)]
pub struct LegStart(pub Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=LegStart)]
pub struct LegStartOf(Vec<Entity>);

#[derive(Component, Debug)]
pub struct ModularRoute {
    pub logical_section: LogicalSection,
}

#[derive(Component, Debug)]
struct RouteLegMarkers {
    pub markers: Vec<RouteMarkerData>,
}

impl RouteLegMarkers {
    pub fn get_speed(&self, index: usize) -> TrainSpeed {
        self.markers.get(index).unwrap().speed
    }

    pub fn has_entered(&self, index: usize) -> bool {
        println!(
            "Checking has_entered: index {}, len {}",
            index,
            self.markers.len()
        );
        index >= self.markers.len().saturating_sub(2)
    }

    pub fn has_exited(&self, index: usize) -> bool {
        index >= 1
    }

    pub fn has_completed(&self, index: usize) -> bool {
        index >= self.markers.len().saturating_sub(1)
    }
}

// probably on train
#[derive(Component, Debug, Default)]
pub struct LegPosition {
    pub position: f32,
    pub prev_marker_index: usize,
}

#[derive(Component, Debug)]
struct RouteState {
    pub current_leg_index: usize,
    pub prev_marker_index: usize,
    pub legs_free: usize,
}

impl RouteState {
    pub fn can_pass(&self, num_legs: usize) -> bool {
        self.current_leg_index < num_legs - 1
    }
}

#[derive(Component, Debug)]
pub struct ModularRouteLeg {
    pub section: LogicalSection,
}
impl ModularRouteLeg {
    pub fn is_turn(&self) -> bool {
        let first_facing = self.section.tracks.first().unwrap().facing;
        let last_facing = self.section.tracks.last().unwrap().facing;
        first_facing != last_facing
    }
}

#[derive(Component, Debug)]
pub struct RouteLegTravelSection {
    pub section: LogicalSection,
}

fn build_route(
    trigger: On<Add, ModularRoute>,
    routes: Query<&ModularRoute>,
    logical_blocks: Query<&LogicalBlock>,
    mut commands: Commands,
) {
    println!("Building modular route...");
    let route_entity = trigger.entity;
    let route = routes.get(route_entity).unwrap();
    let split_tracks = logical_blocks
        .iter()
        .map(|block| block.in_track())
        .collect::<Vec<_>>();
    for (critical_path, _end_track) in route
        .logical_section
        .split_by_tracks_with_overlap(split_tracks)
    {
        commands.spawn((
            ModularRouteLeg {
                section: critical_path,
            },
            RouteLegOf(route_entity),
        ));
    }
}

fn build_route_leg(
    trigger: On<Add, ModularRouteLeg>,
    critical_paths: Query<&ModularRouteLeg>,
    logical_blocks: Query<(Entity, &LogicalBlock, &LogicalBlockSection, &InTrack)>,
    mut commands: Commands,
    tracks: Query<(Option<&InTrackOf>, Option<&Markers>)>,
    entity_map: Res<EntityMap>,
    markers_query: Query<&Marker>,
) {
    println!("Building modular route leg...");
    let critical_path = &critical_paths.get(trigger.entity).unwrap().section;
    let from_track = critical_path.tracks.first().unwrap();
    let to_track = critical_path.tracks.last().unwrap();
    let from_track_entity = entity_map.tracks[&from_track.track()];
    let to_track_entity = entity_map.tracks[&to_track.track()];
    let from_in_track_of = tracks.get(from_track_entity).unwrap().0.unwrap();
    let to_in_track_of = tracks.get(to_track_entity).unwrap().0.unwrap();
    assert!(from_in_track_of.collection().len() == 2);
    assert!(to_in_track_of.collection().len() == 2);
    let (from_block_entity, from_block, from_section, _) = from_in_track_of
        .collection()
        .iter()
        .find_map(|block_entity| {
            let components = logical_blocks.get(*block_entity).unwrap();
            if &components.1.to_logical_id().default_in_marker_track() == from_track {
                Some(components)
            } else {
                None
            }
        })
        .unwrap();
    let (to_block_entity, to_block, to_section, _) = to_in_track_of
        .collection()
        .iter()
        .find_map(|block_entity| {
            let components = logical_blocks.get(*block_entity).unwrap();
            if &components.1.to_logical_id().default_in_marker_track() == to_track {
                Some(components)
            } else {
                None
            }
        })
        .unwrap();

    println!("from block: {:?}", from_block);
    println!("to block: {:?}", to_block);

    let mut travel_section = LogicalSection::new();
    println!("critical path: {:?}", critical_path);
    assert!(
        &to_block.to_logical_id().default_in_marker_track() == critical_path.tracks.last().unwrap()
    );
    assert!(
        &from_block.to_logical_id().default_in_marker_track()
            == critical_path.tracks.first().unwrap()
    );

    if critical_path.tracks.first().unwrap().facing == critical_path.tracks.last().unwrap().facing {
        travel_section.extend_merge(&from_section.section);
        travel_section.extend_merge(&critical_path);
    }
    travel_section.extend_merge(&to_section.section);
    println!("travel section: {:?}", travel_section);
    println!();
    assert!(travel_section.is_connected());
    let mut leg_markers = vec![];

    for logical in critical_path.tracks.iter() {
        println!("  track: {:?}", logical);
        let track_entity = entity_map.tracks[&logical.track()];
        let (_, maybe_marker) = tracks.get(track_entity).unwrap();
        if let Some(markers) = maybe_marker {
            let marker = markers_query
                .get(*markers.collection().first().unwrap())
                .unwrap();
            println!("    marker: {:?}", marker);
            let position = travel_section
                .length_to(&logical)
                .unwrap_or_else(|_| travel_section.length_to(&logical.reversed()).unwrap());

            let route_marker = RouteMarkerData {
                track: logical.clone(),
                color: marker.color,
                speed: marker.logical_data.get(logical).unwrap().speed,
                key: MarkerKey::None,
                position: position,
            };
            leg_markers.push(route_marker);
        }
    }

    commands.entity(trigger.entity).insert((
        RouteLegMarkers {
            markers: leg_markers,
        },
        RouteLegTravelSection {
            section: travel_section,
        },
        LegTarget(to_block_entity),
        LegStart(from_block_entity),
    ));
}

fn assign_first_route_leg(
    query: Query<(Entity, &AssignedRoute), Without<AssignedRouteLeg>>,
    routes: Query<&RouteLegs>,
    mut commands: Commands,
) {
    for (entity, route_assigned) in query.iter() {
        println!("Assigning route legs to route...");
        let route_entity = route_assigned.0;
        let route_legs = routes.get(route_entity).unwrap();
        // if the route legs aren't there  yet, will be assigned in build route
        commands
            .entity(entity)
            .insert(AssignedRouteLeg(route_legs.collection()[0]));
        commands.entity(entity).insert((RouteState {
            current_leg_index: 0,
            prev_marker_index: 0,
            legs_free: 0,
        },));
    }
}

fn on_route_assigned(trigger: On<Insert, AssignedRoute>, mut commands: Commands) {
    println!("Route assigned to train: {:?}", trigger.entity);
    commands.entity(trigger.entity).remove::<AssignedRouteLeg>();
}

fn on_route_leg_assigned(
    trigger: On<Insert, AssignedRouteLeg>,
    old_pos: Query<&LegPosition>,
    mut commands: Commands,
) {
    println!("Assigning LegPosition to route leg...");
    let train_entity = trigger.entity;
    if let Ok(old_leg_pos) = old_pos.get(train_entity) {
        println!(
            "LegPosition already exists on entity {:?}: {:?}",
            train_entity, old_leg_pos
        );
    }
    commands.entity(train_entity).insert((
        LegPosition {
            position: 0.0,
            prev_marker_index: 0,
        },
        OutdatedState,
    ));
}

#[derive(Component, Debug)]
struct OutdatedState;

fn update_train_state(
    mut trains: Query<
        (
            Entity,
            &AssignedRoute,
            &AssignedRouteLeg,
            &mut TrainState,
            &RouteState,
        ),
        With<OutdatedState>,
    >,
    routes: Query<&RouteLegs>,
    legs: Query<(&ModularRouteLeg, &RouteLegMarkers)>,
    mut commands: Commands,
) {
    for (train_entity, assigned_route, assigned_leg, mut train_state, route_state) in
        trains.iter_mut()
    {
        commands.entity(train_entity).remove::<OutdatedState>();
        let (current_leg, leg_markers) = legs.get(assigned_leg.0).unwrap();
        let route_legs = routes.get(assigned_route.0).unwrap();
        let will_stop = !route_state.can_pass(route_legs.len());
        println!(
            "prev_marker_index: {}, has_completed: {}, will_stop: {}, num_legs: {}, num_markers: {}, leg entity: {:?}",
            route_state.prev_marker_index,
            leg_markers.has_completed(route_state.prev_marker_index),
            will_stop,
            route_legs.len(),
            leg_markers.markers.len(),
            assigned_leg.0
        );
        println!("markers: {:?}", leg_markers.markers);
        if leg_markers.has_completed(route_state.prev_marker_index) && will_stop {
            *train_state = TrainState::Stop;
            println!(
                "Train {:?} has completed its route leg and will stop.",
                train_entity
            );
            return;
        };
        let next_leg_entity = route_legs
            .collection()
            .get(route_state.current_leg_index + 1);
        let will_turn = next_leg_entity
            .map(|next_leg_entity| legs.get(*next_leg_entity).unwrap().0.is_turn())
            .unwrap_or(false);

        let mut speed = leg_markers.get_speed(route_state.prev_marker_index);
        if (will_turn || will_stop) && leg_markers.has_entered(route_state.prev_marker_index) {
            speed = TrainSpeed::Slow;
        }
        let facing = current_leg.section.tracks.last().unwrap().facing;
        *train_state = TrainState::Run { speed, facing };
        println!(
            "Train {:?} updated state to {:?}.",
            train_entity, train_state
        );
    }
}

fn draw_route(
    travel_section: Query<&RouteLegTravelSection, With<RouteLegAssignedTo>>,
    mut gizmos: Gizmos,
) {
    for section in travel_section.iter() {
        for connection in section.section.directed_connection_iter() {
            let from_track = connection.from_track;
            let to_track = connection.to_track;
            let from_pos = from_track.get_center_vec2() * LAYOUT_SCALE;
            let to_pos = to_track.get_center_vec2() * LAYOUT_SCALE;
            gizmos.line_2d(from_pos, to_pos, YELLOW);
        }
    }
}

fn move_trains(
    mut trains: Query<(&mut LegPosition, &TrainState, &AssignedRouteLeg)>,
    legs: Query<&ModularRouteLeg>,
    time: Res<Time>,
) {
    for (mut position, state, assigned_leg) in trains.iter_mut() {
        // println!(
        //     "Moving train at position {:?} with state {:?}",
        //     position,
        //     state.get_speed(),
        // );
        let leg = legs.get(assigned_leg.0).unwrap();
        let leg_facing = leg.section.tracks.last().unwrap().facing;
        position.position += state.get_speed() * time.delta_secs() * leg_facing.get_sign();
    }
}

fn advance_route_leg(
    leg_markers: Query<&RouteLegMarkers>,
    route_legs: Query<&RouteLegs>,
    mut trains: Query<(Entity, &mut RouteState, &AssignedRouteLeg, &AssignedRoute)>,
    mut commands: Commands,
) {
    for (train_entity, mut route_state, assigned_leg, assigned_route) in trains.iter_mut() {
        let leg_markers = leg_markers.get(assigned_leg.0).unwrap();
        let route_legs = route_legs.get(assigned_route.0).unwrap();

        if leg_markers.has_completed(route_state.prev_marker_index)
            && route_state.can_pass(route_legs.len())
        {
            route_state.current_leg_index += 1;
            route_state.prev_marker_index = 0;
            println!(
                "Advanced to route leg index {}",
                route_state.current_leg_index
            );
            println!("leg entities: {:?}", route_legs.collection());
            println!(
                "Assigning new leg {:?} to train {:?}",
                route_legs.collection()[route_state.current_leg_index],
                train_entity
            );
            commands.entity(train_entity).insert((AssignedRouteLeg(
                route_legs.collection()[route_state.current_leg_index],
            ),));
        }
    }
}

fn advance_marker_index(
    leg_markers: Query<&RouteLegMarkers>,
    mut trains: Query<(
        Entity,
        &mut RouteState,
        &AssignedRouteLeg,
        &LegPosition,
        &GenericID,
    )>,
    mut marker_message_writer: MessageWriter<MarkerAdvanceMessage>,
    mut commands: Commands,
) {
    for (train_entity, mut route_state, assigned_leg, leg_position, generic_id) in trains.iter_mut()
    {
        let GenericID::Train(train_id) = generic_id else {
            panic!("Expected GenericID::Train")
        };
        let leg_markers = leg_markers.get(assigned_leg.0).unwrap();
        if leg_markers.has_completed(route_state.prev_marker_index) {
            continue;
        };
        let next_index = route_state.prev_marker_index + 1;

        let Some(next_marker) = leg_markers.markers.get(next_index) else {
            panic!("No next marker found")
        };
        if leg_position.position >= next_marker.position {
            route_state.prev_marker_index = next_index;
            println!(
                "Train advanced to marker index {}",
                route_state.prev_marker_index
            );
            marker_message_writer.write(MarkerAdvanceMessage {
                id: *train_id,
                index: next_index,
            });
            commands.entity(train_entity).insert(OutdatedState);
        }
    }
}

#[derive(Component, Debug)]
pub struct ModularTrain;

#[derive(Debug, Default, Clone, Component)]
pub enum TrainState {
    #[default]
    Stop,
    Run {
        facing: Facing,
        speed: TrainSpeed,
    },
}

impl TrainState {
    pub fn get_speed(&self) -> f32 {
        match self {
            TrainState::Stop => 0.0,
            TrainState::Run { speed, facing } => facing.get_sign() * speed.get_speed(),
        }
    }
}

#[derive(
    Clone,
    Copy,
    Hash,
    PartialEq,
    PartialOrd,
    Ord,
    Eq,
    Debug,
    Default,
    Serialize,
    Deserialize,
    Reflect,
    Component,
)]
pub enum TrainSpeed {
    Slow,
    #[default]
    Cruise,
    Fast,
}

impl TrainSpeed {
    pub fn get_speed(&self) -> f32 {
        match self {
            TrainSpeed::Slow => 2.0,
            TrainSpeed::Cruise => 4.0,
            TrainSpeed::Fast => 8.0,
        }
    }

    pub fn as_train_u8(&self) -> u8 {
        match self {
            TrainSpeed::Slow => 2,
            TrainSpeed::Cruise => 3,
            TrainSpeed::Fast => 1,
        }
    }
}

fn debug_draw_train(
    train_query: Query<(&AssignedRouteLeg, &LegPosition)>,
    legs: Query<&RouteLegTravelSection>,
    mut gizmos: Gizmos,
) {
    for (leg_assigned, leg_position) in train_query.iter() {
        let leg_entity = leg_assigned.0;
        if let Ok(leg_section) = legs.get(leg_entity) {
            let pos = leg_section.section.interpolate_pos(leg_position.position) * LAYOUT_SCALE;
            gizmos.circle_2d(pos, 10.0, LIME_100);
        }
    }
}

#[derive(Component, Debug)]
#[relationship_target(relationship=ProxyTrainOf)]
pub struct ProxyTrains(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target=ProxyTrains)]
pub struct ProxyTrainOf(pub Entity);

#[derive(Component, Debug)]
#[relationship(relationship_target=Wagons)]
struct WagonOf(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=WagonOf)]
struct Wagons(Vec<Entity>);

pub struct ModularRoutePlugin;

impl Plugin for ModularRoutePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(build_route);
        app.add_observer(build_route_leg);
        app.add_observer(on_route_leg_assigned);
        app.add_observer(on_route_assigned);
        app.add_systems(
            Update,
            (
                move_trains,
                draw_route,
                assign_first_route_leg,
                debug_draw_train,
                (advance_route_leg, advance_marker_index, update_train_state).chain(),
            ),
        );
    }
}
