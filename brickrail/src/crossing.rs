use bevy::prelude::*;
use bevy_inspector_egui::egui::Ui;
use bevy_prototype_lyon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    ble::HubCommandMessage,
    editor::{EditorState, GenericID},
    inspector::{Inspectable, InspectorPlugin},
    layout::EntityMap,
    layout_devices::LayoutDevice,
    layout_primitives::{LayoutDeviceID, TrackID},
    selectable::{Selectable, SelectablePlugin, SelectableType},
    switch_motor::{MotorPosition, PulseMotor},
    track::{LAYOUT_SCALE, TRACK_WIDTH},
};

#[derive(Debug)]
pub enum CrossingPosition {
    Open,
    Closed,
}

impl CrossingPosition {
    pub fn to_motor_position(&self) -> MotorPosition {
        match self {
            CrossingPosition::Open => MotorPosition::Left,
            CrossingPosition::Closed => MotorPosition::Right,
        }
    }
}

#[derive(Debug, Reflect, Serialize, Deserialize, Clone, Component)]
pub struct LevelCrossing {
    id: TrackID,
    pub motors: Vec<Option<LayoutDeviceID>>,
}

impl LevelCrossing {
    pub fn new(id: TrackID) -> Self {
        Self { id, motors: vec![] }
    }
}

impl Inspectable for LevelCrossing {
    fn inspector(ui: &mut Ui, _world: &mut World) {
        ui.label("Level Crossing");
    }

    fn run_condition(selection_state: Res<crate::editor::SelectionState>) -> bool {
        selection_state.selected_type() == Some(SelectableType::Crossing)
    }
}

impl Selectable for LevelCrossing {
    type ID = TrackID;
    type SpawnMessage = SpawnCrossingMessage;

    fn get_type() -> SelectableType {
        SelectableType::Crossing
    }

    fn id(&self) -> Self::ID {
        self.id
    }

    fn generic_id(&self) -> crate::editor::GenericID {
        GenericID::Crossing(self.id)
    }

    fn get_depth(&self) -> f32 {
        1.5
    }

    fn get_distance(
        &self,
        pos: Vec2,
        _transform: Option<&Transform>,
        _stroke: Option<&Shape>,
    ) -> f32 {
        self.id.distance_to(pos) - TRACK_WIDTH * 0.5 / LAYOUT_SCALE
    }
}

#[derive(Serialize, Deserialize, Clone, Message)]
pub struct SpawnCrossingMessage {
    pub switch: LevelCrossing,
    pub name: Option<String>,
}

impl SpawnCrossingMessage {
    pub fn new(switch: LevelCrossing) -> Self {
        Self { switch, name: None }
    }
}

pub fn spawn_crossing(
    mut commands: Commands,
    mut messages: MessageReader<SpawnCrossingMessage>,
    mut entity_map: ResMut<EntityMap>,
) {
    for event in messages.read() {
        let id = event.switch.id.clone();
        let entity = commands.spawn(event.switch.clone()).id();
        entity_map.add_crossing(id, entity);
    }
}

#[derive(Debug, Message)]
pub struct SetCrossingPositionMessage {
    pub id: TrackID,
    pub position: CrossingPosition,
}

pub fn update_crossing_position(
    mut messages: MessageReader<SetCrossingPositionMessage>,
    crossings: Query<&LevelCrossing>,
    mut motors: Query<(&mut PulseMotor, &LayoutDevice)>,
    entity_map: Res<EntityMap>,
    mut hub_commands: MessageWriter<HubCommandMessage>,
    editor_state: Res<State<EditorState>>,
) {
    for update in messages.read() {
        let position = update.position.to_motor_position();
        if let Some(entity) = entity_map.crossings.get(&update.id) {
            let crossing = crossings.get(*entity).unwrap();
            for motor_id in &crossing.motors {
                if let Some(motor_id) = motor_id {
                    let entity = entity_map.layout_devices.get(motor_id).unwrap();
                    let (mut motor, device) = motors.get_mut(*entity).unwrap();
                    if motor.position == position {
                        continue;
                    }

                    if editor_state.get().ble_commands_enabled() {
                        if let Some(command) = PulseMotor::switch_command(device, &position) {
                            println!("Sending switch command {:?}", command);
                            hub_commands.write(command);
                        }
                    }
                    motor.position = position;
                }
            }
        }
    }
}

pub struct CrossingPlugin;

impl Plugin for CrossingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InspectorPlugin::<LevelCrossing>::new());
        app.add_plugins(SelectablePlugin::<LevelCrossing>::new());
        app.add_message::<SpawnCrossingMessage>();
        app.add_message::<SetCrossingPositionMessage>();
        app.add_systems(
            Update,
            update_crossing_position.run_if(on_message::<SetCrossingPositionMessage>),
        );
        app.add_systems(
            PostUpdate,
            spawn_crossing.run_if(on_message::<SpawnCrossingMessage>),
        );
    }
}
