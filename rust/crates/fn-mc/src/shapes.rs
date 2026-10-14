//! What a placed piece looks like and what it collides with, in Minecraft space.
//! Shapes follow the pieces' trace geometry in `fn_building::damage` (the cooked mesh bounds) and the cooked wall collision box.
use crate::space::{rot_ue_to_mc, ue_to_mc};
use fn_mechanics::building::pieces::PieceKind;
use fn_mechanics::building::world::PlacedPiece;
use glam::{Quat, Vec3};
use serde::Serialize;

/// One box of a piece in its local UE frame (cm): centre, half extents, local rotation.
#[derive(Clone, Copy, Debug)]
pub struct LocalBox { pub c: Vec3, pub h: Vec3, pub q: Quat }
impl LocalBox { fn aa(c: Vec3, h: Vec3) -> Self { LocalBox { c, h, q: Quat::IDENTITY } } }

/// A render box in MC space for a block display: entity position `pos`, and the display transformation
/// (translation relative to `pos`, left rotation `rot` (x, y, z, w), scale) that maps the unit block onto the box.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct RenderBox { pub pos: [f64; 3], pub translation: [f32; 3], pub rot: [f32; 4], pub scale: [f32; 3] }

const WALL_HALF_THICK: f32 = 10.;
const FLOOR_HALF_THICK: f32 = 14.;
const RAMP_HALF_THICK: f32 = 14.047;
const ROOF_APEX_Z: f32 = 179.49;
const ROOF_TIERS: usize = 4;
const RAMP_STEPS: usize = 16;
const RAMP_STEP_THICK: f32 = 30.;

/// Visual boxes (local UE cm).
pub fn render_local(kind: PieceKind) -> Vec<LocalBox> {
    match kind {
        PieceKind::Wall => vec![LocalBox::aa(Vec3::new(0., 0., 192.), Vec3::new(256., WALL_HALF_THICK, 192.))],
        PieceKind::Floor => vec![LocalBox::aa(Vec3::new(0., 256., 3.7), Vec3::new(256., 256., FLOOR_HALF_THICK))],
        PieceKind::Stairs => {
            // a slab from the low edge (y 512, z 0) to the high edge (y 0, z 384)
            let (low, high) = (Vec3::new(0., 512., 0.), Vec3::new(0., 0., 384.));
            let u = (high - low).normalize();
            let angle = u.z.atan2(u.y); // rotation about X that takes +Y onto u
            vec![LocalBox { c: (low + high) * 0.5, h: Vec3::new(256., (high - low).length() * 0.5, RAMP_HALF_THICK), q: Quat::from_rotation_x(angle) }]
        }
        PieceKind::Roof => roof_tiers(),
    }
}

fn roof_tiers() -> Vec<LocalBox> {
    let th = ROOF_APEX_Z / ROOF_TIERS as f32;
    (0..ROOF_TIERS).map(|k| {
        let half = 256. * (1. - k as f32 / ROOF_TIERS as f32);
        LocalBox::aa(Vec3::new(0., 256., th * (k as f32 + 0.5)), Vec3::new(half, half, th * 0.5))
    }).collect()
}

/// Collision boxes (local UE cm, axis aligned in the piece frame): walls / floors are their slab; ramps are 16 thin steps (a 24 cm rise each,
/// under Minecraft's 60 cm step height, so the ramp walks); roofs are their tiers.
pub fn collision_local(kind: PieceKind) -> Vec<LocalBox> {
    match kind {
        PieceKind::Wall | PieceKind::Floor => render_local(kind),
        PieceKind::Stairs => (0..RAMP_STEPS).map(|k| {
            let (s0, s1) = (k as f32 / RAMP_STEPS as f32, (k + 1) as f32 / RAMP_STEPS as f32);
            let (y0, y1) = (512. * (1. - s1), 512. * (1. - s0));
            let top = 384. * s1;
            LocalBox::aa(Vec3::new(0., (y0 + y1) * 0.5, top - RAMP_STEP_THICK * 0.5), Vec3::new(256., (y1 - y0) * 0.5, RAMP_STEP_THICK * 0.5))
        }).collect(),
        PieceKind::Roof => roof_tiers(),
    }
}

fn yaw_q(p: &PlacedPiece) -> Quat { Quat::from_rotation_z(p.yaw_deg.to_radians()) }

/// Block display boxes of a piece (piece location + yaw, mirrored edits are symmetric for these shapes).
pub fn render_boxes(p: &PlacedPiece) -> Vec<RenderBox> { render_boxes_at(p.kind, p.location, p.yaw_deg) }

pub fn render_boxes_at(kind: PieceKind, location: Vec3, yaw_deg: f32) -> Vec<RenderBox> {
    let yq = Quat::from_rotation_z(yaw_deg.to_radians());
    render_local(kind).into_iter().map(|b| {
        let centre_ue = location + yq * b.c;
        let q_mc = rot_ue_to_mc(yq * b.q);
        // half extents in the box's own MC frame: the reflection swaps local y and z
        let h_mc = Vec3::new(b.h.x, b.h.z, b.h.y) / 100.;
        let t = -(q_mc * h_mc);
        RenderBox { pos: ue_to_mc(centre_ue), translation: t.to_array(), rot: q_mc.to_array(), scale: (h_mc * 2.).to_array() }
    }).collect()
}

/// Collision AABBs in MC space `[min x, min y, min z, max x, max y, max z]` (piece yaws are multiples of 90 degrees, so the boxes stay axis aligned).
pub fn collision_aabbs(p: &PlacedPiece) -> Vec<[f64; 6]> {
    let yq = yaw_q(p);
    collision_local(p.kind).into_iter().map(|b| {
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for sx in [-1., 1.] { for sy in [-1., 1.] { for sz in [-1., 1.] {
            let v = p.location + yq * (b.c + b.q * (b.h * Vec3::new(sx, sy, sz)));
            lo = lo.min(v); hi = hi.max(v);
        } } }
        let (a, z) = (ue_to_mc(lo), ue_to_mc(hi));
        [a[0].min(z[0]), a[1].min(z[1]), a[2].min(z[2]), a[0].max(z[0]), a[1].max(z[1]), a[2].max(z[2])]
    }).collect()
}
