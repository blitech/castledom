use crate::flow_field::FlowFieldSet;
use crate::input::{
    footprint_center, mitigate_damage, BuildingKind, DamageType, InputBus, Targeting, UnitType,
    TILE_SIZE,
};
use crate::types::{FactionId, MatchConfig, TeamId};
use crate::TICK_DT;

pub const MAP_WIDTH: f32 = 960.0;
pub const MAP_HEIGHT: f32 = 640.0;
pub const MAP_TILES_X: i16 = 30;
pub const MAP_TILES_Y: i16 = 20;

const CASTLE_HP: f32 = 1200.0;
const CASTLE_FOOTPRINT: (u8, u8) = (2, 2);
pub const BASE_INCOME: f32 = 8.0;
pub const POINT_INCOME: f32 = 6.0;
const STARTING_GOLD: f32 = 15.0;
const CAPTURE_RATE: f32 = 25.0;
const CASTLE_ATTACK_RANGE: f32 = 36.0;
const CASTLE_DPS: f32 = 35.0;
const CASTLE_ARMOR_PHYS: f32 = 30.0;
const CASTLE_ARMOR_MAG: f32 = 15.0;
/// Max distance from an ally anchor to place a new building (world units).
pub const DEPLOY_RADIUS: f32 = TILE_SIZE * 5.0;

#[derive(Clone, Debug)]
pub struct Castle {
    pub faction: FactionId,
    pub tile_x: i16,
    pub tile_y: i16,
    pub x: f32,
    pub y: f32,
    pub hp: f32,
    pub max_hp: f32,
}

impl Castle {
    pub fn alive(&self) -> bool {
        self.hp > 0.0
    }

