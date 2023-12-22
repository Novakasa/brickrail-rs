use core::panic;
use std::marker;

use bevy::prelude::*;
use itertools::Itertools;

use crate::layout::Layout;
use crate::layout_primitives::*;
use crate::marker::*;
use crate::section::LogicalSection;

#[derive(Debug, Clone)]
pub struct RouteMarkerData {
    pub track: LogicalTrackID,
    pub color: MarkerColor,
    pub speed: MarkerSpeed,
    pub key: MarkerKey,
}

pub fn build_route(
    logical_section: &LogicalSection,
    q_markers: &Query<&Marker>,
    layout: &Layout,
) -> Route {
    let mut route = Route::new();
    let in_tracks = layout.in_markers.keys().collect_vec();
    let split = logical_section.split_by_tracks(in_tracks);

    for (section, in_track) in split {
        let target_id = layout.in_markers.get(&in_track).unwrap();
        let mut leg_markers = Vec::new();

        for logical in section.tracks.iter() {
            let marker = q_markers
                .get(*layout.markers.get(&logical.track()).unwrap())
                .unwrap();

            let route_marker = RouteMarkerData {
                track: logical.clone(),
                color: marker.color,
                speed: marker.logical_data.get(logical).unwrap().speed,
                key: layout.get_marker_key(logical, target_id),
            };
            leg_markers.push(route_marker);
        }
        let leg = RouteLeg {
            section: section,
            markers: leg_markers,
            index: 0,
            intention: LegIntention::Pass,
        };
        route.push_leg(leg);
    }
    println!("legs: {:?}", route.legs.len());
    route
}

pub enum TrainState {
    Stop,
    Run { facing: Facing, speed: MarkerSpeed },
}

#[derive(Debug)]
pub struct Route {
    legs: Vec<RouteLeg>,
}

impl Route {
    pub fn new() -> Self {
        Route { legs: vec![] }
    }

    pub fn add_leg_from_section(&mut self, section: LogicalSection) {
        let mut markers = vec![];
        self.push_leg(RouteLeg {
            section: section,
            markers: markers,
            index: 0,
            intention: LegIntention::Pass,
        });
    }

    pub fn push_leg(&mut self, leg: RouteLeg) {
        self.legs.push(leg);
    }

    pub fn next_leg(&mut self) {
        self.legs.remove(0);
    }

    pub fn get_current_leg(&self) -> &RouteLeg {
        &self.legs[0]
    }

    pub fn get_current_leg_mut(&mut self) -> &mut RouteLeg {
        &mut self.legs[0]
    }

    pub fn advance_sensor(&mut self) {
        let current_leg = self.get_current_leg_mut();
        current_leg.advance_marker();
        if current_leg.is_completed() {
            self.next_leg();
        }
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
enum LegIntention {
    Pass,
    Stop,
}

#[derive(Debug)]
pub struct RouteLeg {
    section: LogicalSection,
    markers: Vec<RouteMarkerData>,
    index: usize,
    intention: LegIntention,
}

impl RouteLeg {
    fn is_completed(&self) -> bool {
        // panic if we try to advance a completed leg
        if self.index >= self.markers.len() {
            panic!("this route leg is fucked honestly {:?}", self.index);
        }
        self.index == self.markers.len() - 1
    }

    fn advance_marker(&mut self) {
        if self.index < self.markers.len() - 1 {
            self.index += 1;
        } else {
            panic!("Can't advance completed leg {:?}!", self.index);
        }
    }

    fn has_entered(&self) -> bool {
        for (i, marker) in self.markers.iter().enumerate().rev() {
            if marker.key == MarkerKey::Enter {
                return false;
            }
            if i == self.index {
                return true;
            }
        }
        return true;
    }

    fn get_last_marker(&self) -> &RouteMarkerData {
        self.markers.get(self.index).unwrap()
    }

    fn get_train_state(&self) -> TrainState {
        let should_stop = self.intention == LegIntention::Stop;

        let speed = if should_stop && self.has_entered() {
            MarkerSpeed::Slow
        } else {
            self.get_last_marker().speed
        };
        TrainState::Run {
            facing: self.get_final_facing(),
            speed: speed,
        }
    }

    fn is_flip_type(&self) -> bool {
        if self.section.len() < 2 {
            return false;
        }
        self.section.tracks.get(0).unwrap().reversed() == *self.section.tracks.get(1).unwrap()
    }

    fn get_final_facing(&self) -> Facing {
        self.section.tracks.last().unwrap().facing
    }
}
