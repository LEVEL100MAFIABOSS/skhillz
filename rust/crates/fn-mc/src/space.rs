//! Minecraft space <-> UE space. Minecraft: metres, Y up, right-handed (X east, Z south). UE (the mechanics): centimetres, Z up, left-handed.
//! UE (x, y, z) = (MC (x, z, y) - origin) * 100: swapping two axes flips the handedness, so the mapping is a reflection `P` (P = P^-1).
//!
//! PRECISION: the mechanics use f32. A Minecraft world reaches 30 000 000 blocks (3e9 cm), where f32 steps are over a metre, so every engine
//! works relative to a local `origin` near where it is used ([`set_origin`]). The origin is a whole number of 5.12 m build cells horizontally
//! and 0 vertically, so the build grid (relative to the level origin) is unchanged by it.
use glam::{Mat3, Quat, Vec3};
use std::cell::Cell;

pub const CM: f64 = 100.;
/// One build cell (SnapGridSize 512 cm) in metres.
pub const CELL_M: f64 = 5.12;

thread_local! {
    static ORIGIN: Cell<[f64; 3]> = const { Cell::new([0.; 3]) };
}

/// The current engine's local origin (MC metres).
pub fn origin() -> [f64; 3] { ORIGIN.with(|o| o.get()) }
pub fn set_origin(o: [f64; 3]) { ORIGIN.with(|c| c.set(o)); }
/// A local origin near `p`: whole build cells horizontally, 0 vertically.
pub fn origin_near(p: [f64; 3]) -> [f64; 3] { [(p[0] / CELL_M).round() * CELL_M, 0., (p[2] / CELL_M).round() * CELL_M] }

pub fn mc_to_ue(v: [f64; 3]) -> Vec3 {
    let o = origin();
    Vec3::new(((v[0] - o[0]) * CM) as f32, ((v[2] - o[2]) * CM) as f32, ((v[1] - o[1]) * CM) as f32)
}
pub fn ue_to_mc(v: Vec3) -> [f64; 3] {
    let o = origin();
    [v.x as f64 / CM + o[0], v.z as f64 / CM + o[1], v.y as f64 / CM + o[2]]
}
/// Local UE cm -> absolute UE cm (f64, for saving).
pub fn ue_to_abs(v: Vec3) -> [f64; 3] { let o = origin(); [v.x as f64 + o[0] * CM, v.y as f64 + o[2] * CM, v.z as f64 + o[1] * CM] }
/// Absolute UE cm (f64) -> local UE cm.
pub fn abs_to_ue(v: [f64; 3]) -> Vec3 { let o = origin(); Vec3::new((v[0] - o[0] * CM) as f32, (v[1] - o[2] * CM) as f32, (v[2] - o[1] * CM) as f32) }

/// A direction (no scaling, no origin).
pub fn mc_dir_to_ue(v: [f64; 3]) -> Vec3 { Vec3::new(v[0] as f32, v[2] as f32, v[1] as f32).normalize_or_zero() }

/// The reflection P as a matrix (columns: P e_x = e_x, P e_y = e_z, P e_z = e_y).
fn p() -> Mat3 { Mat3::from_cols(Vec3::X, Vec3::Z, Vec3::Y) }

/// A UE rotation as the equivalent MC rotation: R_mc = P R_ue P (a proper rotation again).
pub fn rot_ue_to_mc(q: Quat) -> Quat { Quat::from_mat3(&(p() * Mat3::from_quat(q) * p())).normalize() }
