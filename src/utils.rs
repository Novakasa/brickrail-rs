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
