use crate::layout_primitives::*;

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
