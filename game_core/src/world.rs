use crate::flow_field::FlowFieldSet;
use crate::input::{InputBus, UnitType};
use crate::types::{FactionId, MatchConfig, TeamId};
use crate::TICK_DT;

pub const MAP_WIDTH: f32 = 960.0;
pub const MAP_HEIGHT: f32 = 640.0;

const CASTLE_HP: f32 = 1200.0;
const BASE_INCOME: f32 = 8.0;
const POINT_INCOME: f32 = 6.0;
const CAPTURE_RATE: f32 = 25.0;
const CASTLE_ATTACK_RANGE: f32 = 28.0;
const CASTLE_DPS: f32 = 35.0;

#[derive(Clone, Debug)]
pub struct Castle {
    pub faction: FactionId,
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub max_hp: f32,
}

impl Castle {
    pub fn alive(&self) -> bool {
        self.hp > 0.0
    }
}

#[derive(Clone, Debug)]
pub struct CapturePoint {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub owner: Option<FactionId>,
    pub progress: f32,
    pub capturing: Option<FactionId>,
}

#[derive(Clone, Debug)]
pub struct SimUnit {
    pub id: u32,
    pub faction: FactionId,
    pub unit_type: UnitType,
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub max_hp: f32,
    pub attack_cd: f32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VictoryState {
    Ongoing,
    TeamWon(TeamId),
}

#[derive(Clone, Debug)]
pub struct TickResult {
    pub victory: VictoryState,
    pub units_spawned: u32,
}

#[derive(Clone, Debug)]
pub struct SimWorld {
    pub config: MatchConfig,
    pub tick: u32,
    pub gold: Vec<f32>,
    pub castles: Vec<Castle>,
    pub points: Vec<CapturePoint>,
    pub units: Vec<SimUnit>,
    pub obstacles: Vec<(f32, f32, f32, f32)>,
    pub flows: FlowFieldSet,
    next_unit_id: u32,
    spawn_jitter: u32,
}

impl SimWorld {
    pub fn new(config: MatchConfig) -> Self {
        let n = config.faction_count();
        let spawns = spawn_positions(n);
        let mut castles = Vec::with_capacity(n);
        for (i, &(x, y)) in spawns.iter().enumerate() {
            castles.push(Castle {
                faction: FactionId(i as u8),
                x,
                y,
                hp: CASTLE_HP,
                max_hp: CASTLE_HP,
            });
        }

        let points = default_capture_points(n);
        let obstacles = default_obstacles();
        let flows = FlowFieldSet::new(&obstacles);

        Self {
            gold: vec![40.0; n],
            config,
            tick: 0,
            castles,
            points,
            units: Vec::with_capacity(512),
            obstacles,
            flows,
            next_unit_id: 1,
            spawn_jitter: 0,
        }
    }

    pub fn team_of(&self, faction: FactionId) -> TeamId {
        self.config.team_for(faction)
    }

    pub fn are_allied(&self, a: FactionId, b: FactionId) -> bool {
        self.team_of(a) == self.team_of(b)
    }

    pub fn tick(&mut self, bus: &InputBus) -> TickResult {
        self.tick = self.tick.wrapping_add(1);
        let mut spawned = 0u32;

        self.apply_income();
        spawned += self.apply_production(bus);
        self.update_units_movement();
        self.update_combat();
        self.update_capture_points();
        let victory = self.check_victory();

        TickResult {
            victory,
            units_spawned: spawned,
        }
    }

    fn apply_income(&mut self) {
        let n = self.config.faction_count();
        for i in 0..n {
            self.gold[i] += BASE_INCOME * TICK_DT;
        }
        for p in &self.points {
            if let Some(owner) = p.owner {
                if owner.index() < n {
                    self.gold[owner.index()] += POINT_INCOME * TICK_DT;
                }
            }
        }
    }

    fn apply_production(&mut self, bus: &InputBus) -> u32 {
        let n = self.config.faction_count();
        let mut spawned = 0u32;
        for i in 0..n {
            let faction = FactionId(i as u8);
            let Some(castle) = self.castles.iter().find(|c| c.faction == faction && c.alive())
            else {
                continue;
            };
            let (cx, cy) = (castle.x, castle.y);
            let input = bus.get(faction);
            if let Some(ut) = input.produce {
                let cost = ut.cost();
                if self.gold[i] >= cost {
                    self.gold[i] -= cost;
                    self.spawn_jitter = self.spawn_jitter.wrapping_add(1);
                    let j = (self.spawn_jitter % 7) as f32 - 3.0;
                    let angle = (self.spawn_jitter as f32) * 0.7;
                    let sx = cx + angle.cos() * 28.0 + j;
                    let sy = cy + angle.sin() * 28.0;
                    self.units.push(SimUnit {
                        id: self.next_unit_id,
                        faction,
                        unit_type: ut,
                        x: sx.clamp(8.0, MAP_WIDTH - 8.0),
                        y: sy.clamp(8.0, MAP_HEIGHT - 8.0),
                        hp: ut.hp(),
                        max_hp: ut.hp(),
                        attack_cd: 0.0,
                    });
                    self.next_unit_id = self.next_unit_id.wrapping_add(1);
                    spawned += 1;
                }
            }
        }
        spawned
    }

