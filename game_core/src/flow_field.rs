//! Grid flow fields: one field per goal cell for cheap mass pathfollowing.

use crate::world::{MAP_HEIGHT, MAP_WIDTH};

pub const CELL: f32 = 20.0;
pub const GRID_W: usize = (MAP_WIDTH / CELL) as usize; // 48
pub const GRID_H: usize = (MAP_HEIGHT / CELL) as usize; // 32

#[derive(Clone, Debug)]
pub struct FlowField {
    /// Direction toward goal per cell (normalized), zero if blocked/unreachable.
    pub dir_x: Vec<f32>,
    pub dir_y: Vec<f32>,
    pub cost: Vec<f32>,
}

impl FlowField {
    pub fn empty() -> Self {
        let n = GRID_W * GRID_H;
        Self {
            dir_x: vec![0.0; n],
            dir_y: vec![0.0; n],
            cost: vec![f32::INFINITY; n],
        }
    }

    pub fn sample(&self, x: f32, y: f32) -> (f32, f32) {
        let gx = (x / CELL).floor() as isize;
        let gy = (y / CELL).floor() as isize;
        if gx < 0 || gy < 0 || gx as usize >= GRID_W || gy as usize >= GRID_H {
            return (0.0, 0.0);
        }
        let i = gy as usize * GRID_W + gx as usize;
        (self.dir_x[i], self.dir_y[i])
    }
}

#[derive(Clone, Debug)]
pub struct FlowFieldSet {
    /// Index = goal cell linear index; lazily filled.
    fields: Vec<Option<FlowField>>,
    walkable: Vec<bool>,
}

impl FlowFieldSet {
    pub fn new(obstacles: &[(f32, f32, f32, f32)]) -> Self {
        let mut walkable = vec![true; GRID_W * GRID_H];
        for &(ox, oy, ow, oh) in obstacles {
            let x0 = ((ox - ow * 0.5) / CELL).floor() as isize;
            let y0 = ((oy - oh * 0.5) / CELL).floor() as isize;
            let x1 = ((ox + ow * 0.5) / CELL).ceil() as isize;
            let y1 = ((oy + oh * 0.5) / CELL).ceil() as isize;
            for gy in y0..y1 {
                for gx in x0..x1 {
                    if gx >= 0 && gy >= 0 && (gx as usize) < GRID_W && (gy as usize) < GRID_H {
                        walkable[gy as usize * GRID_W + gx as usize] = false;
                    }
                }
            }
        }
        Self {
            fields: vec![None; GRID_W * GRID_H],
            walkable,
        }
    }

    pub fn world_to_cell(x: f32, y: f32) -> Option<usize> {
        let gx = (x / CELL).floor() as isize;
        let gy = (y / CELL).floor() as isize;
        if gx < 0 || gy < 0 || gx as usize >= GRID_W || gy as usize >= GRID_H {
            return None;
        }
        Some(gy as usize * GRID_W + gx as usize)
    }

    pub fn field_toward(&mut self, goal_x: f32, goal_y: f32) -> &FlowField {
        let Some(goal) = Self::world_to_cell(goal_x, goal_y) else {
            // Fallback: empty field stored at 0
            if self.fields[0].is_none() {
                self.fields[0] = Some(FlowField::empty());
            }
            return self.fields[0].as_ref().unwrap();
        };

        if self.fields[goal].is_none() {
            self.fields[goal] = Some(self.build_field(goal));
        }
        self.fields[goal].as_ref().unwrap()
    }

    fn build_field(&self, goal: usize) -> FlowField {
        let n = GRID_W * GRID_H;
        let mut cost = vec![f32::INFINITY; n];
        let mut dir_x = vec![0.0; n];
        let mut dir_y = vec![0.0; n];

        if !self.walkable[goal] {
            // Snap to nearest walkable
            if let Some(alt) = self.nearest_walkable(goal) {
                return self.build_field(alt);
            }
            return FlowField {
                dir_x,
                dir_y,
                cost,
            };
        }

        // Dijkstra / BFS integration cost from goal
        let mut queue = std::collections::VecDeque::new();
        cost[goal] = 0.0;
        queue.push_back(goal);

        const NEIGHBORS: [(isize, isize, f32); 8] = [
            (1, 0, 1.0),
            (-1, 0, 1.0),
            (0, 1, 1.0),
            (0, -1, 1.0),
            (1, 1, 1.414),
            (1, -1, 1.414),
            (-1, 1, 1.414),
            (-1, -1, 1.414),
        ];

        while let Some(i) = queue.pop_front() {
            let cx = (i % GRID_W) as isize;
            let cy = (i / GRID_W) as isize;
            for &(dx, dy, step) in &NEIGHBORS {
                let nx = cx + dx;
                let ny = cy + dy;
                if nx < 0 || ny < 0 || nx as usize >= GRID_W || ny as usize >= GRID_H {
                    continue;
                }
                let ni = ny as usize * GRID_W + nx as usize;
                if !self.walkable[ni] {
                    continue;
                }
                let new_cost = cost[i] + step;
                if new_cost < cost[ni] {
                    cost[ni] = new_cost;
                    queue.push_back(ni);
                }
            }
        }

        // Gradient: point to neighbor with lowest cost
        for i in 0..n {
            if !self.walkable[i] || cost[i].is_infinite() {
                continue;
            }
            let cx = (i % GRID_W) as isize;
            let cy = (i / GRID_W) as isize;
            let mut best = cost[i];
            let mut bdx = 0.0f32;
            let mut bdy = 0.0f32;
            for &(dx, dy, _) in &NEIGHBORS {
                let nx = cx + dx;
                let ny = cy + dy;
                if nx < 0 || ny < 0 || nx as usize >= GRID_W || ny as usize >= GRID_H {
                    continue;
                }
                let ni = ny as usize * GRID_W + nx as usize;
                if cost[ni] < best {
                    best = cost[ni];
                    bdx = dx as f32;
                    bdy = dy as f32;
                }
            }
            let len = (bdx * bdx + bdy * bdy).sqrt();
            if len > 0.0 {
                dir_x[i] = bdx / len;
                dir_y[i] = bdy / len;
            }
        }

        FlowField { dir_x, dir_y, cost }
    }

    fn nearest_walkable(&self, from: usize) -> Option<usize> {
        let cx = (from % GRID_W) as isize;
        let cy = (from / GRID_W) as isize;
        for r in 1..16isize {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let nx = cx + dx;
                    let ny = cy + dy;
                    if nx < 0 || ny < 0 || nx as usize >= GRID_W || ny as usize >= GRID_H {
                        continue;
                    }
                    let ni = ny as usize * GRID_W + nx as usize;
                    if self.walkable[ni] {
                        return Some(ni);
                    }
                }
            }
        }
        None
    }
}
