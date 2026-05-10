use std::collections::VecDeque;

use bevy::prelude::*;

use crate::driver::{DriverLeg, DriverMarkerHit, QueueDriverLeg};
use crate::layout_primitives::TrainID;

/// Virtual train driver component. Lives on its own entity (not the train entity).
/// Simulates a train advancing through driver legs by tracking continuous position
/// and emitting `DriverMarkerHit` events.
///
/// Consumes `QueueDriverLeg` messages and maintains its own internal leg queue.
/// Fully decoupled from simulation state — communicates only through the driver interface.
#[derive(Component)]
pub struct VirtualDriver {
    /// Which train this driver controls.
    pub train: TrainID,
    /// Speed in position units per second.
    pub speed: f32,
    /// Continuous position along the current leg (normalized, 0.0 to 1.0).
    position: f32,
    /// Internal queue of legs to drive through.
    legs: VecDeque<DriverLeg>,
    /// Index of the next marker the driver expects to cross in the current leg.
    /// Starts at 1 (marker 0 is the starting position, already passed).
    next_marker_index: usize,
}

impl VirtualDriver {
    pub fn new(train: TrainID, speed: f32) -> Self {
        Self {
            train,
            speed,
            position: 0.0,
            legs: VecDeque::new(),
            next_marker_index: 1,
        }
    }
}

/// Handles `QueueDriverLeg` messages by appending legs to the matching virtual driver.
fn handle_queue_driver_leg(
    mut messages: MessageReader<QueueDriverLeg>,
    mut driver_query: Query<&mut VirtualDriver>,
) {
    for msg in messages.read() {
        for mut driver in &mut driver_query {
            if driver.train == msg.train {
                driver.legs.push_back(msg.leg.clone());
            }
        }
    }
}

/// Advances virtual drivers each tick. Moves position forward and emits
/// `DriverMarkerHit` when crossing marker positions. When all markers in
/// the current leg are passed, advances to the next leg in the queue.
fn virtual_driver_tick(
    time: Res<Time>,
    mut driver_query: Query<&mut VirtualDriver>,
    mut marker_hit_writer: MessageWriter<DriverMarkerHit>,
) {
    let dt = time.delta_secs();

    for mut driver in &mut driver_query {
        // All markers passed on current leg — try to advance
        if let Some(current_leg) = driver.legs.front() {
            if driver.next_marker_index >= current_leg.markers.len() {
                if driver.legs.len() > 1 {
                    driver.legs.pop_front();
                    driver.position = 0.0;
                    driver.next_marker_index = 1;
                } else {
                    continue;
                }
            }
        } else {
            continue;
        }

        // Advance position
        driver.position += driver.speed * dt;

        // Check all markers we might have crossed this tick
        let train = driver.train;
        loop {
            let marker_pos = driver
                .legs
                .front()
                .and_then(|leg| leg.markers.get(driver.next_marker_index))
                .map(|m| m.position);
            match marker_pos {
                Some(pos) if driver.position >= pos => {
                    marker_hit_writer.write(DriverMarkerHit::new(train));
                    driver.next_marker_index += 1;
                }
                _ => break,
            }
        }
    }
}

/// Plugin for the virtual train driver.
/// Registers `QueueDriverLeg` message handling and the driver tick system.
/// The simulation layer must separately register `DriverMarkerHit` (done in `SimulationStatePlugin`).
pub struct VirtualDriverPlugin;

impl Plugin for VirtualDriverPlugin {
    fn build(&self, app: &mut App) {
        // QueueDriverLeg and DriverMarkerHit are registered by SimulationStatePlugin.
        app.add_systems(
            Update,
            (
                handle_queue_driver_leg.run_if(on_message::<QueueDriverLeg>),
                virtual_driver_tick,
            )
                .chain(),
        );
    }
}
