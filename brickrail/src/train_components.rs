use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::layout_primitives::*;

#[derive(Component, Debug)]
struct Train {
    pub id: TrainID,
}

#[derive(Component, Debug)]
#[relationship(relationship_target=RouteAssignedTo)]
struct RouteAssigned(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=RouteAssigned)]
struct RouteAssignedTo(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target=RouteLegsAssignedTo)]
struct RouteLegAssigned(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=RouteLegAssigned)]
struct RouteLegsAssignedTo(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target=RouteLegs)]
struct RouteLegOf(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=RouteLegOf)]
struct RouteLegs(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target=LegTargetOf)]
struct LegTarget(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=LegTarget)]
struct LegTargetOf(Vec<Entity>);

#[derive(Component, Debug)]
#[relationship(relationship_target=LegStartOf)]
struct LegStart(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=LegStart)]
struct LegStartOf(Vec<Entity>);

#[derive(Component, Debug)]
struct Route;

#[derive(Component, Debug)]
struct RouteLeg;

#[derive(Component, Debug)]
struct LegPosition {
    pub position: f32,
    pub prev_marker_index: usize,
}

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

#[derive(Component, Debug)]
#[relationship(relationship_target=Wagons)]
struct WagonOf(Entity);

#[derive(Component, Debug)]
#[relationship_target(relationship=WagonOf)]
struct Wagons(Vec<Entity>);
