use bevy::{color::palettes::css::YELLOW, prelude::*};

use crate::{
    block::{InTrack, InTrackOf, LogicalBlock, LogicalBlockSection},
    layout::EntityMap,
    marker::{Marker, MarkerKey},
    route::RouteMarkerData,
    section::LogicalSection,
    track::LAYOUT_SCALE,
};

#[derive(Component, Debug)]
#[relationship(relationship_target=RouteAssignedTo)]
pub struct RouteAssigned(pub Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=RouteAssigned)]
pub struct RouteAssignedTo(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target=RouteLegAssignedTo)]
pub struct RouteLegAssigned(pub Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=RouteLegAssigned)]
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

#[derive(Component, Debug)]
pub struct ModularRouteLeg {
    pub section: LogicalSection,
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
    tracks: Query<(Option<&InTrackOf>, Option<&Marker>)>,
    entity_map: Res<EntityMap>,
) {
    let critical_path = &critical_paths.get(trigger.entity).unwrap().section;
    let from_track = critical_path.tracks.first().unwrap().track();
    let to_track = critical_path.tracks.last().unwrap().track();
    let from_track_entity = entity_map.tracks[&from_track];
    let to_track_entity = entity_map.tracks[&to_track];
    let (from_in_track_of, _) = tracks.get(from_track_entity).unwrap();
    let (to_in_track_of, _) = tracks.get(to_track_entity).unwrap();
    let (from_block_entity, _from_block, from_section, _) = logical_blocks
        .get(*from_in_track_of.unwrap().collection().first().unwrap())
        .unwrap();
    let (to_block_entity, _to_block, to_section, _) = logical_blocks
        .get(*to_in_track_of.unwrap().collection().first().unwrap())
        .unwrap();

    let mut travel_section = LogicalSection::new();
    debug!("critical path: {:?}", critical_path);
    if critical_path.tracks.first().unwrap().facing == critical_path.tracks.last().unwrap().facing {
        travel_section.extend_merge(&from_section.section);
        travel_section.extend_merge(&critical_path);
    }
    travel_section.extend_merge(&to_section.section);
    debug!("travel section: {:?}", travel_section);
    let mut leg_markers = vec![];

    for logical in critical_path.tracks.iter() {
        debug!("  track: {:?}", logical);
        let track_entity = entity_map.tracks[&logical.track()];
        let (_, maybe_marker) = tracks.get(track_entity).unwrap();
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

pub struct ModularRoutePlugin;

impl Plugin for ModularRoutePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(build_route);
        app.add_observer(build_route_leg);
        app.add_systems(Update, draw_route);
    }
}
