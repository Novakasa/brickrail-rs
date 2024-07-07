use bevy::{prelude::*, transform::commands};

use crate::{
    block::Block,
    editor::{ControlState, ControlStateMode},
    layout::{Connections, EntityMap, MarkerMap, TrackLocks},
    layout_primitives::{BlockDirection, BlockID, DestinationID, Facing},
    marker::Marker,
    train::{QueuedDestination, TargetChoiceStrategy, Train, WaitTime},
};

#[derive(Debug, Clone, Component)]
pub struct Destination {
    pub id: DestinationID,
    pub blocks: Vec<(BlockID, Option<BlockDirection>, Option<Facing>)>,
}

pub struct ScheduleEntry {
    pub dest: Destination,
    pub depart_time: f32,
    pub max_wait: Option<f32>,
}

pub struct Schedule {
    pub entries: Vec<ScheduleEntry>,
    pub current: usize,
    pub cycle_length: f32,
    pub cycle_offset: f32,
}

#[derive(Resource)]
struct ControlInfo {
    cycle: f32,
    wait_time: f32,
}

impl Default for ControlInfo {
    fn default() -> Self {
        Self {
            cycle: 0.0,
            wait_time: 4.0,
        }
    }
}

fn assign_random_routes(
    q_wait_time: Query<(Entity, &WaitTime), Without<QueuedDestination>>,
    q_blocks: Query<&Block>,
    mut commands: Commands,
    control_info: Res<ControlInfo>,
) {
    for (entity, wait_time) in q_wait_time.iter() {
        if wait_time.time > control_info.wait_time {
            let dest = Destination {
                id: DestinationID::new(0),
                blocks: q_blocks
                    .iter()
                    .map(|block| (block.id, None, None))
                    .collect(),
            };
            println!("Assigning random route to {:?}", entity);
            commands.entity(entity).insert(QueuedDestination {
                dest,
                strategy: TargetChoiceStrategy::Random,
                allow_locked: false,
            });
        }
    }
}

pub struct SchedulePlugin;

impl Plugin for SchedulePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ControlInfo::default());
        app.add_systems(
            Update,
            assign_random_routes.run_if(in_state(ControlStateMode::Random)),
        );
    }
}
