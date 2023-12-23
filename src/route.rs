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
    let split = logical_section.split_by_tracks_with_overlap(in_tracks);

    for (section, in_track) in split {
        let target_id = layout.in_markers.get(&in_track).unwrap();
        let mut leg_markers = Vec::new();

        for logical in section.tracks.iter() {
            println!("looking for marker at {:?}", logical);
            if let Some(entity) = layout.markers.get(&logical.track()) {
                let marker = q_markers.get(*entity).unwrap();
                let route_marker = RouteMarkerData {
                    track: logical.clone(),
                    color: marker.color,
                    speed: marker.logical_data.get(logical).unwrap().speed,
                    key: layout.get_marker_key(logical, target_id),
                };
                leg_markers.push(route_marker);
            }
        }
        let leg = RouteLeg {
            section: section,
            markers: leg_markers,
            index: 0,
            intention: LegIntention::Pass,
            section_position: 0.0,
        };
        route.push_leg(leg);
    }
    route.get_current_leg_mut().set_completed();
    route.legs.last_mut().unwrap().intention = LegIntention::Stop;
    println!(
        "legs: {:?}, {:?}",
        route.legs.len(),
        route.get_current_leg().markers
    );
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
    section_position: f32,
}

impl RouteLeg {
    fn get_enter_index(&self) -> usize {
        for (i, marker) in self.markers.iter().enumerate() {
            if marker.key == MarkerKey::Enter {
                return i;
            }
        }
        return self.markers.len() - 2;
    }

    fn is_completed(&self) -> bool {
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
        return self.index >= self.get_enter_index();
    }

    fn get_last_marker(&self) -> &RouteMarkerData {
        self.markers.get(self.index).unwrap()
    }

    fn get_train_state(&self) -> TrainState {
        let should_stop = self.intention == LegIntention::Stop;

        if should_stop && self.is_completed() {
            return TrainState::Stop;
        }

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

    fn get_final_facing(&self) -> Facing {
        self.section.tracks.last().unwrap().facing
    }

    fn set_completed(&mut self) {
        self.index = self.markers.len() - 1;
        self.section_position = self.section.length_to(&self.markers.last().unwrap().track);
    }

    pub fn get_current_pos(&self) -> Vec2 {
        self.section.interpolate_pos(self.section_position)
    }
}
