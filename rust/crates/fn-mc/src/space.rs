//! Minecraft space <-> UE space. Minecraft: metres, Y up, right-handed (X east, Z south). UE (the mechanics): centimetres, Z up, left-handed.
//! UE (x, y, z) = MC (x, z, y) * 100: swapping two axes flips the handedness, so the mapping is a reflection `P` (P = P^-1).
use glam::{Mat3, Quat, Vec3};

pub const CM: f64 = 100.;

pub fn mc_to_ue(v: [f64; 3]) -> Vec3 { Vec3::new((v[0] * CM) as f32, (v[2] * CM) as f32, (v[1] * CM) as f32) }
pub fn ue_to_mc(v: Vec3) -> [f64; 3] { [v.x as f64 / CM, v.z as f64 / CM, v.y as f64 / CM] }
/// A direction (no scaling).
pub fn mc_dir_to_ue(v: [f64; 3]) -> Vec3 { Vec3::new(v[0] as f32, v[2] as f32, v[1] as f32).normalize_or_zero() }

/// The reflection P as a matrix (columns: P e_x = e_x, P e_y = e_z, P e_z = e_y).
fn p() -> Mat3 { Mat3::from_cols(Vec3::X, Vec3::Z, Vec3::Y) }

/// A UE rotation as the equivalent MC rotation: R_mc = P R_ue P (a proper rotation again).
pub fn rot_ue_to_mc(q: Quat) -> Quat { Quat::from_mat3(&(p() * Mat3::from_quat(q) * p())).normalize() }
