use bevy::utils::HashMap;

use crate::layout_primitives::*;
use crate::marker::*;

#[derive(Clone, Copy, Debug)]
pub enum RouteStatus {
    Incomplete,
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
        return RouteStatus::Incomplete;
    }

    pub fn advance_sensor(&mut self) -> RouteStatus {
        match self.legs[0].advance_marker() {
            LegStatus::Completed => self.next_leg(),
            _ => RouteStatus::Incomplete,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum LegStatus {
    Incomplete(usize),
    Completed,
}

#[derive(Debug)]
pub struct RouteLeg {
    tracks: Vec<LogicalTrackID>,
    markers: Vec<LogicalMarker>,
    status: LegStatus,
}

impl RouteLeg {
    fn advance_marker(&mut self) -> LegStatus {
        if let LegStatus::Incomplete(index) = self.status {
            if index + 1 == self.markers.len() {
                self.status = LegStatus::Completed;
            } else {
                self.status = LegStatus::Incomplete(index + 1);
            }
            return self.status;
        } else {
            panic!("Can't advance completed leg {:?}!", self);
        }
    }
}
