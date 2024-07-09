use bevy::{
    ecs::system::{SystemParam, SystemState},
    prelude::*,
};
use bevy_inspector_egui::egui::{self, CollapsingHeader, Grid, Ui};
use serde::{Deserialize, Serialize};

use crate::{
    block::Block,
    destination::{BlockDirectionFilter, Destination},
    editor::{ControlStateMode, GenericID, Selectable, SelectionState},
    layout::EntityMap,
    layout_primitives::{DestinationID, ScheduleID},
    train::{QueuedDestination, TargetChoiceStrategy, WaitTime},
};

#[derive(Debug, Component, Clone, Serialize, Deserialize, Default)]
pub struct AssignedSchedule {
    pub schedule_id: Option<ScheduleID>,
    pub offset: f32,
}

#[derive(Debug, Component, Clone, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub dest: Option<DestinationID>,
    pub depart_time: f32,
    pub min_wait: f32,
}

impl Default for ScheduleEntry {
    fn default() -> Self {
        Self {
            dest: None,
            depart_time: 0.0,
            min_wait: 4.0,
        }
    }
}

#[derive(Debug, Component, Clone, Serialize, Deserialize)]
pub struct TrainSchedule {
    pub id: ScheduleID,
    pub entries: Vec<ScheduleEntry>,
    #[serde(skip)]
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

    pub fn inspector(ui: &mut Ui, world: &mut World) {
        let mut state = SystemState::<(
            Query<&mut TrainSchedule>,
            Query<(&Destination, Option<&Name>)>,
            Res<EntityMap>,
            Res<SelectionState>,
            Res<AppTypeRegistry>,
        )>::new(world);
        let (mut schedules, destinations, entity_map, selection_state, _type_registry) =
            state.get_mut(world);
        if let Some(entity) = selection_state.get_entity(&entity_map) {
            if let Ok(mut schedule) = schedules.get_mut(entity) {
                ui.heading("Schedule");
                Grid::new("settings").show(ui, |ui| {
                    ui.label("Cycle length [s]");
                    ui.add(egui::DragValue::new(&mut schedule.cycle_length));
                    ui.end_row();

                    ui.label("Cycle offset [s]");
                    ui.add(egui::DragValue::new(&mut schedule.cycle_offset));
                    ui.end_row();
                });
                ui.heading("Stops");
                let mut remove_stop = None;
                for (i, entry) in schedule.entries.iter_mut().enumerate() {
                    CollapsingHeader::new(format!(
                        "Stop {}: {}",
                        i + 1,
                        Destination::label_from_query(&entry.dest, &destinations)
                    ))
                    .id_source(i)
                    .show(ui, |ui| {
                        Grid::new("settings").show(ui, |ui| {
                            ui.label("Destination");
                            Destination::selector_option(&destinations, ui, &mut entry.dest);
                            ui.end_row();
                            ui.label("Departure time [s]");
                            ui.add(egui::DragValue::new(&mut entry.depart_time));
                            ui.end_row();
                            ui.label("Minimum wait time [s]");
                            ui.add(egui::DragValue::new(&mut entry.min_wait));
                            ui.end_row();
                            if ui.button("Remove stop").clicked() {
                                remove_stop = Some(i);
                            }
                        });
                    });
                }
                if let Some(i) = remove_stop {
                    schedule.entries.remove(i);
                }
                if ui.button("Add stop").clicked() {
                    schedule.entries.push(ScheduleEntry::default());
                }
            }
        }
    }
}

impl Selectable for TrainSchedule {
    type SpawnEvent = SpawnScheduleEvent;
    type ID = ScheduleID;

    fn generic_id(&self) -> GenericID {
        GenericID::Schedule(self.id)
    }

    fn id(&self) -> Self::ID {
        self.id
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

#[derive(Debug, Event, Serialize, Deserialize, Clone)]
pub struct SpawnScheduleEvent {
    pub schedule: TrainSchedule,
    pub name: Option<String>,
}

#[derive(SystemParam)]
pub struct SpawnScheduleEventQuery<'w, 's> {
    query: Query<'w, 's, (&'static TrainSchedule, &'static Name)>,
}
impl SpawnScheduleEventQuery<'_, '_> {
    pub fn get(&self) -> Vec<SpawnScheduleEvent> {
        self.query
            .iter()
            .map(|(schedule, name)| SpawnScheduleEvent {
                schedule: schedule.clone(),
                name: Some(name.to_string()),
            })
            .collect()
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