    pub fn footprint(&self) -> (u8, u8) {
        CASTLE_FOOTPRINT
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

#[derive(Clone, Debug)]
pub struct SimBuilding {
    pub id: u32,
    pub faction: FactionId,
    pub kind: BuildingKind,
    pub tile_x: i16,
    pub tile_y: i16,
    pub produce_cd: f32,
}

impl SimBuilding {
    pub fn center(&self) -> (f32, f32) {
        let (fw, fh) = self.kind.footprint();
        footprint_center(self.tile_x, self.tile_y, fw, fh)
    }
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
    pub buildings_placed: u32,
}

#[derive(Clone, Debug)]
pub struct SimWorld {
    pub config: MatchConfig,
    pub tick: u32,
    pub gold: Vec<f32>,
    pub castles: Vec<Castle>,
    pub points: Vec<CapturePoint>,
    pub units: Vec<SimUnit>,
    pub buildings: Vec<SimBuilding>,
    pub obstacles: Vec<(f32, f32, f32, f32)>,
    pub flows: FlowFieldSet,
    next_unit_id: u32,
    next_building_id: u32,
    spawn_jitter: u32,
}

impl SimWorld {
    pub fn new(config: MatchConfig) -> Self {
        let n = config.faction_count();
        let spawns = spawn_tile_positions(n);
        let mut castles = Vec::with_capacity(n);
        for (i, &(tx, ty)) in spawns.iter().enumerate() {
            let (fw, fh) = CASTLE_FOOTPRINT;
            let (x, y) = footprint_center(tx, ty, fw, fh);
            castles.push(Castle {
                faction: FactionId(i as u8),
                tile_x: tx,
                tile_y: ty,
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
            gold: vec![STARTING_GOLD; n],
            config,
            tick: 0,
            castles,
            points,
            units: Vec::with_capacity(512),
            buildings: Vec::with_capacity(64),
            obstacles,
            flows,
            next_unit_id: 1,
            next_building_id: 1,
            spawn_jitter: 0,
        }
    }

    pub fn team_of(&self, faction: FactionId) -> TeamId {
        self.config.team_for(faction)
    }

    pub fn are_allied(&self, a: FactionId, b: FactionId) -> bool {
        self.team_of(a) == self.team_of(b)
    }

    /// Passive income rate (gold / second) for HUD.
    pub fn income_per_sec(&self, faction: FactionId) -> f32 {
        let mut inc = BASE_INCOME;
        for p in &self.points {
            if p.owner == Some(faction) {
                inc += POINT_INCOME;
            }
        }
        inc
    }

    pub fn tick(&mut self, bus: &InputBus) -> TickResult {
        self.tick = self.tick.wrapping_add(1);
        let mut units_spawned = 0u32;
        let mut buildings_placed = 0u32;

        self.apply_income();
        buildings_placed += self.apply_placements(bus);
        units_spawned += self.apply_building_production();
        self.update_units_movement();
        self.update_combat();
        self.update_capture_points();
        let victory = self.check_victory();

        TickResult {
            victory,
            units_spawned,
            buildings_placed,
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

    pub fn can_place(&self, faction: FactionId, kind: BuildingKind, tile_x: i16, tile_y: i16) -> bool {
        let i = faction.index();
        if i >= self.gold.len() || self.gold[i] < kind.cost() {
            return false;
        }
        if !self
            .castles
            .iter()
            .any(|c| c.faction == faction && c.alive())
        {
            return false;
        }
        let (fw, fh) = kind.footprint();
        if !footprint_in_bounds(tile_x, tile_y, fw, fh) {
            return false;
        }
        if self.footprint_blocked(tile_x, tile_y, fw, fh) {
            return false;
        }
        self.within_deploy_radius(faction, tile_x, tile_y, fw, fh)
    }

    fn footprint_blocked(&self, tile_x: i16, tile_y: i16, fw: u8, fh: u8) -> bool {
        for c in &self.castles {
            let (cw, ch) = c.footprint();
            if footprints_overlap(tile_x, tile_y, fw, fh, c.tile_x, c.tile_y, cw, ch) {
                return true;
            }
        }
        for b in &self.buildings {
            let (bw, bh) = b.kind.footprint();
            if footprints_overlap(tile_x, tile_y, fw, fh, b.tile_x, b.tile_y, bw, bh) {
                return true;
            }
        }
        let (cx, cy) = footprint_center(tile_x, tile_y, fw, fh);
        let hw = fw as f32 * TILE_SIZE * 0.5;
        let hh = fh as f32 * TILE_SIZE * 0.5;
        for &(ox, oy, ow, oh) in &self.obstacles {
            if aabb_overlap(cx, cy, hw, hh, ox, oy, ow * 0.5, oh * 0.5) {
                return true;
            }
        }
        false
    }

    fn within_deploy_radius(
        &self,
        faction: FactionId,
        tile_x: i16,
        tile_y: i16,
        fw: u8,
        fh: u8,
    ) -> bool {
        let team = self.team_of(faction);
        let (bx, by) = footprint_center(tile_x, tile_y, fw, fh);
        for c in &self.castles {
            if self.team_of(c.faction) == team && c.alive() {
                if dist(bx, by, c.x, c.y) <= DEPLOY_RADIUS {
                    return true;
                }
            }
        }
        for b in &self.buildings {
            if self.team_of(b.faction) == team {
                let (ax, ay) = b.center();
                if dist(bx, by, ax, ay) <= DEPLOY_RADIUS {
                    return true;
                }
            }
        }
        false
    }

    fn apply_placements(&mut self, bus: &InputBus) -> u32 {
        let n = self.config.faction_count();
        let mut placed = 0u32;
        for i in 0..n {
            let faction = FactionId(i as u8);
            let input = bus.get(faction);
            let Some(cmd) = input.place else {
                continue;
            };
            if !self.can_place(faction, cmd.kind, cmd.tile_x, cmd.tile_y) {
                continue;
            }
            self.gold[i] -= cmd.kind.cost();
            self.buildings.push(SimBuilding {
                id: self.next_building_id,
                faction,
                kind: cmd.kind,
                tile_x: cmd.tile_x,
                tile_y: cmd.tile_y,
                produce_cd: cmd.kind.produce_interval() * 0.35,
            });
            self.next_building_id = self.next_building_id.wrapping_add(1);
            placed += 1;
        }
        placed
    }

    fn apply_building_production(&mut self) -> u32 {
        let mut spawned = 0u32;
        let mut spawns: Vec<(FactionId, UnitType, f32, f32)> = Vec::new();

        for b in &mut self.buildings {
            b.produce_cd -= TICK_DT;
            if b.produce_cd > 0.0 {
                continue;
            }
            b.produce_cd = b.kind.produce_interval();
            let (cx, cy) = b.center();
            self.spawn_jitter = self.spawn_jitter.wrapping_add(1);
            let angle = (self.spawn_jitter as f32) * 0.9;
            let j = (self.spawn_jitter % 5) as f32 - 2.0;
            let sx = cx + angle.cos() * (TILE_SIZE * 1.4) + j;
            let sy = cy + angle.sin() * (TILE_SIZE * 1.4);
            spawns.push((
                b.faction,
                b.kind.produces(),
                sx.clamp(8.0, MAP_WIDTH - 8.0),
                sy.clamp(8.0, MAP_HEIGHT - 8.0),
            ));
        }

        for (faction, ut, x, y) in spawns {
            self.units.push(SimUnit {
                id: self.next_unit_id,
                faction,
                unit_type: ut,
                x,
                y,
                hp: ut.hp(),
                max_hp: ut.hp(),
                attack_cd: 0.0,
            });
            self.next_unit_id = self.next_unit_id.wrapping_add(1);
            spawned += 1;
        }
        spawned
    }

    /// Suggest a free tile near anchors for AI / tools.
    pub fn find_build_site(&self, faction: FactionId, kind: BuildingKind) -> Option<(i16, i16)> {
        let (fw, fh) = kind.footprint();
        let mut anchors: Vec<(f32, f32)> = Vec::new();
        let team = self.team_of(faction);
        for c in &self.castles {
            if self.team_of(c.faction) == team && c.alive() {
                anchors.push((c.x, c.y));
            }
        }
        for b in &self.buildings {
            if self.team_of(b.faction) == team {
                anchors.push(b.center());
            }
        }
        if anchors.is_empty() {
            return None;
        }

        let mut candidates: Vec<(i16, i16, f32)> = Vec::new();
        for &(ax, ay) in &anchors {
            let (atx, aty) = crate::input::world_to_tile(ax, ay);
            for dy in -6i16..=6 {
                for dx in -6i16..=6 {
                    let tx = atx + dx;
                    let ty = aty + dy;
                    if !self.can_place(faction, kind, tx, ty) {
                        continue;
                    }
                    let (cx, cy) = footprint_center(tx, ty, fw, fh);
                    let d = dist2(ax, ay, cx, cy);
                    candidates.push((tx, ty, d));
                }
            }
        }
        candidates.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Less));
        candidates.first().map(|&(x, y, _)| (x, y))
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

        let snapshot: Vec<(u32, FactionId, TeamId, f32, f32, f32, UnitType, f32, f32)> = self
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
                    u.unit_type,
                    u.hp,
                    u.max_hp,
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

            for &(id, _f, _t, ox, oy, r, ..) in &snapshot {
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

            if unit.unit_type.heal_power() > 0.0 {
                if let Some((tx, ty)) =
                    nearest_wounded_ally(my_team, unit.id, unit.x, unit.y, &snapshot)
                {
                    let dx = tx - unit.x;
                    let dy = ty - unit.y;
                    let d = (dx * dx + dy * dy).sqrt().max(0.1);
                    if d < unit.unit_type.range() * 2.0 {
                        vx = dx / d;
                        vy = dy / d;
                    }
                }
            } else if let Some((ex, ey)) = nearest_enemy_pos(my_team, unit.x, unit.y, &snapshot) {
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
        let mut unit_heal: Vec<(usize, f32)> = Vec::new();
        let mut castle_dmg: Vec<(usize, f32)> = Vec::new();
        let mut attackers: Vec<usize> = Vec::new();

        for i in 0..n {
            if self.units[i].attack_cd > 0.0 {
                continue;
            }
            let ut = self.units[i].unit_type;
            let ax = self.units[i].x;
            let ay = self.units[i].y;
            let range = ut.range();
            let my_team = alliances[i];
            let mut did_action = false;

            if ut.heal_power() > 0.0 {
                let mut best: Option<(usize, f32)> = None;
                for j in 0..n {
                    if i == j || alliances[j] != my_team {
                        continue;
                    }
                    let missing = self.units[j].max_hp - self.units[j].hp;
                    if missing <= 0.5 {
                        continue;
                    }
                    if dist(ax, ay, self.units[j].x, self.units[j].y) <= range {
                        if best.map(|(_, m)| missing > m).unwrap_or(true) {
                            best = Some((j, missing));
                        }
                    }
                }
                if let Some((j, _)) = best {
                    unit_heal.push((j, ut.heal_power()));
                    did_action = true;
                }
            } else {
                let mut primary: Option<usize> = None;
                let mut best_d = f32::MAX;
                for j in 0..n {
                    if i == j || alliances[j] == my_team {
                        continue;
                    }
                    let d = dist(ax, ay, self.units[j].x, self.units[j].y);
                    if d <= range && d < best_d {
                        best_d = d;
                        primary = Some(j);
                    }
                }

                if let Some(pj) = primary {
                    match ut.targeting() {
                        Targeting::Single => {
                            let dmg = compute_unit_damage(ut, self.units[pj].unit_type);
                            unit_dmg.push((pj, dmg));
                            did_action = true;
                        }
                        Targeting::Multitarget { count } => {
                            let mut hits: Vec<(usize, f32)> = Vec::new();
                            for j in 0..n {
                                if i == j || alliances[j] == my_team {
                                    continue;
                                }
                                let d = dist(ax, ay, self.units[j].x, self.units[j].y);
                                if d <= range {
                                    hits.push((j, d));
                                }
                            }
                            hits.sort_by(|a, b| {
                                a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Less)
                            });
                            for &(j, _) in hits.iter().take(count as usize) {
                                let dmg = compute_unit_damage(ut, self.units[j].unit_type);
                                unit_dmg.push((j, dmg));
                            }
                            did_action = !hits.is_empty();
                        }
                        Targeting::Aoe { radius } => {
                            let (tx, ty) = (self.units[pj].x, self.units[pj].y);
                            for j in 0..n {
                                if alliances[j] == my_team {
                                    continue;
                                }
                                if dist(tx, ty, self.units[j].x, self.units[j].y) <= radius {
                                    let dmg = compute_unit_damage(ut, self.units[j].unit_type);
                                    unit_dmg.push((j, dmg));
                                }
                            }
                            did_action = true;
                        }
                    }
                }

                if !did_action {
                    for (ci, castle) in self.castles.iter().enumerate() {
                        if !castle.alive() || castle_teams[ci] == my_team {
                            continue;
                        }
                        if dist(ax, ay, castle.x, castle.y) <= CASTLE_ATTACK_RANGE.max(range) {
                            let armor = match ut.damage_type() {
                                DamageType::Physical => CASTLE_ARMOR_PHYS,
                                DamageType::Magical => CASTLE_ARMOR_MAG,
                            };
                            let pen = match ut.damage_type() {
                                DamageType::Physical => ut.pen_phys(),
                                DamageType::Magical => ut.pen_mag(),
                            };
                            let dmg = mitigate_damage(ut.damage(), armor, pen);
                            castle_dmg.push((ci, dmg));
                            did_action = true;
                            break;
                        }
                    }
                }
            }

            if did_action {
                attackers.push(i);
            }
        }

        for (j, dmg) in unit_dmg {
            if j < self.units.len() {
                self.units[j].hp -= dmg;
            }
        }
        for (j, heal) in unit_heal {
            if j < self.units.len() {
                let u = &mut self.units[j];
                u.hp = (u.hp + heal).min(u.max_hp);
            }
        }
        for (ci, dmg) in castle_dmg {
            if ci < self.castles.len() {
                self.castles[ci].hp -= dmg;
            }
        }
        for i in attackers {
            if i < self.units.len() {
                let cd = self.units[i].unit_type.attack_cooldown();
                self.units[i].attack_cd = cd;
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
                    let dmg = mitigate_damage(
                        CASTLE_DPS * TICK_DT,
                        unit.unit_type.armor_phys(),
                        0.0,
                    );
                    unit.hp -= dmg;
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

fn compute_unit_damage(attacker: UnitType, defender: UnitType) -> f32 {
    let (armor, pen) = match attacker.damage_type() {
        DamageType::Physical => (defender.armor_phys(), attacker.pen_phys()),
        DamageType::Magical => (defender.armor_mag(), attacker.pen_mag()),
    };
    mitigate_damage(attacker.damage(), armor, pen)
}

fn footprint_in_bounds(tx: i16, ty: i16, fw: u8, fh: u8) -> bool {
    tx >= 0
        && ty >= 0
        && tx + fw as i16 <= MAP_TILES_X
        && ty + fh as i16 <= MAP_TILES_Y
}

fn footprints_overlap(
    ax: i16,
    ay: i16,
    aw: u8,
    ah: u8,
    bx: i16,
    by: i16,
    bw: u8,
    bh: u8,
) -> bool {
    ax < bx + bw as i16 && ax + aw as i16 > bx && ay < by + bh as i16 && ay + ah as i16 > by
}

fn aabb_overlap(
    ax: f32,
    ay: f32,
    ahw: f32,
    ahh: f32,
    bx: f32,
    by: f32,
    bhw: f32,
    bhh: f32,
) -> bool {
    (ax - ahw) < (bx + bhw)
        && (ax + ahw) > (bx - bhw)
        && (ay - ahh) < (by + bhh)
        && (ay + ahh) > (by - bhh)
}

fn spawn_tile_positions(n: usize) -> Vec<(i16, i16)> {
    match n {
        2 => vec![(2, 9), (26, 9)],
        4 => vec![(2, 2), (2, 16), (26, 2), (26, 16)],
        _ => {
            let mut v = Vec::new();
            for i in 0..n {
                let a = (i as f32 / n as f32) * std::f32::consts::TAU;
                let x = (MAP_WIDTH * 0.5 + a.cos() * 320.0) / TILE_SIZE;
                let y = (MAP_HEIGHT * 0.5 + a.sin() * 220.0) / TILE_SIZE;
                v.push((x as i16, y as i16));
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
    snapshot: &[(u32, FactionId, TeamId, f32, f32, f32, UnitType, f32, f32)],
) -> Option<(f32, f32)> {
    let mut best = None;
    for &(_id, _f, team, x, y, ..) in snapshot {
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

fn nearest_wounded_ally(
    my_team: TeamId,
    self_id: u32,
    ux: f32,
    uy: f32,
    snapshot: &[(u32, FactionId, TeamId, f32, f32, f32, UnitType, f32, f32)],
) -> Option<(f32, f32)> {
    let mut best = None;
    for &(id, _f, team, x, y, _, _, hp, max_hp) in snapshot {
        if id == self_id || team != my_team || hp >= max_hp - 0.5 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::GameInput;
    use crate::types::MatchConfig;

    #[test]
    fn mitigate_formula() {
        let d = mitigate_damage(100.0, 100.0, 0.0);
        assert!((d - 50.0).abs() < 0.01);
        let d2 = mitigate_damage(100.0, 100.0, 50.0);
        assert!((d2 - (100.0 * 100.0 / 150.0)).abs() < 0.01);
    }

    #[test]
    fn place_barracks_near_castle() {
        let mut world = SimWorld::new(MatchConfig::duel_vs_ai());
        world.gold[0] = 100.0;
        let site = world
            .find_build_site(FactionId(0), BuildingKind::Barracks)
            .expect("site");
        let mut bus = InputBus::default();
        bus.set(
            FactionId(0),
            GameInput::place(BuildingKind::Barracks, site.0, site.1),
        );
        let r = world.tick(&bus);
        assert_eq!(r.buildings_placed, 1);
        assert_eq!(world.buildings.len(), 1);
    }
}
