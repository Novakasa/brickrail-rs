use crate::{
    layout_primitives::*,
    route_modular::{LegPosition, RouteLegAssigned, RouteLegTravelSection},
    track::LAYOUT_SCALE,
};
use bevy::{color::palettes::tailwind::LIME_100, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Component, Debug)]
pub struct ModularTrain;

#[derive(Debug, Default, Clone, Component)]
pub enum TrainState {
    #[default]
    Stop,
    Run {
        facing: Facing,
        speed: TrainSpeed,
    },
}

impl TrainState {
    pub fn get_speed(&self) -> f32 {
        match self {
            TrainState::Stop => 0.0,
            TrainState::Run { speed, facing } => facing.get_sign() * speed.get_speed(),
        }
    }
}

#[derive(
    Clone,
    Copy,
    Hash,
    PartialEq,
    PartialOrd,
    Ord,
    Eq,
    Debug,
    Default,
    Serialize,
    Deserialize,
    Reflect,
    Component,
)]
pub enum TrainSpeed {
    Slow,
    #[default]
    Cruise,
    Fast,
}

impl TrainSpeed {
    pub fn get_speed(&self) -> f32 {
        match self {
            TrainSpeed::Slow => 2.0,
            TrainSpeed::Cruise => 4.0,
            TrainSpeed::Fast => 8.0,
        }
    }

    pub fn as_train_u8(&self) -> u8 {
        match self {
            TrainSpeed::Slow => 2,
            TrainSpeed::Cruise => 3,
            TrainSpeed::Fast => 1,
        }
    }
}

fn debug_draw_train(
    train_query: Query<(&RouteLegAssigned, &LegPosition)>,
    legs: Query<&RouteLegTravelSection>,
    mut gizmos: Gizmos,
) {
    for (leg_assigned, leg_position) in train_query.iter() {
        let leg_entity = leg_assigned.0;
        if let Ok(leg_section) = legs.get(leg_entity) {
            let pos = leg_section.section.interpolate_pos(leg_position.position) * LAYOUT_SCALE;
            gizmos.circle_2d(pos, 10.0, LIME_100);
        }
    }
}

#[derive(Component, Debug)]
#[relationship(relationship_target=Wagons)]
struct WagonOf(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=WagonOf)]
struct Wagons(Vec<Entity>);

pub struct ModularTrainPlugin;

impl Plugin for ModularTrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, debug_draw_train);
    }
}
