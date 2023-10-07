use bevy::prelude::*;
use bevy::utils::HashMap;

use crate::layout_primitives::*;
use crate::marker::*;

#[derive(Resource, Default)]
struct Scheduler {
    locked_tracks: HashMap<TrackID, TrainID>,
}

pub enum TrainInstruction {
    Stop,
    Run {
        flip_heading: bool,
        speed: MarkerSpeed,
    },
}

impl TrainInstruction {
    fn with_flip(&self, flip: bool) -> Self {
        match self {
            Self::Stop => Self::Stop,
            Self::Run {
                flip_heading: _,
                speed,
            } => Self::Run {
                flip_heading: flip,
                speed: *speed,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum RouteStatus {
    Start,
    Running,
    Paused,
    Completed,
}

pub struct Route {
    legs: Vec<RouteLeg>,
}

impl Route {
    pub fn new() -> Self {
        Route { legs: vec![] }
    }

    pub fn push_leg(mut self, leg: RouteLeg) {
        self.legs.push(leg);
    }

    pub fn next_leg(&mut self) -> RouteStatus {
        self.legs.remove(0);
        if self.legs.len() == 0 {
            return RouteStatus::Completed;
        }
        return RouteStatus::Running;
    }

    pub fn get_current_leg(&self) -> &RouteLeg {
        &self.legs[0]
    }

    pub fn get_current_leg_mut(&mut self) -> &mut RouteLeg {
        &mut self.legs[0]
    }

    pub fn advance_sensor(&mut self) -> RouteStatus {
        match self.get_current_leg_mut().advance_marker() {
            LegStatus::Completed => self.next_leg(),
            _ => RouteStatus::Running,
        }
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
enum LegStatus {
    Running(usize),
    Completed,
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
enum LegIntention {
    Pass,
    Stop,
}

#[derive(Debug)]
pub struct RouteLeg {
    tracks: Vec<LogicalTrackID>,
    markers: Vec<LogicalMarker>,
    status: LegStatus,
    intention: LegIntention,
}

impl RouteLeg {
    fn advance_marker(&mut self) -> LegStatus {
        if let LegStatus::Running(index) = self.status {
            if index + 1 == self.markers.len() {
                self.status = LegStatus::Completed;
            } else {
                self.status = LegStatus::Running(index + 1);
            }
            return self.status;
        } else {
            panic!("Can't advance completed leg {:?}!", self);
        }
    }

    fn has_entered(&self) -> bool {
        match self.status {
            LegStatus::Completed => true,
            LegStatus::Running(index) => {
                for (i, marker) in self.markers.iter().enumerate().rev() {
                    if let MarkerKey::Enter(_) = marker.key {
                        return false;
                    }
                    if i == index {
                        return true;
                    }
                }
                return true;
            }
        }
    }

    fn get_last_marker(&self) -> &LogicalMarker {
        match self.status {
            LegStatus::Completed => self.markers.last().unwrap(),
            LegStatus::Running(index) => self.markers.get(index).unwrap(),
        }
    }

    fn get_current_instruction(&self) -> TrainInstruction {
        let should_stop = self.intention == LegIntention::Stop;

        if self.status == LegStatus::Completed {
            TrainInstruction::Stop
        } else {
            let speed = if should_stop && self.has_entered() {
                MarkerSpeed::Slow
            } else {
                self.get_last_marker().speed
            };
            TrainInstruction::Run {
                flip_heading: false,
                speed: speed,
            }
        }
    }

    fn get_enter_instruction(&self) -> TrainInstruction {
        if self.status != LegStatus::Running(0) {
            panic!("enter instruction only valid when index is 0!");
        }
        self.get_current_instruction()
            .with_flip(self.is_flip_type())
    }

    fn is_flip_type(&self) -> bool {
        if self.tracks.len() < 2 {
            return false;
        }
        self.tracks.get(0).unwrap().reversed() == *self.tracks.get(1).unwrap()
    }
}
