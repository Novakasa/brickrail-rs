use bevy::prelude::*;
use itertools::Itertools;

use crate::block::Block;
use crate::layout::EntityMap;
use crate::layout::MarkerMap;
use crate::layout::TrackLocks;
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

impl RouteMarkerData {
    pub fn as_train_u8(&self, override_enter_key: bool) -> u8 {
        let speed = self.speed.as_train_u8();
        let color = self.color.as_train_u8();
        let key = if override_enter_key {
            MarkerKey::Enter.as_train_u8()
        } else {
            self.key.as_train_u8()
        };
        (speed << 6) | color | (key << 4)
    }
}

pub fn build_route(
    train_id: TrainID,
    logical_section: &LogicalSection,
    q_markers: &Query<&Marker>,
    q_blocks: &Query<&Block>,
    entity_map: &EntityMap,
    marker_map: &MarkerMap,
) -> Route {
    let mut route = Route::new(train_id);
    let in_tracks = marker_map.in_markers.keys().collect_vec();
    let split = logical_section.split_by_tracks_with_overlap(in_tracks);
    assert!(split.len() > 0);

    for (section, in_track) in split {
        let target_id = marker_map.in_markers.get(&in_track).unwrap();
        let mut leg_markers = Vec::new();
        let target_block = q_blocks
            .get(entity_map.blocks.get(&target_id.block).unwrap().clone())
            .unwrap();
        let target_section = target_block.get_logical_section(target_id.clone());

        for logical in section.tracks.iter() {
            println!("looking for marker at {:?}", logical);
            if let Some(entity) = entity_map.markers.get(&logical.track()) {
                println!("found marker at {:?}", logical);
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
            intention: LegIntention::Stop,
            section_position: 0.0,
            target_block: target_id.clone(),
            to_section: target_section,
        };
        route.push_leg(leg);
    }
    route.get_current_leg_mut().set_completed();
    println!(
        "legs: {:?}, {:?}",
        route.legs.len(),
        route.get_current_leg().markers
    );
    route
}

#[derive(Debug, Default, Clone)]
pub enum TrainState {
    #[default]
    Stop,
    Run {
        facing: Facing,
        speed: MarkerSpeed,
    },
}

impl TrainState {
    pub fn get_speed(&self) -> f32 {
        match self {
            TrainState::Stop => 0.0,
            TrainState::Run { speed, .. } => speed.get_speed(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Route {
    legs: Vec<RouteLeg>,
    train_id: TrainID,
    leg_index: usize,
}

impl Route {
    pub fn new(id: TrainID) -> Self {
        Route {
            legs: vec![],
            train_id: id,
            leg_index: 0,
        }
    }

    pub fn num_legs(&self) -> usize {
        self.legs.len()
    }

    pub fn iter_legs(&self) -> std::slice::Iter<RouteLeg> {
        self.legs.iter()
    }

    pub fn iter_legs_remaining(&self) -> std::slice::Iter<RouteLeg> {
        self.legs[self.leg_index..].iter()
    }

    pub fn push_leg(&mut self, leg: RouteLeg) {
        self.legs.push(leg);
    }

    pub fn next_leg(&mut self) {
        assert_ne!(self.legs.len(), self.leg_index + 1, "No next leg!");
        self.leg_index += 1;
    }

    pub fn get_current_leg(&self) -> &RouteLeg {
        &self.legs[self.leg_index]
    }

    pub fn get_next_leg(&self) -> Option<&RouteLeg> {
        self.legs.get(self.leg_index + 1)
    }

    pub fn get_current_leg_mut(&mut self) -> &mut RouteLeg {
        &mut self.legs[self.leg_index]
    }

    pub fn update_intentions(&mut self, track_locks: &TrackLocks) {
        let mut free_until = 0;
        for (i, leg) in self.iter_legs_remaining().enumerate() {
            if track_locks.can_lock(&self.train_id, &leg.section)
                && track_locks.can_lock(&self.train_id, &leg.to_section)
            {
                free_until = i + self.leg_index;
            } else {
                break;
            }
        }
        for (i, leg) in self.legs.iter_mut().enumerate() {
            if i < free_until {
                leg.intention = LegIntention::Pass;
            } else {
                leg.intention = LegIntention::Stop;
            }
        }
    }

    pub fn update_locks(&self, track_locks: &mut TrackLocks) {
        let current_leg = self.get_current_leg();
        track_locks.unlock_all(&self.train_id);
        if current_leg.get_leg_state() != LegState::Completed {
            track_locks.lock(&self.train_id, &current_leg.section);
        }
        track_locks.lock(&self.train_id, &current_leg.to_section);
        if let Some(next_leg) = self.get_next_leg() {
            if current_leg.get_leg_state() != LegState::None
                && current_leg.intention == LegIntention::Pass
            {
                track_locks.lock(&self.train_id, &next_leg.section);
                track_locks.lock(&self.train_id, &next_leg.to_section);
            }
        }
    }

    pub fn advance_sensor(&mut self) {
        let current_leg = self.get_current_leg_mut();
        current_leg.advance_marker();
        if current_leg.get_leg_state() == LegState::Completed {
            self.next_leg();
        }
    }

    pub fn get_train_state(&self) -> TrainState {
        let mut will_turn = false;
        if let Some(next_leg) = self.get_next_leg() {
            if next_leg.is_flip() {
                will_turn = true;
            }
        }
        self.get_current_leg().get_train_state(will_turn)
    }

    pub fn advance_distance(&mut self, distance: f32) -> bool {
        let mut change_locks = false;
        let mut remainder = Some(distance);
        while remainder.is_some() {
            let leg_state = self.get_current_leg().get_leg_state();
            remainder = self
                .get_current_leg_mut()
                .advance_distance(remainder.unwrap());
            if self.get_current_leg().get_leg_state() != leg_state {
                change_locks = true;
            }
            if let Some(_) = remainder {
                self.next_leg();
                change_locks = true;
                if self.legs.len() == 0 {
                    break;
                }
            }
        }
        return change_locks;
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

impl LegIntention {
    pub fn as_train_flag(&self) -> u8 {
        match self {
            LegIntention::Pass => 0,
            LegIntention::Stop => 1,
        }
    }
}

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum LegState {
    None,
    Entered,
    Completed,
}

#[derive(Debug, Clone)]
pub struct RouteLeg {
    to_section: LogicalSection,
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
        if self.markers.len() == 1 {
            return 0;
        }
        return self.markers.len() - 2;
    }

    fn advance_marker(&mut self) {
        if self.index < self.markers.len() - 1 {
            self.index += 1;
        } else {
            panic!("Can't advance completed leg {:?}!", self.index);
        }
    }

    pub fn get_leg_state(&self) -> LegState {
        if self.index >= self.markers.len() {
            panic!(
                "Invalid index {} for leg with {} markers",
                self.index,
                self.markers.len()
            );
        }
        if self.index == self.markers.len() - 1 {
            return LegState::Completed;
        }
        if self.index >= self.get_enter_index() {
            return LegState::Entered;
        }
        return LegState::None;
    }

    fn get_previous_marker(&self) -> &RouteMarkerData {
        self.markers.get(self.index).unwrap()
    }

    fn get_train_state(&self, will_turn: bool) -> TrainState {
        let should_stop = self.intention == LegIntention::Stop;
        let leg_state = self.get_leg_state();

        if should_stop && leg_state == LegState::Completed {
            return TrainState::Stop;
        }

        let speed = if (should_stop || will_turn) && leg_state == LegState::Entered {
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

    fn is_flip(&self) -> bool {
        self.section.tracks[0].facing != self.get_final_facing()
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
        if self.get_leg_state() == LegState::Completed {
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
            if self.get_leg_state() == LegState::Completed {
                if self.intention == LegIntention::Stop {
                    return None;
                }
                return Some(remainder);
            }
        }
        self.section_position += remainder;
        None
    }

    pub fn as_train_data(&self) -> Vec<u8> {
        let mut data = Vec::new();
        for (i, marker) in self.markers.iter().enumerate() {
            data.push(marker.as_train_u8(i == self.get_enter_index()));
        }
        data.push(self.intention.as_train_flag() | self.get_final_facing().as_train_flag());
        data
    }
}