    fn primary_goal(&self, faction: FactionId) -> (f32, f32) {
        let team = self.team_of(faction);
        let mut best_pt: Option<(f32, f32, f32)> = None;
        if let Some(castle) = self.castles.iter().find(|c| c.faction == faction && c.alive()) {
            for p in &self.points {
                let enemy_or_neutral = match p.owner {
                    None => true,
                    Some(o) => self.team_of(o) != team,
                };
                if enemy_or_neutral {
                    let d = dist2(castle.x, castle.y, p.x, p.y);
                    if best_pt.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                        best_pt = Some((p.x, p.y, d));
                    }
                }
            }
            if let Some((x, y, _)) = best_pt {
                let owned = self
                    .points
                    .iter()
                    .filter(|p| p.owner.map(|o| self.team_of(o) == team).unwrap_or(false))
                    .count();
                if owned >= self.points.len().saturating_sub(1) {
                    return self.nearest_enemy_castle(faction).unwrap_or((x, y));
                }
                return (x, y);
            }
        }
        self.nearest_enemy_castle(faction)
            .unwrap_or((MAP_WIDTH * 0.5, MAP_HEIGHT * 0.5))
    }

    fn nearest_enemy_castle(&self, faction: FactionId) -> Option<(f32, f32)> {
        let team = self.team_of(faction);
        let origin = self
            .castles
            .iter()
            .find(|c| c.faction == faction)
            .map(|c| (c.x, c.y))?;
        let mut best = None;
        for c in &self.castles {
            if !c.alive() || self.team_of(c.faction) == team {
                continue;
            }
            let d = dist2(origin.0, origin.1, c.x, c.y);
            if best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
                best = Some((c.x, c.y, d));
            }
        }
        best.map(|(x, y, _)| (x, y))
    }

    fn update_units_movement(&mut self) {
        let n = self.config.faction_count();
        let goals: Vec<(f32, f32)> = (0..n)
            .map(|i| self.primary_goal(FactionId(i as u8)))
            .collect();

        let snapshot: Vec<(u32, FactionId, TeamId, f32, f32, f32)> = self
            .units
            .iter()
            .map(|u| {
                (
                    u.id,
                    u.faction,
                    self.config.team_for(u.faction),
                    u.x,
                    u.y,
                    u.unit_type.radius(),
                )
            })
            .collect();

        let obstacles = self.obstacles.clone();

        for unit in &mut self.units {
            let goal = goals[unit.faction.index()];
            let (fx, fy) = {
                let field = self.flows.field_toward(goal.0, goal.1);
                field.sample(unit.x, unit.y)
            };

            let mut vx = fx;
            let mut vy = fy;
            let my_team = self.config.team_for(unit.faction);

            for &(id, _f, _t, ox, oy, r) in &snapshot {
                if id == unit.id {
                    continue;
                }
                let dx = unit.x - ox;
                let dy = unit.y - oy;
                let d2 = dx * dx + dy * dy;
                let min_d = unit.unit_type.radius() + r;
                if d2 < min_d * min_d && d2 > 0.01 {
                    let d = d2.sqrt();
                    let push = (min_d - d) / min_d;
                    vx += (dx / d) * push * 1.5;
                    vy += (dy / d) * push * 1.5;
                }
            }

            if let Some((ex, ey)) = nearest_enemy_pos(my_team, unit.x, unit.y, &snapshot) {
                let dx = ex - unit.x;
                let dy = ey - unit.y;
                let d = (dx * dx + dy * dy).sqrt().max(0.1);
                if d < unit.unit_type.range() * 1.8 {
                    vx = dx / d;
                    vy = dy / d;
                }
            }

            let len = (vx * vx + vy * vy).sqrt();
            if len > 0.001 {
                vx /= len;
                vy /= len;
                let speed = unit.unit_type.speed();
                unit.x = (unit.x + vx * speed * TICK_DT).clamp(4.0, MAP_WIDTH - 4.0);
                unit.y = (unit.y + vy * speed * TICK_DT).clamp(4.0, MAP_HEIGHT - 4.0);
                resolve_obstacles(unit, &obstacles);
            }
        }
    }

    fn update_combat(&mut self) {
        let n = self.units.len();
        for i in 0..n {
            if self.units[i].attack_cd > 0.0 {
                self.units[i].attack_cd -= TICK_DT;
            }
        }

        let alliances: Vec<TeamId> = self
            .units
            .iter()
            .map(|u| self.config.team_for(u.faction))
            .collect();
        let castle_teams: Vec<TeamId> = self
            .castles
            .iter()
            .map(|c| self.config.team_for(c.faction))
            .collect();

        let mut unit_dmg: Vec<(usize, f32)> = Vec::new();
        let mut castle_dmg: Vec<(usize, f32)> = Vec::new();
        let mut attackers: Vec<usize> = Vec::new();

        for i in 0..n {
            if self.units[i].attack_cd > 0.0 {
                continue;
            }
            let ax = self.units[i].x;
            let ay = self.units[i].y;
            let range = self.units[i].unit_type.range();
            let dmg = self.units[i].unit_type.damage();
            let my_team = alliances[i];
            let mut did_attack = false;

            for j in 0..n {
                if i == j || alliances[j] == my_team {
                    continue;
                }
                if dist(ax, ay, self.units[j].x, self.units[j].y) <= range {
                    unit_dmg.push((j, dmg));
                    did_attack = true;
                    break;
                }
            }

            if !did_attack {
                for (ci, castle) in self.castles.iter().enumerate() {
                    if !castle.alive() || castle_teams[ci] == my_team {
                        continue;
                    }
                    if dist(ax, ay, castle.x, castle.y) <= CASTLE_ATTACK_RANGE.max(range) {
                        castle_dmg.push((ci, dmg));
                        did_attack = true;
                        break;
                    }
                }
            }

            if did_attack {
                attackers.push(i);
            }
        }

        for (j, dmg) in unit_dmg {
            if j < self.units.len() {
                self.units[j].hp -= dmg;
            }
        }
        for (ci, dmg) in castle_dmg {
            if ci < self.castles.len() {
                self.castles[ci].hp -= dmg;
            }
        }
        for i in attackers {
            if i < self.units.len() {
                self.units[i].attack_cd = 0.45;
            }
        }

        let castle_shots: Vec<(TeamId, f32, f32)> = self
            .castles
            .iter()
            .zip(castle_teams.iter())
            .filter(|(c, _)| c.alive())
            .map(|(c, t)| (*t, c.x, c.y))
            .collect();

        for (ct, cx, cy) in castle_shots {
            for unit in &mut self.units {
                if self.config.team_for(unit.faction) == ct {
                    continue;
                }
                if dist(cx, cy, unit.x, unit.y) <= CASTLE_ATTACK_RANGE {
                    unit.hp -= CASTLE_DPS * TICK_DT;
                }
            }
        }

        self.units.retain(|u| u.hp > 0.0);
        for c in &mut self.castles {
            c.hp = c.hp.max(0.0);
        }
    }

    fn update_capture_points(&mut self) {
        let team_lookup: Vec<TeamId> = (0..self.config.faction_count())
            .map(|i| self.config.team_for(FactionId(i as u8)))
            .collect();
        let team_of = |f: FactionId| -> TeamId {
            team_lookup
                .get(f.index())
                .copied()
                .unwrap_or(TeamId(f.0))
        };

        // Gather presence first (immutable pass)
        let presence_per_point: Vec<Vec<(FactionId, u32)>> = self
            .points
            .iter()
            .map(|point| {
                let mut presence: Vec<(FactionId, u32)> = Vec::new();
                for unit in &self.units {
                    if dist(unit.x, unit.y, point.x, point.y) <= point.radius {
                        if let Some(entry) = presence.iter_mut().find(|(f, _)| *f == unit.faction) {
                            entry.1 += 1;
                        } else {
                            presence.push((unit.faction, 1));
                        }
                    }
                }
                presence
            })
            .collect();

        for (point, mut presence) in self.points.iter_mut().zip(presence_per_point) {
            if presence.is_empty() {
                continue;
            }
            presence.sort_by(|a, b| b.1.cmp(&a.1));
            let (top_f, top_n) = presence[0];
            let contested = presence.len() > 1
                && presence[1].1 == top_n
                && team_of(presence[1].0) != team_of(top_f);
            if contested {
                continue;
            }
            if point.owner == Some(top_f) {
                point.progress = 100.0;
                continue;
            }
            if point.capturing != Some(top_f) {
                point.capturing = Some(top_f);
                point.progress = 0.0;
            }
            point.progress += CAPTURE_RATE * TICK_DT * (top_n as f32).min(5.0);
            if point.progress >= 100.0 {
                point.owner = Some(top_f);
                point.progress = 100.0;
            }
        }
    }

    fn check_victory(&self) -> VictoryState {
        let n = self.config.faction_count();
        let mut team_alive = [false; 8];
        for i in 0..n {
            let f = FactionId(i as u8);
            let team = self.team_of(f);
            if self.castles.iter().any(|c| c.faction == f && c.alive()) {
                team_alive[team.0 as usize] = true;
            }
        }
        let alive_teams: Vec<u8> = (0..8u8).filter(|&t| team_alive[t as usize]).collect();
        if alive_teams.len() == 1 {
            VictoryState::TeamWon(TeamId(alive_teams[0]))
        } else {
            VictoryState::Ongoing
        }
    }
}

