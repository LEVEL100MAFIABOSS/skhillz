//! The host (Minecraft) world as the building code sees it: solid blocks are ground (`Landscape` hits), so they support pieces and never
//! block them; pieces and blocks co-exist.
use crate::space::ue_to_mc;
use fn_mechanics::building::structural::{WorldCollision, WorldHit, WorldHitKind};
use glam::Vec3;

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "fnmc")]
extern "C" {
    /// 1 when any block with collision intersects the MC box (metres).
    fn solid_box(min_x: f64, min_y: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> i32;
}

#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    /// Native builds (tests): the solid-box query. Default: a flat ground filling everything below y = 0.
    pub static NATIVE: std::cell::RefCell<Box<dyn Fn([f64; 3], [f64; 3]) -> bool>> = std::cell::RefCell::new(Box::new(|min: [f64; 3], _max: [f64; 3]| min[1] < 0.));
}

/// Is any solid block inside this MC box?
pub fn solid(min: [f64; 3], max: [f64; 3]) -> bool {
    #[cfg(target_arch = "wasm32")]
    { unsafe { solid_box(min[0], min[1], min[2], max[0], max[1], max[2]) != 0 } }
    #[cfg(not(target_arch = "wasm32"))]
    { NATIVE.with(|f| (f.borrow())(min, max)) }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HostBlocks;
impl WorldCollision for HostBlocks {
    fn sweep_box(&self, start: Vec3, end: Vec3, half: Vec3) -> Vec<WorldHit> {
        let (lo, hi) = (ue_to_mc(start.min(end) - half), ue_to_mc(start.max(end) + half));
        let e = 1e-3;
        let min = [lo[0].min(hi[0]) - e, lo[1].min(hi[1]) - e, lo[2].min(hi[2]) - e];
        let max = [lo[0].max(hi[0]) + e, lo[1].max(hi[1]) + e, lo[2].max(hi[2]) + e];
        if solid(min, max) { vec![WorldHit { kind: WorldHitKind::Landscape, location: start, actor: None }] } else { vec![] }
    }
}
