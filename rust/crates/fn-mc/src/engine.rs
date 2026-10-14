//! One building world per Minecraft dimension: players' build previews, placement, pickaxe swings, piece damage and structural collapse.
//! Driven by [`Engine::handle`] with JSON commands (see `Cmd`); every command answers with the world changes since the previous answer.
use crate::host::HostBlocks;
use crate::shapes::{collision_aabbs, render_boxes, render_boxes_at, RenderBox};
use crate::space::{mc_dir_to_ue, mc_to_ue};
use fn_mechanics::building::damage::{self, BuildingDamage, Hit};
use fn_mechanics::building::grid::Rot;
use fn_mechanics::building::pieces::{ConnectivityCube, Material, PieceCatalog, PieceKind};
use fn_mechanics::building::placement::{self, BuildPreview, HostWorld, PlacementEnv, QueryResult, ViewInput};
use fn_mechanics::building::structural::{no_anchor, StructuralEnv, Voxel};
use fn_mechanics::building::world::{BuildingWorld, PieceId, WorldEvent};
use fn_mechanics::combat::{Notify, Swing};
use fn_mechanics::{placeholder, MechanicsData};
use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

/// Pickaxe reach from the eye (Minecraft's own block reach is about 4.5 m; Fortnite's melee sweep is 2.88 m from the chest).
pub const PICKAXE_REACH_CM: f32 = 350.;
pub const PICKAXE_RADIUS_CM: f32 = 20.;

#[derive(Deserialize, Debug)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Cmd {
    /// Start over with `data` = a mechanics-data.json text (None: the placeholder catalog). `grid_y` = MC height of building level 0.
    Init { data: Option<String>, grid_y: Option<f64> },
    SetGridY { y: f64 },
    /// One player's state this tick. `piece`: "wall" | "floor" | "stairs" | "roof" | null (not building).
    Player {
        id: String, eye: [f64; 3], look: [f64; 3], feet: [f64; 3],
        piece: Option<String>, material: Option<String>,
        place: bool, rotate: bool, attack: bool, pickaxe: bool, dt: f32,
        /// Where the look ray hits a Minecraft block (within build range) and that face's normal, from the host's raycast.
        #[serde(default)] hit: Option<[f64; 3]>, #[serde(default)] hit_normal: Option<[f64; 3]>,
    },
    RemovePlayer { id: String },
    /// Advance building timers (build-up, structural collapse checks) by `dt` seconds.
    Tick { dt: f32 },
    /// All pieces (for saving) / restore saved pieces.
    Export,
    Import { pieces: Vec<SavedPiece> },
    /// Every live piece as `spawn` changes (rebuilding the Minecraft side after a reload).
    Snapshot,
    /// Remove every piece.
    Clear,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SavedPiece { pub kind: String, pub material: String, pub location: [f32; 3], pub yaw: f32, pub health: f32 }

#[derive(Serialize, Debug, Clone)]
pub struct Spawn { pub id: u32, pub kind: &'static str, pub material: &'static str, pub boxes: Vec<RenderBox>, pub aabbs: Vec<[f64; 6]> }

#[derive(Serialize, Debug, Clone)]
pub struct HitOut { pub id: u32, pub material: &'static str, pub pos: [f64; 3], pub damage: f32, pub health: f32, pub max_health: f32, pub destroyed: bool }

#[derive(Serialize, Debug, Clone)]
pub struct Preview { pub player: String, pub ok: bool, pub result: String, pub kind: &'static str, pub material: &'static str, pub boxes: Vec<RenderBox> }

#[derive(Serialize, Debug, Default)]
pub struct Out {
    pub spawn: Vec<Spawn>,
    pub remove: Vec<u32>,
    pub hits: Vec<HitOut>,
    pub preview: Vec<Preview>,
    /// Players whose preview went away (stopped building / left).
    pub preview_clear: Vec<String>,
    pub messages: Vec<(String, String)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export: Option<Vec<SavedPiece>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

struct Player { preview: BuildPreview, building: bool, swing: Swing, last_preview: Option<(Vec3, f32, PieceKind, Material, bool)> }
impl Default for Player {
    fn default() -> Self { Player { preview: BuildPreview::default(), building: false, swing: Swing::default(), last_preview: None } }
}

pub struct Engine {
    catalog: PieceCatalog,
    world: BuildingWorld,
    damage: BuildingDamage,
    env: PlacementEnv,
    players: HashMap<String, Player>,
    events_cursor: u64,
    destroyed_cursor: u64,
}

pub fn kind_name(k: PieceKind) -> &'static str { match k { PieceKind::Wall => "wall", PieceKind::Floor => "floor", PieceKind::Stairs => "stairs", PieceKind::Roof => "roof" } }
pub fn material_name(m: Material) -> &'static str { match m { Material::Wood => "wood", Material::Stone => "stone", Material::Metal => "metal" } }
pub fn parse_kind(s: &str) -> Option<PieceKind> { match s { "wall" => Some(PieceKind::Wall), "floor" => Some(PieceKind::Floor), "stairs" | "ramp" => Some(PieceKind::Stairs), "roof" | "cone" => Some(PieceKind::Roof), _ => None } }
pub fn parse_material(s: &str) -> Option<Material> { match s { "wood" => Some(Material::Wood), "stone" | "brick" => Some(Material::Stone), "metal" => Some(Material::Metal), _ => None } }

fn query_name(r: QueryResult) -> String { format!("{r:?}") }

impl Default for Engine { fn default() -> Self { Engine::new(placeholder::catalog(), 64.) } }

impl Engine {
    pub fn new(catalog: PieceCatalog, grid_y: f64) -> Engine {
        let mut e = Engine { catalog, world: BuildingWorld::new(), damage: BuildingDamage::new(), env: PlacementEnv::default(), players: HashMap::new(), events_cursor: 0, destroyed_cursor: 0 };
        e.env.host = HostWorld(Some(Arc::new(HostBlocks)));
        e.set_grid_y(grid_y);
        e
    }

