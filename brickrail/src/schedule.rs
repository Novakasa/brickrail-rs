use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    block::Block,
    destination::{BlockDirectionFilter, Destination},
    editor::{ControlStateMode, GenericID, Selectable},
    layout_primitives::{DestinationID, ScheduleID},
    train::{QueuedDestination, TargetChoiceStrategy, WaitTime},
};

#[derive(Debug, Component)]
pub struct AssignedSchedule {
    pub schedule_id: ScheduleID,
    pub offset: f32,
}

#[derive(Debug, Component, Clone, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub dest: DestinationID,
    pub depart_time: f32,
    pub min_wait: f32,
}

#[derive(Debug, Component, Clone, Serialize, Deserialize)]
pub struct TrainSchedule {
    pub id: ScheduleID,
    pub entries: Vec<ScheduleEntry>,
    pub current: usize,
    pub cycle_length: f32,
    pub cycle_offset: f32,
}

impl TrainSchedule {
    pub fn new(id: ScheduleID) -> Self {
        Self {
            id,
            entries: vec![],
            current: 0,
            cycle_length: 0.0,
            cycle_offset: 0.0,
        }
    }
}

#[derive(Debug, Event, Serialize, Deserialize)]
pub struct SpawnScheduleEvent {
    pub schedule: TrainSchedule,
    pub name: Option<String>,
}

impl Selectable for TrainSchedule {
    type SpawnEvent = SpawnScheduleEvent;

    fn get_id(&self) -> GenericID {
        GenericID::Schedule(self.id)
    }

    fn default_spawn_event(
        entity_map: &mut ResMut<crate::layout::EntityMap>,
    ) -> Option<Self::SpawnEvent> {
        Some(SpawnScheduleEvent {
            schedule: TrainSchedule::new(entity_map.new_schedule_id()),
            name: None,
        })
    }
}

#[derive(Resource)]
struct ControlInfo {
    time: f32,
    wait_time: f32,
}

impl Default for ControlInfo {
    fn default() -> Self {
        Self {
            time: 0.0,
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
                    .map(|block| (block.id, BlockDirectionFilter::Any, None))
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

fn spawn_schedule(
    mut commands: Commands,
    mut events: EventReader<SpawnScheduleEvent>,
    mut entity_map: ResMut<crate::layout::EntityMap>,
) {
    for event in events.read() {
        let schedule = event.schedule.clone();
        let id = schedule.id;
        let name = Name::new(event.name.clone().unwrap_or(format!("{}", id)));
        let entity = commands.spawn((name, schedule)).id();
        entity_map.add_schedule(id, entity);
    }
}

pub struct SchedulePlugin;

impl Plugin for SchedulePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ControlInfo::default());
        app.add_event::<SpawnScheduleEvent>();
        app.add_systems(
            Update,
            (
                assign_random_routes.run_if(in_state(ControlStateMode::Random)),
                spawn_schedule.run_if(on_event::<SpawnScheduleEvent>()),
            ),
        );
    }
}
