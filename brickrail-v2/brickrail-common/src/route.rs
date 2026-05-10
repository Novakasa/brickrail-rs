use std::marker::PhantomData;

use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use crate::block::BlockData;
use crate::layout_primitives::*;
use crate::lifecycle::LayoutType;
use crate::marker::MarkerData;

/// A resolved route leg with three stages and collected markers.
#[derive(Component, Clone, Debug)]
pub struct RouteLeg {
    /// The train's facing during this leg. Can change between legs
    /// when travel direction reverses.
    pub facing: Facing,
    /// The portion of the starting block the train traverses to exit.
    pub start_block: RouteLegBlock,
    /// The tracks between the start and target blocks.
    pub travel: Vec<DirectedTrackID>,
    /// The portion of the target block the train enters.
    pub target_block: RouteLegBlock,
    /// All markers collected from the three stages, in travel order.
    pub markers: Vec<RouteLegMarker>,
}

/// A block's participation in a route leg.
#[derive(Clone, Debug)]
pub struct RouteLegBlock {
    pub block_id: BlockID,
    /// The block's section tracks as traversed in this leg (may be reversed
    /// relative to the block's stored section if travel is against section direction).
    pub section: Vec<DirectedTrackID>,
}

/// A marker within a resolved route leg, with its assigned role.
#[derive(Clone, Debug)]
pub struct RouteLegMarker {
    /// The track this marker sits on.
    pub track: TrackID,
    /// The expected marker color for hardware validation.
    pub color: MarkerColor,
    /// The role of this marker in this route leg, if any.
    /// `None` means the marker is only used for visual progress interpolation.
    pub role: Option<MarkerRole>,
    /// Normalized position within the leg (0.0 = start, 1.0 = end).
    pub position: f32,
}

/// The role a marker plays within a specific route leg.
/// Roles align with train-block states. Markers without a role correspond
/// to the Outside state and are used only for visual progress interpolation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerRole {
    /// First marker — the train is exiting the start block.
    Exiting,
    /// Marker before Entered — the train is entering the target block.
    Entering,
    /// Last marker — the train has fully entered the target block. Triggers lock release.
    Entered,
}

impl RouteLeg {
    /// Build route legs from a path (sensor trajectory).
    /// Splits the path at canonical enter markers and resolves each segment into a leg.
    /// Returns None if the path doesn't contain at least two enter markers.
    pub fn build_from_path(
        path: &[LogicalTrackID],
        block_data_map: &HashMap<BlockID, BlockData>,
        marker_data_map: &HashMap<TrackID, MarkerData>,
    ) -> Option<Vec<RouteLeg>> {
        let enter_marker_map = build_enter_marker_map(block_data_map);
        let segments = split_path_at_enter_markers(path, &enter_marker_map)?;

        segments
            .iter()
            .map(|slice| Self::build(slice, &enter_marker_map, block_data_map, marker_data_map))
            .collect()
    }

    /// Build a single leg from a path slice (enter marker to enter marker).
    /// Resolves block sections, travel tracks, and markers.
    fn build(
        sensor_trajectory_logical: &[LogicalTrackID],
        enter_marker_map: &HashMap<LogicalTrackID, LogicalBlockID>,
        block_data_map: &HashMap<BlockID, BlockData>,
        marker_data_map: &HashMap<TrackID, MarkerData>,
    ) -> Option<RouteLeg> {
        let start = *enter_marker_map.get(sensor_trajectory_logical.first()?)?;
        let target = *enter_marker_map.get(sensor_trajectory_logical.last()?)?;

        let start_data = block_data_map.get(&start.block)?;
        let target_data = block_data_map.get(&target.block)?;

        let start_section = oriented_section(start_data, start.direction);
        let target_section = oriented_section(target_data, target.direction);

        // Travel tracks: everything in the slice that isn't in either block's section
        let start_tracks: bevy::platform::collections::HashSet<TrackID> =
            start_data.section.iter().map(|d| d.track).collect();
        let target_tracks: bevy::platform::collections::HashSet<TrackID> =
            target_data.section.iter().map(|d| d.track).collect();
        let travel: Vec<DirectedTrackID> = sensor_trajectory_logical
            .iter()
            .filter(|lt| {
                !start_tracks.contains(&lt.track()) && !target_tracks.contains(&lt.track())
            })
            .map(|lt| lt.directed)
            .collect();

        let markers = collect_markers(sensor_trajectory_logical, marker_data_map);

        Some(RouteLeg {
            facing: start.facing,
            start_block: RouteLegBlock {
                block_id: start.block,
                section: start_section,
            },
            travel,
            target_block: RouteLegBlock {
                block_id: target.block,
                section: target_section,
            },
            markers,
        })
    }
}

// --- ECS relationships and messages ---

/// Relationship: a route leg entity belongs to a train entity.
#[derive(Component)]
#[relationship(relationship_target = TrainLegs)]
pub struct LegOf(pub Entity);

/// Relationship target: a train entity has many route leg entities.
#[derive(Component)]
#[relationship_target(relationship = LegOf)]
pub struct TrainLegs(Vec<Entity>);

/// Message to append pre-built route legs to a train's leg queue.
/// The first new leg's start block must match the last existing leg's target block
/// (if the train already has legs). Legs are spawned in traversal order.
#[derive(Message, Clone)]
pub struct AppendLegs {
    pub train: Entity,
    pub legs: Vec<RouteLeg>,
}

/// Plugin that registers the route leg management system.
pub struct RoutePlugin<L: LayoutType>(PhantomData<L>);

