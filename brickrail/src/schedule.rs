use bevy::prelude::*;

use crate::{
    block::Block,
    layout::{Connections, EntityMap, MarkerMap, TrackLocks},
    marker::Marker,
    train::Train,
};

#[derive(Debug, Clone, Eq, PartialEq, Hash, Default, States)]
pub enum ControlState {
    #[default]
    Manual,
    Random,
    Schedule,
}

#[derive(Resource, Default)]
struct ControlInfo {
    cycle: f32,
    wait_time: f32,
}

fn assign_random_routes(
    q_trains: Query<&Train>,
    q_blocks: Query<&Block>,
    entity_map: Res<EntityMap>,
    connections: Res<Connections>,
    track_locks: Res<TrackLocks>,
    q_markers: Query<&Marker>,
    marker_map: Res<MarkerMap>,
) {
}

struct SchedulePlugin;

impl Plugin for SchedulePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ControlInfo::default());
        app.add_systems(Update, assign_random_routes);
    }
}