    fn set_grid_y(&mut self, y: f64) { self.env.grid_origin = Vec3::new(0., 0., (y * 100.) as f32); }

    pub fn handle_json(&mut self, text: &str) -> String {
        let out = match serde_json::from_str::<Cmd>(text) {
            Ok(cmd) => self.handle(cmd),
            Err(e) => Out { error: Some(format!("bad command: {e}")), ..Default::default() },
        };
        serde_json::to_string(&out).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }

    pub fn handle(&mut self, cmd: Cmd) -> Out {
        let mut out = Out::default();
        match cmd {
            Cmd::Init { data, grid_y } => {
                let catalog = match data {
                    Some(text) => match MechanicsData::from_str(&text) {
                        Ok(d) if !d.pieces.defs().is_empty() => d.pieces,
                        Ok(_) => { out.error = Some("mechanics data has no pieces; using the placeholder catalog".into()); placeholder::catalog() }
                        Err(e) => { out.error = Some(format!("{e}; using the placeholder catalog")); placeholder::catalog() }
                    },
                    None => placeholder::catalog(),
                };
                *self = Engine::new(catalog, grid_y.unwrap_or(64.));
            }
            Cmd::SetGridY { y } => self.set_grid_y(y),
            Cmd::Player { id, eye, look, feet, piece, material, place, rotate, attack, pickaxe, dt, hit, hit_normal } => {
                let host_hit = hit.map(|h| mc_to_ue(h) + hit_normal.map(mc_dir_to_ue).unwrap_or(Vec3::Z) * fn_mechanics::building::grid::TRACE_NORMAL_PUSH);
                self.player(&mut out, id, eye, look, feet, piece, material, place, rotate, attack, pickaxe, dt, host_hit);
            }
            Cmd::RemovePlayer { id } => { if self.players.remove(&id).is_some() { out.preview_clear.push(id); } }
            Cmd::Tick { dt } => self.tick(dt),
            Cmd::Export => {
                out.export = Some(self.world.iter().map(|p| SavedPiece { kind: kind_name(p.kind).into(), material: material_name(p.material).into(), location: p.location.to_array(), yaw: p.yaw_deg, health: p.health }).collect());
            }
            Cmd::Import { pieces } => {
                for s in pieces {
                    let (Some(k), Some(m)) = (parse_kind(&s.kind), parse_material(&s.material)) else { continue };
                    let Some(def) = self.catalog.def(k, m) else { continue };
                    let id = placement::spawn_piece(&mut self.world, def, Vec3::from(s.location), s.yaw, false, self.env.grid_origin);
                    self.world.set_health(id, s.health.min(def.max_health).max(1.));
                }
                // a restored piece is fully built
                for _ in 0..400 { self.world.tick(0.1); if self.world.iter().all(|p| !p.is_building()) { break; } }
            }
            Cmd::Snapshot => {
                for p in self.world.iter() { out.spawn.push(Spawn { id: p.id, kind: kind_name(p.kind), material: material_name(p.material), boxes: render_boxes(p), aabbs: collision_aabbs(p) }); }
                let (_, c) = self.world.events_since(u64::MAX); self.events_cursor = c;
                return out;
            }
            Cmd::Clear => {
                let ids: Vec<PieceId> = self.world.iter().map(|p| p.id).collect();
                for id in ids { self.world.remove(id); }
                self.damage = BuildingDamage::new();
            }
        }
        self.collect(&mut out);
        out
    }

    fn structural_env<'a>(catalog: &'a PieceCatalog, host: &'a HostBlocks, grid_origin: Vec3, cubes: &'a dyn Fn(&str) -> Option<ConnectivityCube>, anchored: &'a dyn Fn(&Voxel) -> bool) -> StructuralEnv<'a> {
        let _ = catalog;
        StructuralEnv { collision: host, cubes, grid_origin, anchored }
    }

