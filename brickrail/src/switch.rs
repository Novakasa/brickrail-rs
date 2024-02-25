use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    editor::{GenericID, Selectable, SpawnEvent},
    layout::EntityMap,
    layout_primitives::*,
    track::{LAYOUT_SCALE, TRACK_WIDTH},
};

#[derive(Component, Debug, Reflect, Serialize, Deserialize, Clone)]
pub struct Switch {
    id: DirectedTrackID,
    positions: Vec<SwitchPosition>,
    pos_index: usize,
}

impl Selectable for Switch {
    fn get_id(&self) -> GenericID {
        GenericID::Switch(self.id)
    }

    fn get_depth(&self) -> f32 {
        1.5
    }

    fn get_distance(&self, pos: Vec2) -> f32 {
        self.id.distance_to(pos) - TRACK_WIDTH * 0.5 / LAYOUT_SCALE
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct SerializedSwitch {
    pub switch: Switch,
}

#[derive(Debug, Event)]
pub struct UpdateSwitchTurnsEvent {
    pub id: DirectedTrackID,
    pub positions: Vec<SwitchPosition>,
}

pub fn update_switches(
    mut events: EventReader<UpdateSwitchTurnsEvent>,
    mut switch_spawn_events: EventWriter<SpawnEvent<SerializedSwitch>>,
    mut switches: Query<&mut Switch>,
    entity_map: Res<EntityMap>,
) {
    for update in events.read() {
        if update.positions.len() > 1 {
            if let Some(entity) = entity_map.switches.get(&update.id) {
                let mut switch = switches.get_mut(*entity).unwrap();
                switch.positions = update.positions.clone();
            } else {
                switch_spawn_events.send(SpawnEvent(SerializedSwitch {
                    switch: Switch {
                        id: update.id,
                        positions: update.positions.clone(),
                        pos_index: 0,
                    },
                }));
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

pub fn spawn_switches(
    mut commands: Commands,
    mut events: EventReader<SpawnEvent<SerializedSwitch>>,
    mut entity_map: ResMut<EntityMap>,
) {
    for SpawnEvent(serialized_switch) in events.read() {
        let entity = commands.spawn(serialized_switch.switch.clone()).id();
        entity_map.add_switch(serialized_switch.switch.id, entity);
    }
}

pub struct SwitchPlugin;

impl Plugin for SwitchPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnEvent<SerializedSwitch>>();
        app.add_event::<UpdateSwitchTurnsEvent>();
        app.add_systems(
            Update,
            (
                spawn_switches.run_if(on_event::<SpawnEvent<SerializedSwitch>>()),
                update_switches.run_if(on_event::<UpdateSwitchTurnsEvent>()),
                draw_switches,
            ),
        );
    }
}