fn spawn_positions(n: usize) -> Vec<(f32, f32)> {
    match n {
        2 => vec![
            (120.0, MAP_HEIGHT * 0.5),
            (MAP_WIDTH - 120.0, MAP_HEIGHT * 0.5),
        ],
        4 => vec![
            (110.0, 110.0),
            (110.0, MAP_HEIGHT - 110.0),
            (MAP_WIDTH - 110.0, 110.0),
            (MAP_WIDTH - 110.0, MAP_HEIGHT - 110.0),
        ],
        _ => {
            let mut v = Vec::new();
            for i in 0..n {
                let a = (i as f32 / n as f32) * std::f32::consts::TAU;
                v.push((
                    MAP_WIDTH * 0.5 + a.cos() * 320.0,
                    MAP_HEIGHT * 0.5 + a.sin() * 220.0,
                ));
            }
            v
        }
    }
}

fn default_capture_points(n: usize) -> Vec<CapturePoint> {
    let mut pts = vec![
        CapturePoint {
            x: MAP_WIDTH * 0.5,
            y: MAP_HEIGHT * 0.5,
            radius: 42.0,
            owner: None,
            progress: 0.0,
            capturing: None,
        },
        CapturePoint {
            x: MAP_WIDTH * 0.35,
            y: MAP_HEIGHT * 0.35,
            radius: 36.0,
            owner: None,
            progress: 0.0,
            capturing: None,
        },
        CapturePoint {
            x: MAP_WIDTH * 0.65,
            y: MAP_HEIGHT * 0.65,
            radius: 36.0,
            owner: None,
            progress: 0.0,
            capturing: None,
        },
        CapturePoint {
            x: MAP_WIDTH * 0.35,
            y: MAP_HEIGHT * 0.65,
            radius: 36.0,
            owner: None,
            progress: 0.0,
            capturing: None,
        },
        CapturePoint {
            x: MAP_WIDTH * 0.65,
            y: MAP_HEIGHT * 0.35,
            radius: 36.0,
            owner: None,
            progress: 0.0,
            capturing: None,
        },
    ];
    if n <= 2 {
        pts.truncate(3);
        pts[1].x = MAP_WIDTH * 0.5;
        pts[1].y = MAP_HEIGHT * 0.28;
        pts[2].x = MAP_WIDTH * 0.5;
        pts[2].y = MAP_HEIGHT * 0.72;
    }
    pts
}

