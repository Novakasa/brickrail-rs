use bevy::prelude::*;
use itertools::Itertools;

use crate::layout::EntityMap;
use crate::layout::MarkerMap;
use crate::layout_primitives::*;
use crate::marker::*;
use crate::section::LogicalSection;
use crate::track::LAYOUT_SCALE;

#[derive(Debug, Clone)]
pub struct RouteMarkerData {
    pub track: LogicalTrackID,
    pub color: MarkerColor,
    pub speed: MarkerSpeed,
    pub key: MarkerKey,
    pub position: f32,
}

pub fn build_route(
    logical_section: &LogicalSection,
    q_markers: &Query<&Marker>,
    entity_map: &EntityMap,
    marker_map: &MarkerMap,
) -> Route {
    let mut route = Route::new();
    let in_tracks = marker_map.in_markers.keys().collect_vec();
    let split = logical_section.split_by_tracks_with_overlap(in_tracks);

    for (section, in_track) in split {
        let target_id = marker_map.in_markers.get(&in_track).unwrap();
        let mut leg_markers = Vec::new();

        for logical in section.tracks.iter() {
            println!("looking for marker at {:?}", logical);
            if let Some(entity) = entity_map.markers.get(&logical.track()) {
                let marker = q_markers.get(*entity).unwrap();
                let route_marker = RouteMarkerData {
                    track: logical.clone(),
                    color: marker.color,
                    speed: marker.logical_data.get(logical).unwrap().speed,
                    key: marker_map.get_marker_key(logical, target_id),
                    position: section.length_to(&logical),
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
            target_block: target_id.clone(),
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

#[derive(Debug)]
pub enum TrainState {
    Stop,
    Run { facing: Facing, speed: MarkerSpeed },
}

impl TrainState {
    pub fn get_speed(&self) -> f32 {
        match self {
            TrainState::Stop => 0.0,
            TrainState::Run { speed, .. } => speed.get_speed(),
        }
    }
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
        if self.legs.len() == 1 {
            panic!("Can't advance with single leg!");
        }
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

    pub fn get_train_state(&self) -> TrainState {
        self.get_current_leg().get_train_state()
    }

    pub fn advance_distance(&mut self, distance: f32) {
        let mut remainder = Some(distance);
        while remainder.is_some() {
            remainder = self
                .get_current_leg_mut()
                .advance_distance(remainder.unwrap());
            if let Some(_) = remainder {
                self.next_leg();
                if self.legs.len() == 0 {
                    break;
                }
            }
        }
    }

    pub fn draw_with_gizmos(&self, gizmos: &mut Gizmos) {
        for leg in self.legs.iter() {
            for track in leg.section.tracks.iter() {
                track
                    .dirtrack
                    .draw_with_gizmos(gizmos, LAYOUT_SCALE, Color::GREEN);
            }
        }
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum LegIntention {
    Pass,
    Stop,
}

#[derive(Debug)]
pub struct RouteLeg {
    section: LogicalSection,
    markers: Vec<RouteMarkerData>,
    index: usize,
    pub intention: LegIntention,
    pub section_position: f32,
    target_block: LogicalBlockID,
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

    fn get_previous_marker(&self) -> &RouteMarkerData {
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
            self.get_previous_marker().speed
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
        self.section_position = self.get_previous_marker_pos();
    }

    pub fn get_current_pos(&self) -> Vec2 {
        self.section.interpolate_pos(self.section_position)
    }

    pub fn get_target_block_id(&self) -> LogicalBlockID {
        self.target_block.clone()
    }

    pub fn get_next_marker_pos(&self) -> f32 {
        self.markers[self.index + 1].position
    }

    pub fn get_previous_marker_pos(&self) -> f32 {
        self.markers[self.index].position
    }

    pub fn advance_distance(&mut self, distance: f32) -> Option<f32> {
        if self.is_completed() {
            if self.intention == LegIntention::Stop {
                return None;
            }
            return Some(distance);
        }
        let mut remainder = distance;
        while self.section_position + remainder > self.get_next_marker_pos() {
            remainder -= self.get_next_marker_pos() - self.section_position;
            self.section_position = self.get_next_marker_pos();
            self.advance_marker();
            if self.is_completed() {
                if self.intention == LegIntention::Stop {
                    return None;
                }
                return Some(remainder);
            }
        }
        self.section_position += remainder;
        None
    }
}
