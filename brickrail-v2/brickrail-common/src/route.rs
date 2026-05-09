use bevy::platform::collections::HashMap;
use petgraph::algo::astar;

use crate::block::{Block, BlockData};
use crate::layout_primitives::*;
use crate::logical_graph::LogicalGraph;
use crate::lifecycle::{LayoutType, Registry};
use crate::marker::MarkerData;

/// A resolved route leg with three stages and collected markers.
#[derive(Clone, Debug)]
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

// --- Route building ---

/// Build a track-to-block lookup from block data.
fn build_track_to_block(
    block_registry: &Registry<Block, impl LayoutType>,
    block_data: &HashMap<BlockID, BlockData>,
) -> HashMap<TrackID, BlockID> {
    let mut map = HashMap::new();
    for (id, _) in block_registry.iter() {
        if let Some(data) = block_data.get(id) {
            for directed in &data.section {
                map.insert(directed.track, *id);
            }
        }
    }
    map
}

/// Returns the enter logical track for a logical block.
/// This is the canonical enter marker track with the given facing.
fn enter_logical_track(data: &BlockData, direction: BlockDirection, facing: Facing) -> LogicalTrackID {
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

/// Determine the block direction for a given track traversal within a block.
fn block_direction_for(data: &BlockData, directed: DirectedTrackID) -> Option<BlockDirection> {
    // Check if this directed track matches the section direction (aligned)
    if data.section.contains(&directed) {
        return Some(BlockDirection::Aligned);
    }
    // Check if the opposite matches (against)
    if data.section.contains(&directed.opposite()) {
        return Some(BlockDirection::Against);
    }
    None
}

/// Returns the full block section in travel order for a given direction.
fn oriented_section(data: &BlockData, direction: BlockDirection) -> Vec<DirectedTrackID> {
    match direction {
        BlockDirection::Aligned => data.section.clone(),
        BlockDirection::Against => data.section.iter().rev().map(|d| d.opposite()).collect(),
    }
}

/// Build a route from a start logical block to a target logical block.
/// Uses A* (with uniform cost) on the logical graph.
/// Returns None if no path exists.
pub fn build_route<L: LayoutType>(
    start: LogicalBlockID,
    target: LogicalBlockID,
    logical_graph: &LogicalGraph<L>,
    block_registry: &Registry<Block, L>,
    block_data_map: &HashMap<BlockID, BlockData>,
    marker_data_map: &HashMap<TrackID, MarkerData>,
) -> Option<Vec<RouteLeg>> {
    let start_data = block_data_map.get(&start.block)?;
    let target_data = block_data_map.get(&target.block)?;

    let start_track = enter_logical_track(start_data, start.direction, start.facing);
    let target_track = enter_logical_track(target_data, target.direction, target.facing);

    // A* with uniform cost (equivalent to BFS)
    let (_cost, path) = astar(
        &logical_graph.graph,
        start_track,
        |n| n == target_track,
        |_| 1u32,
        |_| 0u32,
    )?;

    let track_to_block = build_track_to_block(block_registry, block_data_map);

    // Split path into legs.
    // The path goes from the enter marker of the start block to the enter marker of the
    // target block. We walk the path, identify which tracks belong to blocks vs travel,
    // and split at block boundaries. Block sections come from block data, not from the path.
    split_path_into_legs(&path, start, target, &track_to_block, block_data_map, marker_data_map)
}

/// A raw segment of the path between two consecutive blocks.
/// Contains only what the path walk directly observes — no resolved block data.
struct PathSegment {
    /// Travel tracks between the two blocks (not belonging to either block).
    travel: Vec<DirectedTrackID>,
    /// The logical track that entered the target block (determines direction and facing).
    target_entry: LogicalTrackID,
    /// The target block ID.
    target_block: BlockID,
}

/// Split a path of logical tracks into raw segments at block boundaries.
fn split_path_into_segments(
    path: &[LogicalTrackID],
    start_block: BlockID,
    track_to_block: &HashMap<TrackID, BlockID>,
) -> Option<Vec<PathSegment>> {
    if path.is_empty() {
        return None;
    }

    let mut segments = Vec::new();
    let mut current_block = start_block;
    let mut travel = Vec::new();

    for logical in path {
        let track_id = logical.track();
        let in_block = track_to_block.get(&track_id).copied();

        match in_block {
            Some(blk) if blk == current_block => {
                // Still in the current block — skip
            }
            Some(blk) => {
                segments.push(PathSegment {
                    travel: std::mem::take(&mut travel),
                    target_entry: *logical,
                    target_block: blk,
                });
                current_block = blk;
            }
            None => {
                travel.push(logical.directed);
            }
        }
    }

    if segments.is_empty() {
        return None;
    }

    Some(segments)
}

/// Collect markers from all tracks in a leg, assigning roles by position.
/// Roles: first marker = Exiting, last = Entered, marker before last = Entering.
fn collect_markers(
    start_section: &[DirectedTrackID],
    travel: &[DirectedTrackID],
    target_section: &[DirectedTrackID],
    marker_data_map: &HashMap<TrackID, MarkerData>,
) -> Vec<RouteLegMarker> {
    let total_tracks = start_section.len() + travel.len() + target_section.len();

    let all_tracks = start_section.iter()
        .chain(travel.iter())
        .chain(target_section.iter());

    // First pass: collect markers with positions, roles assigned after.
    let mut markers: Vec<(TrackID, MarkerColor, f32)> = Vec::new();
    for (i, directed) in all_tracks.enumerate() {
        if let Some(data) = marker_data_map.get(&directed.track) {
            let position = i as f32 / (total_tracks - 1).max(1) as f32;
            markers.push((directed.track, data.color, position));
        }
    }

    let len = markers.len();
    markers.into_iter().enumerate().map(|(idx, (track, color, position))| {
        let role = match idx {
            0 if len >= 2 => Some(MarkerRole::Exiting),
            i if i == len - 1 && len >= 2 => Some(MarkerRole::Entered),
            i if i == len - 2 && len >= 3 => Some(MarkerRole::Entering),
            _ => None,
        };
        RouteLegMarker { track, color, role, position }
    }).collect()
}

/// Build a route leg from a path segment and the start logical block.
/// Returns the leg and the logical block ID for the target (used as start of the next leg).
fn build_leg(
    start: LogicalBlockID,
    segment: PathSegment,
    block_data_map: &HashMap<BlockID, BlockData>,
    marker_data_map: &HashMap<TrackID, MarkerData>,
) -> Option<(RouteLeg, LogicalBlockID)> {
    let start_data = block_data_map.get(&start.block)?;
    let target_data = block_data_map.get(&segment.target_block)?;
    let target_direction = block_direction_for(target_data, segment.target_entry.directed)?;

    let target = LogicalBlockID {
        block: segment.target_block,
        direction: target_direction,
        facing: segment.target_entry.facing,
    };

    let start_section = oriented_section(start_data, start.direction);
    let target_section = oriented_section(target_data, target_direction);

    let markers = collect_markers(
        &start_section,
        &segment.travel,
        &target_section,
        marker_data_map,
    );

    let leg = RouteLeg {
        facing: start.facing,
        start_block: RouteLegBlock {
            block_id: start.block,
            section: start_section,
        },
        travel: segment.travel,
        target_block: RouteLegBlock {
            block_id: target.block,
            section: target_section,
        },
        markers,
    };

    Some((leg, target))
}

/// Split a path of logical tracks into route legs at block boundaries.
fn split_path_into_legs(
    path: &[LogicalTrackID],
    start: LogicalBlockID,
    _target: LogicalBlockID,
    track_to_block: &HashMap<TrackID, BlockID>,
    block_data_map: &HashMap<BlockID, BlockData>,
    marker_data_map: &HashMap<TrackID, MarkerData>,
) -> Option<Vec<RouteLeg>> {
    let segments = split_path_into_segments(path, start.block, track_to_block)?;
    let mut legs = Vec::new();
    let mut current_start = start;
    for segment in segments {
        let (leg, next_start) = build_leg(current_start, segment, block_data_map, marker_data_map)?;
        legs.push(leg);
        current_start = next_start;
    }
    if legs.is_empty() {
        return None;
    }
    Some(legs)
}

