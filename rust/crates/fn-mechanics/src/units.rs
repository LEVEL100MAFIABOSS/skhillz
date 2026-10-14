//! Conversions between the two coordinate conventions the crates use.
//!
//! * UE space (building, hitreg, rules, the movement component inside the controller): centimetres, Z up, X forward, Y right.
//! * Controller space ([`crate::movement::Controller`]): metres; `position` = (UE X, UE Y) / 100, `height` = UE Z / 100.
//! * Camera / game space ([`crate::camera`]): metres, Y up: (x, y, z) = (UE X, UE Z, UE Y) / 100.
use glam::{Vec2, Vec3};

pub const CM_PER_M: f32 = 100.;

/// Controller feet (position, height) -> UE centimetres.
pub fn controller_to_ue(position: Vec2, height: f32) -> Vec3 { Vec3::new(position.x, position.y, height) * CM_PER_M }
/// UE centimetres -> controller (position, height).
pub fn ue_to_controller(p: Vec3) -> (Vec2, f32) { (Vec2::new(p.x, p.y) / CM_PER_M, p.z / CM_PER_M) }
/// Controller feet -> camera / game space (metres, Y up).
pub fn controller_to_game(position: Vec2, height: f32) -> Vec3 { Vec3::new(position.x, height, position.y) }
/// Camera / game space (metres, Y up) -> UE centimetres.
pub fn game_to_ue(g: Vec3) -> Vec3 { Vec3::new(g.x, g.z, g.y) * CM_PER_M }
/// UE centimetres -> camera / game space (metres, Y up).
pub fn ue_to_game(u: Vec3) -> Vec3 { Vec3::new(u.x, u.z, u.y) / CM_PER_M }
