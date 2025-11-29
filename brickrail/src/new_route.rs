use bevy::{color::palettes::css::YELLOW, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    block::{InTrack, InTrackOf, LogicalBlock},
    layout::EntityMap,
    layout_primitives::*,
    marker::{Marker, MarkerKey},
    route::RouteMarkerData,
    section::LogicalSection,
    track::{LAYOUT_SCALE, Track},
};

#[derive(Component, Debug)]
struct Train {
    pub id: TrainID,
}

#[derive(Component, Debug)]
#[relationship(relationship_target=RouteAssignedTo)]
struct RouteAssigned(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=RouteAssigned)]
struct RouteAssignedTo(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target=RouteLegAssignedTo)]
struct RouteLegAssigned(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=RouteLegAssigned)]
struct RouteLegAssignedTo(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target=RouteLegs)]
struct RouteLegOf(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=RouteLegOf)]
struct RouteLegs(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target=LegTargetOf)]
struct LegTarget(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=LegTarget)]
struct LegTargetOf(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target=LegStartOf)]
struct LegStart(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=LegStart)]
struct LegStartOf(Vec<Entity>);

#[derive(Component, Debug)]
pub struct NewRoute {
    pub logical_section: LogicalSection,
}

#[derive(Component, Debug)]
struct RouteLeg {
    pub markers: Vec<RouteMarkerData>,
}

// probably on train
#[derive(Component, Debug)]
struct LegPosition {
    pub position: f32,
    pub prev_marker_index: usize,
}

struct RouteState {
    pub current_leg_index: usize,
    pub legs_free: usize,
}

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

#[derive(Component, Debug)]
#[relationship(relationship_target=Wagons)]
struct WagonOf(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=WagonOf)]
struct Wagons(Vec<Entity>);

fn build_route(
    trigger: On<Add, NewRoute>,
    routes: Query<&NewRoute>,
    mut commands: Commands,
    logical_blocks: Query<(Entity, &LogicalBlock, &LogicalSection, &InTrack)>,
    tracks: Query<(&Track, Option<&InTrackOf>, Option<&Marker>)>,
    entity_map: Res<EntityMap>,
) {
    let route_entity = trigger.entity;
    let route = routes.get(route_entity).unwrap();
    let split_tracks = logical_blocks
        .iter()
        .map(|(_, block, _, _)| block.in_track())
        .collect::<Vec<_>>();
    for (critical_path, end_track) in route
        .logical_section
        .split_by_tracks_with_overlap(split_tracks)
    {
        let from_track = critical_path.tracks.first().unwrap().track();
        let to_track = end_track.track();
        let from_track_entity = entity_map.tracks[&from_track];
        let to_track_entity = entity_map.tracks[&to_track];
        let (_, from_in_track_of, _) = tracks.get(from_track_entity).unwrap();
        let (_, to_in_track_of, _) = tracks.get(to_track_entity).unwrap();
        let (from_block_entity, _from_block, from_section, _) = logical_blocks
            .get(*from_in_track_of.unwrap().collection().first().unwrap())
            .unwrap();
        let (to_block_entity, _to_block, to_section, _) = logical_blocks
            .get(*to_in_track_of.unwrap().collection().first().unwrap())
            .unwrap();

        let mut travel_section = LogicalSection::new();
        debug!("critical path: {:?}", critical_path);
        if critical_path.tracks.first().unwrap().facing
            == critical_path.tracks.last().unwrap().facing
        {
            travel_section.extend_merge(&from_section);
            travel_section.extend_merge(&critical_path);
        }
        travel_section.extend_merge(&to_section);
        debug!("travel section: {:?}", travel_section);
        let mut leg_markers = vec![];

        for logical in critical_path.tracks.iter() {
            debug!("  track: {:?}", logical);
            let track_entity = entity_map.tracks[&logical.track()];
            let (_, _, maybe_marker) = tracks.get(track_entity).unwrap();
            if let Some(marker) = maybe_marker {
                debug!("    marker: {:?}", marker);

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

        commands.spawn((
            RouteLeg {
                markers: leg_markers,
            },
            RouteLegOf(route_entity),
            travel_section,
            LegTarget(to_block_entity),
            LegStart(from_block_entity),
        ));
    }
}

fn draw_route(travel_section: Query<&LogicalSection, With<RouteLegOf>>, mut gizmos: Gizmos) {
    for section in travel_section.iter() {
        for connection in section.directed_connection_iter() {
            let from_track = connection.from_track;
            let to_track = connection.to_track;
            let from_pos = from_track.get_center_vec2() * LAYOUT_SCALE;
            let to_pos = to_track.get_center_vec2() * LAYOUT_SCALE;
            gizmos.line_2d(from_pos, to_pos, YELLOW);
        }
    }
}

pub struct NewRoutePlugin;

impl Plugin for NewRoutePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(build_route);
        app.add_systems(Update, draw_route);
    }
}
