use bevy::{
    ecs::system::{SystemParam, SystemState},
    prelude::*,
};
use bevy_inspector_egui::egui::{self, CollapsingHeader, Grid, Ui};
use serde::{Deserialize, Serialize};

use crate::{
    destination::Destination,
    editor::{ControlState, ControlStateMode, GenericID, Selectable, SelectionState},
    layout::EntityMap,
    layout_primitives::{DestinationID, ScheduleID},
    train::{QueuedDestination, TargetChoiceStrategy, WaitTime},
};

#[derive(Debug, Component, Clone, Serialize, Deserialize, Default)]
pub struct AssignedSchedule {
    pub schedule_id: Option<ScheduleID>,
    pub offset: f32,
    #[serde(skip)]
    pub current_stop_index: usize,
}

impl AssignedSchedule {
    pub fn advance_stops(
        &mut self,
        schedule: &TrainSchedule,
        time: f32,
        wait_time: f32,
    ) -> Option<QueuedDestination> {
        let current_stop = self.curent_stop(schedule);

        if self.next_departure(time, schedule) < 0.0 && wait_time >= current_stop.min_wait {
            self.current_stop_index += 1;
            if self.current_stop_index >= schedule.entries.len() {
                self.current_stop_index = 0;
            }
            let current_stop = schedule.entries[self.current_stop_index].clone();
            return Some(QueuedDestination {
                dest: current_stop.dest.unwrap(),
                strategy: TargetChoiceStrategy::Closest,
                allow_locked: false,
            });
        }
        None
    }

    pub fn curent_stop(&self, schedule: &TrainSchedule) -> ScheduleEntry {
        schedule.entries[self.current_stop_index].clone()
    }

    pub fn cycle_time(&self, time: f32, schedule: &TrainSchedule) -> f32 {
        let cycle_time = (time + schedule.cycle_offset + self.offset) % schedule.cycle_length;
        cycle_time
    }

    pub fn next_departure(&self, time: f32, schedule: &TrainSchedule) -> f32 {
        let current_stop = self.curent_stop(schedule);
        let prev_stop = schedule.entries
            [(self.current_stop_index + schedule.entries.len() - 1) % schedule.entries.len()]
        .clone();
        let cycle_time = self.cycle_time(time, schedule);
        let next_departure = current_stop.depart_time - cycle_time;
        if current_stop.depart_time < prev_stop.depart_time {
            // this is for the wrapping case, the depart time is earlier than cycle time before wrapping
            if cycle_time > prev_stop.depart_time {
                return next_departure + schedule.cycle_length;
            }
        }
        next_departure
    }
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
            Query<(&Name, &AssignedSchedule, Option<&WaitTime>)>,
            Res<ControlInfo>,
        )>::new(world);
        let (
            mut schedules,
            destinations,
            entity_map,
            selection_state,
            _type_registry,
            q_assigned,
            control_info,
        ) = state.get_mut(world);
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
                ui.separator();
                ui.heading("Assigned trains");
                for (name, assigned, wait_option) in q_assigned.iter() {
                    if assigned.schedule_id != Some(schedule.id) {
                        continue;
                    }
                    ui.label(name.to_string());
                    let cycle_time = assigned.cycle_time(control_info.time, &schedule);
                    let current_stop = assigned.curent_stop(&schedule);
                    let next_departure = assigned.next_departure(control_info.time, &schedule);
                    let destination = entity_map
                        .query_get(
                            &destinations,
                            &GenericID::Destination(current_stop.dest.unwrap()),
                        )
                        .unwrap();
                    ui.label(format!(
                        "Current stop {}: {}",
                        assigned.current_stop_index + 1,
                        destination.1.unwrap().to_string()
                    ));
                    ui.label(format!("Next departure: {:1.1}", next_departure));
                    ui.label(format!("Cycle time: {:1.1}", cycle_time,));
                    if let Some(wait_time) = wait_option {
                        ui.label(format!("Wait time: {:1.1}", wait_time.time));
                    }
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
pub struct ControlInfo {
    pub time: f32,
    pub wait_time: f32,
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
    mut commands: Commands,
    control_info: Res<ControlInfo>,
) {
    for (entity, wait_time) in q_wait_time.iter() {
        if wait_time.time > control_info.wait_time {
            println!("Assigning random route to {:?}", entity);
            commands.entity(entity).insert(QueuedDestination {
                dest: DestinationID::Random,
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

fn update_time(time: Res<Time>, mut control_info: ResMut<ControlInfo>) {
    control_info.time += time.delta_seconds();
}

fn update_schedules(
    control_info: Res<ControlInfo>,
    q_schedules: Query<&TrainSchedule>,
    mut q_assignments: Query<
        (Entity, &mut AssignedSchedule, &WaitTime),
        Without<QueuedDestination>,
    >,
    entity_map: Res<EntityMap>,
    mut commands: Commands,
) {
    for (entity, mut assigned_schedule, wait_time) in q_assignments.iter_mut() {
        if let Some(schedule_id) = assigned_schedule.schedule_id {
            let schedule = q_schedules
                .get(
                    entity_map
                        .get_entity(&GenericID::Schedule(schedule_id))
                        .unwrap(),
                )
                .unwrap();
            if let Some(queued_dest) =
                assigned_schedule.advance_stops(schedule, control_info.time, wait_time.time)
            {
                commands.entity(entity).insert(queued_dest);
            }
        }
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
                update_time.run_if(in_state(ControlState)),
                assign_random_routes.run_if(in_state(ControlStateMode::Random)),
                update_schedules.run_if(in_state(ControlStateMode::Schedule)),
                spawn_schedule.run_if(on_event::<SpawnScheduleEvent>()),
            ),
        );
    }
}
