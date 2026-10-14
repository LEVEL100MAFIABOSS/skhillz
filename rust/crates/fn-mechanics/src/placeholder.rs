//! A stand-in building catalog for trying the mechanics before (or without) baking `mechanics-data.json`.
//!
//! Numbers: the 1.8 values the original runtime's data test checks against the paks (`fn-building/tests/core_data.rs`):
//! max health walls 200 / 300 / 400 and other pieces 190 / 280 / 370 (wood / stone / metal), build time 4 / 12 / 20 s,
//! initial health 50 / 30 / 20 %, cost 10. Connectivity cubes: the cooked `ConnectivityCubeData` (`structural::base_cube`).
//!
//! Collision: ONLY the wall box is the cooked one (X -256..256, Y -8..8, Z 0..384 about the pivot, same test). Floor, stairs and roof
//! are simple stand-in shapes placed from the native centroid offsets (`grid::geom`): a 512 x 512 slab, a 512 x 512 ramp rising
//! 384 toward the pivot edge, and a slab for the roof. Bake real data, or replace them with your own meshes' convex hulls.
use crate::building::pieces::{ClassData, Material, PieceCatalog, PieceDef, PieceKind};
use crate::building::structural::base_cube;
use glam::Vec3;

fn cuboid(lo: Vec3, hi: Vec3) -> Vec<Vec3> {
    let mut v = Vec::with_capacity(8);
    for &x in &[lo.x, hi.x] { for &y in &[lo.y, hi.y] { for &z in &[lo.z, hi.z] { v.push(Vec3::new(x, y, z)); } } }
    v
}

/// Actor-local collision convexes (UE cm) of the stand-in pieces.
pub fn convexes(kind: PieceKind) -> Vec<Vec<Vec3>> {
    match kind {
        PieceKind::Wall => vec![cuboid(Vec3::new(-256., -8., 0.), Vec3::new(256., 8., 384.))],
        PieceKind::Floor => vec![cuboid(Vec3::new(-256., 0., -10.), Vec3::new(256., 512., 10.))],
        PieceKind::Roof => vec![cuboid(Vec3::new(-256., 0., 0.), Vec3::new(256., 512., 20.))],
        // wedge: high edge (z 384) on the pivot side (local y 0), low edge at local y 512
        PieceKind::Stairs => vec![vec![
            Vec3::new(-256., 0., 0.), Vec3::new(256., 0., 0.), Vec3::new(-256., 0., 384.), Vec3::new(256., 0., 384.),
            Vec3::new(-256., 512., 0.), Vec3::new(256., 512., 0.),
        ]],
    }
}

fn numbers(kind: PieceKind, m: Material) -> (f32, f32, f32) {
    let i = m.index();
    let health = if kind == PieceKind::Wall { [200., 300., 400.][i] } else { [190., 280., 370.][i] };
    (health, [4., 12., 20.][i], [0.5, 0.3, 0.2][i])
}

/// The 12 (piece, material) defs with the 1.8 numbers and stand-in collision.
pub fn catalog() -> PieceCatalog {
    let mut defs = Vec::new();
    for material in Material::ALL {
        for kind in PieceKind::ALL {
            let (max_health, build_time, initial_health_pct) = numbers(kind, material);
            let name = format!("Placeholder_{kind:?}_{material:?}");
            let class = ClassData {
                class_path: format!("/Placeholder/{name}.{name}_C"),
                kind_type: kind.building_type(),
                material: Some(material),
                current_level: 0,
                edit_pattern: String::new(),
                static_mesh: String::new(),
                attribute_category: String::new(),
                attribute_subcategory: String::new(),
                convexes: convexes(kind),
                cube: base_cube(kind),
            };
            defs.push(PieceDef { kind, material, class, max_health, initial_health_pct, build_time, cost: 10 });
        }
    }
    PieceCatalog::from_parts(defs, vec![])
}
