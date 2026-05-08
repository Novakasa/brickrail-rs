use bevy::platform::collections::HashMap;
use petgraph::algo::astar;

use crate::block::{Block, BlockData};
use crate::layout_primitives::*;
use crate::logical_graph::LogicalGraph;
use crate::lifecycle::{LayoutType, Registry};

/// A resolved route: an ordered sequence of route legs from one block to another.
#[derive(Clone, Debug)]
pub struct Route {
    pub legs: Vec<RouteLeg>,
}

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
/// Markers without a role (`None`) are used only for visual progress interpolation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MarkerRole {
    /// The canonical enter marker — signals the train has entered the target block.
    Enter,
    /// Signals the train is leaving the start block.
    Leaving,
    /// Signals the train is entering the target block (but not yet fully entered).
    Entering,
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
) -> Option<Route> {
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
    split_path_into_legs(&path, start, target, &track_to_block, block_data_map)
}

/// Split a path of logical tracks into route legs at block boundaries.
fn split_path_into_legs(
    path: &[LogicalTrackID],
    start: LogicalBlockID,
    _target: LogicalBlockID,
    track_to_block: &HashMap<TrackID, BlockID>,
    block_data_map: &HashMap<BlockID, BlockData>,
) -> Option<Route> {
    if path.is_empty() {
        return None;
    }

    // Collect the sequence of (block_id, direction) visited, plus travel tracks between them.
    // Start with the start block.
    struct LegBuilder {
        start_block: BlockID,
        start_direction: BlockDirection,
        travel: Vec<DirectedTrackID>,
        facing: Facing,
    }

    let mut legs = Vec::new();
    let mut current = LegBuilder {
        start_block: start.block,
        start_direction: start.direction,
        travel: Vec::new(),
        facing: start.facing,
    };

    for logical in path {
        let track_id = logical.track();
        let in_block = track_to_block.get(&track_id).copied();

        match in_block {
            Some(blk) if blk == current.start_block => {
                // Still in (or re-entering) the current start block — skip
            }
            Some(blk) => {
                // Entered a new block — this is the target of the current leg
                let block_data = block_data_map.get(&blk)?;
                let direction = block_direction_for(block_data, logical.directed)?;

                legs.push(RouteLeg {
                    facing: current.facing,
                    start_block: RouteLegBlock {
                        block_id: current.start_block,
                        section: oriented_section(
                            block_data_map.get(&current.start_block)?,
                            current.start_direction,
                        ),
                    },
                    travel: std::mem::take(&mut current.travel),
                    target_block: RouteLegBlock {
                        block_id: blk,
                        section: oriented_section(block_data, direction),
                    },
                    markers: Vec::new(), // TODO: marker collection
                });

                // This block becomes the start of the next leg
                current.start_block = blk;
                current.start_direction = direction;
                current.facing = logical.facing;
            }
            None => {
                // Travel section
                current.travel.push(logical.directed);
            }
        }
    }

    if legs.is_empty() {
        return None;
    }

    Some(Route { legs })
}