fn default_obstacles() -> Vec<(f32, f32, f32, f32)> {
    vec![
        (MAP_WIDTH * 0.5, MAP_HEIGHT * 0.18, 40.0, 120.0),
        (MAP_WIDTH * 0.5, MAP_HEIGHT * 0.82, 40.0, 120.0),
    ]
}

fn dist(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    dist2(ax, ay, bx, by).sqrt()
}

fn dist2(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;
    dx * dx + dy * dy
}

fn nearest_enemy_pos(
    my_team: TeamId,
    ux: f32,
    uy: f32,
    snapshot: &[(u32, FactionId, TeamId, f32, f32, f32)],
) -> Option<(f32, f32)> {
    let mut best = None;
    for &(_id, _f, team, x, y, _) in snapshot {
        if team == my_team {
            continue;
        }
        let d = dist2(ux, uy, x, y);
        if best.map(|(_, _, bd)| d < bd).unwrap_or(true) {
            best = Some((x, y, d));
        }
    }
    best.map(|(x, y, _)| (x, y))
}

fn resolve_obstacles(unit: &mut SimUnit, obstacles: &[(f32, f32, f32, f32)]) {
    for &(ox, oy, ow, oh) in obstacles {
        let hw = ow * 0.5;
        let hh = oh * 0.5;
        let left = ox - hw;
        let right = ox + hw;
        let bottom = oy - hh;
        let top = oy + hh;
        if unit.x > left && unit.x < right && unit.y > bottom && unit.y < top {
            let dl = unit.x - left;
            let dr = right - unit.x;
            let db = unit.y - bottom;
            let dt = top - unit.y;
            let m = dl.min(dr).min(db).min(dt);
            if m == dl {
                unit.x = left;
            } else if m == dr {
                unit.x = right;
            } else if m == db {
                unit.y = bottom;
            } else {
                unit.y = top;
            }
        }
    }
}