impl<L: LayoutType> Default for RoutePlugin<L> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<L: LayoutType> RoutePlugin<L> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<L: LayoutType> Plugin for RoutePlugin<L> {
    fn build(&self, app: &mut App) {
        app.add_message::<AppendLegs>();
        app.add_systems(Update, handle_append_legs.run_if(on_message::<AppendLegs>));
    }
}

/// System that handles `AppendLegs` messages by spawning leg entities with train relationships.
fn handle_append_legs(
    mut commands: Commands,
    mut messages: MessageReader<AppendLegs>,
    leg_query: Query<&RouteLeg>,
    train_legs_query: Query<&TrainLegs>,
) {
    for msg in messages.read() {
        if msg.legs.is_empty() {
            continue;
        }

        // Check compatibility with last existing leg
        if let Ok(existing_legs) = train_legs_query.get(msg.train) {
            if let Some(&last_leg_entity) = existing_legs.0.last() {
                if let Ok(last_leg) = leg_query.get(last_leg_entity) {
                    let first_new = &msg.legs[0];
                    assert_eq!(
                        last_leg.target_block.block_id, first_new.start_block.block_id,
                        "New legs must start where existing legs end: \
                         last target {:?} != first start {:?}",
                        last_leg.target_block.block_id, first_new.start_block.block_id,
                    );
                }
            }
        }

        for leg in &msg.legs {
            commands.spawn((leg.clone(), LegOf(msg.train)));
        }
    }
}

// --- Route building ---

/// Returns the enter logical track for a logical block.
/// This is the canonical enter marker track with the given facing.
pub fn enter_logical_track(
    data: &BlockData,
    direction: BlockDirection,
    facing: Facing,
) -> LogicalTrackID {
    // From the canonical enter marker table:
    // Aligned + Forward → B (last), Aligned + Backward → A (first)
    // Against + Forward → A (first, opposite dir), Against + Backward → B (last, opposite dir)
    match (direction, facing) {
        (BlockDirection::Aligned, Facing::Forward) => {
            LogicalTrackID::new(*data.section.last().unwrap(), Facing::Forward)
        }
        (BlockDirection::Aligned, Facing::Backward) => {
            LogicalTrackID::new(*data.section.first().unwrap(), Facing::Backward)
        }
        (BlockDirection::Against, Facing::Forward) => {
            LogicalTrackID::new(data.section.first().unwrap().opposite(), Facing::Forward)
        }
        (BlockDirection::Against, Facing::Backward) => {
            LogicalTrackID::new(data.section.last().unwrap().opposite(), Facing::Backward)
        }
    }
}

/// Returns the full block section in travel order for a given direction.
fn oriented_section(data: &BlockData, direction: BlockDirection) -> Vec<DirectedTrackID> {
    match direction {
        BlockDirection::Aligned => data.section.clone(),
        BlockDirection::Against => data.section.iter().rev().map(|d| d.opposite()).collect(),
    }
}

/// Build a map from logical track → logical block for all canonical enter markers.
fn build_enter_marker_map(
    block_data_map: &HashMap<BlockID, BlockData>,
) -> HashMap<LogicalTrackID, LogicalBlockID> {
    let mut map = HashMap::new();
    for (&block_id, data) in block_data_map {
        for direction in [BlockDirection::Aligned, BlockDirection::Against] {
            for facing in [Facing::Forward, Facing::Backward] {
                let logical_track = enter_logical_track(data, direction, facing);
                map.insert(
                    logical_track,
                    LogicalBlockID {
                        block: block_id,
                        direction,
                        facing,
                    },
                );
            }
        }
    }
    map
}

/// Split a path at enter marker boundaries, returning overlapping slices.
/// Each slice runs from one enter marker to the next (inclusive on both ends).
fn split_path_at_enter_markers<'a>(
    path: &'a [LogicalTrackID],
    enter_marker_map: &HashMap<LogicalTrackID, LogicalBlockID>,
) -> Option<Vec<&'a [LogicalTrackID]>> {
    let enter_indices: Vec<usize> = path
        .iter()
        .enumerate()
        .filter(|(_, lt)| enter_marker_map.contains_key(*lt))
        .map(|(i, _)| i)
        .collect();

    if enter_indices.len() < 2 {
        return None;
    }

    Some(
        enter_indices
            .windows(2)
            .map(|w| &path[w[0]..=w[1]])
            .collect(),
    )
}

/// Collect markers along the sensor's trajectory (the path slice), assigning roles by position.
/// Roles: first marker = Exiting, last = Entered, marker before last = Entering.
fn collect_markers(
    path_slice: &[LogicalTrackID],
    marker_data_map: &HashMap<TrackID, MarkerData>,
) -> Vec<RouteLegMarker> {
    let total_tracks = path_slice.len();

    let mut markers: Vec<(TrackID, MarkerColor, f32)> = Vec::new();
    for (i, logical) in path_slice.iter().enumerate() {
        if let Some(data) = marker_data_map.get(&logical.track()) {
            let position = i as f32 / (total_tracks - 1).max(1) as f32;
            markers.push((logical.track(), data.color, position));
        }
    }

    let len = markers.len();
    markers
        .into_iter()
        .enumerate()
        .map(|(idx, (track, color, position))| {
            let role = match idx {
                0 if len >= 2 => Some(MarkerRole::Exiting),
                i if i == len - 1 && len >= 2 => Some(MarkerRole::Entered),
                i if i == len - 2 && len >= 3 => Some(MarkerRole::Entering),
                _ => None,
            };
            RouteLegMarker {
                track,
                color,
                role,
                position,
            }
        })
        .collect()
}