    fn tick(&mut self, dt: f32) {
        self.world.tick(dt);
        let host = HostBlocks;
        let cat = &self.catalog;
        let cubes = |c: &str| cat.class(c).map(|k| k.cube);
        let senv = Self::structural_env(cat, &host, self.env.grid_origin, &cubes, &no_anchor);
        self.damage.tick(&mut self.world, &senv);
    }

    #[allow(clippy::too_many_arguments)]
    fn player(&mut self, out: &mut Out, id: String, eye: [f64; 3], look: [f64; 3], feet: [f64; 3], piece: Option<String>, material: Option<String>, place: bool, rotate: bool, attack: bool, pickaxe: bool, dt: f32, host_hit: Option<Vec3>) {
        let pl = self.players.entry(id.clone()).or_default();
        pl.preview.host_hit = host_hit;
        let eye_ue = mc_to_ue(eye);
        let dir = mc_dir_to_ue(look);
        let kind = piece.as_deref().and_then(parse_kind);
        if let Some(m) = material.as_deref().and_then(parse_material) { pl.preview.material = m; }
        match kind {
            Some(k) => {
                pl.building = true;
                pl.preview.set_piece(k);
                if rotate { pl.preview.rotate(true); }
                let rot = Rot { pitch: dir.z.clamp(-1., 1.).asin().to_degrees(), yaw: dir.y.atan2(dir.x).to_degrees(), roll: 0. };
                let pawn = mc_to_ue([feet[0], feet[1] + 0.9, feet[2]]);
                let q = pl.preview.update(&self.world, &self.catalog, &self.env, &ViewInput { camera: eye_ue, view: rot, pawn });
                if place {
                    pl.preview.prepare_place();
                    match placement::place(&mut self.world, &self.catalog, &self.env, &pl.preview, u32::MAX) {
                        Ok(_) => {}
                        Err(r) => out.messages.push((id.clone(), format!("Can't build here ({})", query_name(r)))),
                    }
                }
                let ok = q == QueryResult::CanAdd;
                let key = (pl.preview.loc, pl.preview.rot.yaw, k, pl.preview.material, ok);
                if pl.last_preview != Some(key) || place {
                    pl.last_preview = Some(key);
                    out.preview.push(Preview { player: id.clone(), ok, result: query_name(q), kind: kind_name(k), material: material_name(pl.preview.material), boxes: render_boxes_at(k, pl.preview.loc, pl.preview.rot.yaw) });
                }
            }
            None => {
                if pl.building { pl.building = false; pl.last_preview = None; out.preview_clear.push(id.clone()); }
                // the pickaxe montage: a hit trace at each TriggerAbility notify while the attack button is held
                let down = attack && pickaxe;
                if let Some(Notify::TriggerAbility { .. }) = pl.swing.step(down, dt) {
                    if let Some((pid, dist, _)) = damage::trace(&self.world, eye_ue, dir, PICKAXE_REACH_CM, PICKAXE_RADIUS_CM) {
                        let host = HostBlocks;
                        let cat = &self.catalog;
                        let cubes = |c: &str| cat.class(c).map(|k| k.cube);
                        let senv = Self::structural_env(cat, &host, self.env.grid_origin, &cubes, &no_anchor);
                        let material = self.world.get(pid).map(|p| p.material).unwrap_or(Material::Wood);
                        let max_health = self.world.get(pid).map(|p| p.max_health).unwrap_or(0.);
                        if let Some(r) = self.damage.apply_hit(&mut self.world, pid, Hit::Pickaxe { weak_spot: false }, &senv) {
                            let at = eye_ue + dir * dist;
                            out.hits.push(HitOut { id: pid, material: material_name(material), pos: crate::space::ue_to_mc(at), damage: r.damage, health: r.health, max_health, destroyed: r.destroyed });
                        }
                    }
                }
            }
        }
    }

    /// World events since the last answer -> spawn / remove lists (a piece changed by an edit is removed and spawned again).
    fn collect(&mut self, out: &mut Out) {
        let (events, next) = self.world.events_since(self.events_cursor);
        let mut changed: BTreeMap<PieceId, bool> = BTreeMap::new(); // id -> still alive
        for e in events {
            match e {
                WorldEvent::Placed(id) | WorldEvent::Edited(id) => { changed.insert(*id, true); }
                WorldEvent::Removed(id) => { changed.insert(*id, false); }
                _ => {}
            }
        }
        self.events_cursor = next;
        let (_, dnext) = self.damage.destroyed_since(self.destroyed_cursor);
        self.destroyed_cursor = dnext;
        for (id, alive) in changed {
            out.remove.push(id);
            if alive {
                if let Some(p) = self.world.get(id) {
                    out.spawn.push(Spawn { id, kind: kind_name(p.kind), material: material_name(p.material), boxes: render_boxes(p), aabbs: collision_aabbs(p) });
                }
            }
        }
    }

    pub fn piece_count(&self) -> usize { self.world.len() }
}
