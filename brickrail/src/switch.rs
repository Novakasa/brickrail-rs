use bevy::prelude::*;
use bevy_trait_query::RegisterExt;
use serde::{Deserialize, Serialize};

use crate::{
    ble_switch::BLESwitch,
    editor::{GenericID, Selectable},
    layout::EntityMap,
    layout_primitives::*,
    track::{LAYOUT_SCALE, TRACK_WIDTH},
};

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone)]
pub struct Switch {
    id: DirectedTrackID,
    positions: Vec<SwitchPosition>,
    #[serde(skip)]
    pos_index: usize,
}

impl Switch {
    pub fn set_positions(&mut self, positions: Vec<SwitchPosition>) {
        self.positions = positions;
        self.pos_index = 0;
    }
}

impl Selectable for Switch {
    fn get_id(&self) -> GenericID {
        GenericID::Switch(self.id)
    }

    fn get_depth(&self) -> f32 {
        1.5
    }

    fn get_distance(&self, pos: Vec2) -> f32 {
        self.id.to_slot().get_vec2().distance(pos) - TRACK_WIDTH * 0.5 / LAYOUT_SCALE
    }
}

#[derive(Serialize, Deserialize, Clone, Event)]
pub struct SpawnSwitchEvent {
    pub switch: Switch,
    pub ble_switch: BLESwitch,
}

#[derive(Debug, Event)]
pub struct UpdateSwitchTurnsEvent {
    pub id: DirectedTrackID,
    pub positions: Vec<SwitchPosition>,
}

#[derive(Debug, Event)]
pub struct SetSwitchPositionEvent {
    pub id: DirectedTrackID,
    pub position: SwitchPosition,
}

pub fn update_switch_position(
    mut events: EventReader<SetSwitchPositionEvent>,
    mut switches: Query<(&mut Switch, &mut BLESwitch)>,
    entity_map: Res<EntityMap>,
) {
    for update in events.read() {
        if let Some(entity) = entity_map.switches.get(&update.id) {
            let (mut switch, mut ble_switch) = switches.get_mut(*entity).unwrap();
        }
    }
}

pub fn update_switch_turns(
    mut events: EventReader<UpdateSwitchTurnsEvent>,
    mut switch_spawn_events: EventWriter<SpawnSwitchEvent>,
    mut switches: Query<(&mut Switch, &mut BLESwitch)>,
    entity_map: Res<EntityMap>,
) {
    for update in events.read() {
        if update.positions.len() > 1 {
            if let Some(entity) = entity_map.switches.get(&update.id) {
                let (mut switch, mut ble_switch) = switches.get_mut(*entity).unwrap();
                switch.set_positions(update.positions.clone());
                ble_switch.set_num_motors(update.positions.len() - 1);
            } else {
                let mut ble_switch = BLESwitch::new(update.id);
                ble_switch.set_num_motors(update.positions.len() - 1);
                switch_spawn_events.send(SpawnSwitchEvent {
                    switch: Switch {
                        id: update.id,
                        positions: update.positions.clone(),
                        pos_index: 0,
                    },
                    ble_switch: ble_switch,
                });
            }
        } else {
            //todo!("Remove switches")
        }
    }
}

pub fn draw_switches(mut gizmos: Gizmos, switches: Query<&Switch>) {
    for switch in switches.iter() {
        let pos = switch.id.to_slot().get_vec2();
        gizmos.circle_2d(pos * LAYOUT_SCALE, 0.1 * LAYOUT_SCALE, Color::RED);
    }
}

pub fn spawn_switch(
    mut commands: Commands,
    mut events: EventReader<SpawnSwitchEvent>,
    mut entity_map: ResMut<EntityMap>,
) {
    for spawn_event in events.read() {
        let entity = commands
            .spawn((spawn_event.switch.clone(), spawn_event.ble_switch.clone()))
            .id();
        entity_map.add_switch(spawn_event.switch.id, entity);
    }
}

pub struct SwitchPlugin;

impl Plugin for SwitchPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnSwitchEvent>();
        app.add_event::<UpdateSwitchTurnsEvent>();
        app.register_component_as::<dyn Selectable, Switch>();
        app.add_systems(
            Update,
            (
                spawn_switch.run_if(on_event::<SpawnSwitchEvent>()),
                update_switch_turns.run_if(on_event::<UpdateSwitchTurnsEvent>()),
                draw_switches,
            ),
        );
    }
}
