use std::{fs::File, io::Read, path::Path};

use bevy::prelude::*;
use sha2::Digest;

pub fn bresenham_line(start: (i32, i32), stop: (i32, i32)) -> Vec<(i32, i32)> {
    if start == stop {
        return vec![];
    }
    let mut points = vec![];
    let mut current = start;
    let delta = (stop.0 - start.0, stop.1 - start.1);

    if delta.0 == 0 {
        while current.1 != stop.1 {
            current.1 += delta.1.signum();
            points.push(current);
        }
        return points;
    }

    let ybyx = delta.1 as f32 / delta.0 as f32;
    let mut dist = 0.0;

    while current != stop {
        current.0 += delta.0.signum();
        dist += ybyx * delta.0.signum() as f32;
        points.push(current);
        while dist.abs() > 0.5 {
            current.1 += dist.signum() as i32;
            dist -= dist.signum();
            points.push(current);
        }
    }

    points
}

pub fn distance_to_segment(pos: Vec2, p1: Vec2, p2: Vec2) -> f32 {
    // wikipedia
    let t = (pos - p1).dot(p2 - p1) / (p2 - p1).length_squared();
    let t = t.clamp(0.0, 1.0);
    (p1 + (p2 - p1) * t - pos).length()

    // ((p2.x - p1.x) * (p1.y - p0.y) - (p1.x - p0.x) * (p2.y - p1.y)).abs()
    //    / ((p2.x - p1.x).powi(2) + (p2.y - p1.y).powi(2)).sqrt()
}

pub fn get_file_hash(path: &Path) -> String {
    let mut file = File::open(path).unwrap();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).unwrap();
    let mut hasher = sha2::Sha256::new();
    hasher.update(&buffer);
    let result = hasher.finalize();
    format!("{:x}", result)
}
