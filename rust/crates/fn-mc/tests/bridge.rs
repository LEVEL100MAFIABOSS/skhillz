//! The bridge driven the way the Fabric mod drives it (JSON commands), on the native flat test ground (solid below y = 0).
use fn_mc::Engine;
use glam::{Quat, Vec3};
use serde_json::{json, Value};

fn call(e: &mut Engine, v: Value) -> Value { serde_json::from_str(&e.handle_json(&v.to_string())).unwrap() }

fn player(e: &mut Engine, piece: Option<&str>, look: [f64; 3], place: bool, attack: bool) -> Value { player_at(e, [0., 0.], piece, look, place, attack) }

/// The player standing at `base` + (0.5, 0, 0.5) on the flat ground y = 0.
fn player_at(e: &mut Engine, base: [f64; 2], piece: Option<&str>, look: [f64; 3], place: bool, attack: bool) -> Value {
    let (px, pz) = (base[0] + 0.5, base[1] + 0.5);
    // what Minecraft's block raycast reports: the ground plane y = 0 within 8 m
    let n = (look[0] * look[0] + look[1] * look[1] + look[2] * look[2]).sqrt();
    let t = if look[1] < 0. { 1.62 / (-look[1] / n) } else { f64::INFINITY };
    let (hit, normal) = if t <= 8. { (json!([px + look[0] / n * t, 0.0, pz + look[2] / n * t]), json!([0.0, 1.0, 0.0])) } else { (Value::Null, Value::Null) };
    call(e, json!({"cmd":"player","id":"steve","eye":[px,1.62,pz],"look":look,"feet":[px,0.0,pz],"piece":piece,"material":"wood",
        "place":place,"rotate":false,"attack":attack,"pickaxe":true,"dt":0.05,"hit":hit,"hit_normal":normal}))
}

fn engine() -> Engine { let mut e = Engine::default(); call(&mut e, json!({"cmd":"init","data":null,"grid_y":0.0})); e }

/// Where the unit block's corners land under a display transformation (Minecraft: T * L * S * R, R = identity).
fn display_aabb(b: &Value) -> ([f64; 3], [f64; 3]) {
    let f = |k: &str, i: usize| b[k][i].as_f64().unwrap();
    // the entity position is a double in Minecraft; the transformation is relative to it (floats)
    let pos = [f("pos", 0), f("pos", 1), f("pos", 2)];
    let t = Vec3::new(f("translation", 0) as f32, f("translation", 1) as f32, f("translation", 2) as f32);
    let q = Quat::from_xyzw(f("rot", 0) as f32, f("rot", 1) as f32, f("rot", 2) as f32, f("rot", 3) as f32);
    let s = Vec3::new(f("scale", 0) as f32, f("scale", 1) as f32, f("scale", 2) as f32);
    let (mut lo, mut hi) = (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY));
    for x in [0., 1.] { for y in [0., 1.] { for z in [0., 1.] { let v = t + q * (s * Vec3::new(x, y, z)); lo = lo.min(v); hi = hi.max(v); } } }
    ([pos[0] + lo.x as f64, pos[1] + lo.y as f64, pos[2] + lo.z as f64], [pos[0] + hi.x as f64, pos[1] + hi.y as f64, pos[2] + hi.z as f64])
}

#[test]
fn build_a_wall_in_front_and_pickaxe_it_down() { wall_and_pickaxe_at([0., 0.]); }

/// Where a game test once ran: f32 world coordinates there are over a metre apart, so this only passes with the engine's local origin.
#[test]
fn far_from_the_world_origin_works_the_same() { wall_and_pickaxe_at([-12_696_762., -12_464_777.]); }

fn wall_and_pickaxe_at(base: [f64; 2]) {
    let mut e = engine();
    // looking east (+X), slightly down
    let look = [0.97, -0.24, 0.0];
    let r = player_at(&mut e, base, Some("wall"), look, false, false);
    assert_eq!(r["preview"][0]["ok"], true, "{r}");
    let r = player_at(&mut e, base, Some("wall"), look, true, false);
    let spawn = &r["spawn"][0];
    assert_eq!(spawn["kind"], "wall", "{r}");
    // the wall stands on the ground (y 0..3.84), 5.12 wide, 20 cm thick, in front of the player
    let a: Vec<f64> = spawn["aabbs"][0].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
    let size = [a[3] - a[0], a[4] - a[1], a[5] - a[2]];
    assert!((a[1] - 0.).abs() < 1e-3 && (size[1] - 3.84).abs() < 1e-3, "{a:?}");
    assert!(((size[0] - 0.2).abs() < 1e-3 && (size[2] - 5.12).abs() < 1e-3) || ((size[2] - 0.2).abs() < 1e-3 && (size[0] - 5.12).abs() < 1e-3), "{a:?}");
    assert!(a[0] > base[0] + 0.5, "the wall is east of the player: {a:?}");
    // the display box covers the same volume as the collision box
    let (lo, hi) = display_aabb(&spawn["boxes"][0]);
    for i in 0..3 { assert!((lo[i] - a[i]).abs() < 1e-3 && (hi[i] - a[i + 3]).abs() < 1e-3, "display {lo:?} {hi:?} vs {a:?}"); }
    // let it finish building, then swing at it
    for _ in 0..100 { call(&mut e, json!({"cmd":"tick","dt":0.05})); }
    // walk up to 1.5 m in front of the wall's face (west of it), level with its middle, and look east at it
    let stand = [a[0] - 1.5 - 0.5, (a[2] + a[5]) * 0.5 - 0.5];
    let (mut hits, mut destroyed) = (0, false);
    for _ in 0..400 {
        let r = player_at(&mut e, stand, None, [1.0, 0.0, 0.0], false, true);
        for h in r["hits"].as_array().unwrap() { hits += 1; if h["destroyed"] == true { destroyed = true; } }
        if destroyed { assert_eq!(r["remove"].as_array().unwrap().len(), 1, "{r}"); break; }
        call(&mut e, json!({"cmd":"tick","dt":0.05}));
    }
    assert!(destroyed && hits == 8, "hits {hits}");
}

#[test]
fn every_piece_previews_and_the_display_matches_its_box() {
    let mut e = engine();
    for piece in ["wall", "floor", "stairs", "roof"] {
        let r = player(&mut e, Some(piece), [0.9, -0.43, 0.0], false, false);
        let p = &r["preview"][0];
        assert_eq!(p["kind"], piece, "{r}");
        assert!(!p["boxes"].as_array().unwrap().is_empty());
        for b in p["boxes"].as_array().unwrap() {
            let s: Vec<f64> = b["scale"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
            assert!(s.iter().all(|v| *v > 0.), "{b}");
            // the box centre is the entity position
            let (lo, hi) = display_aabb(b);
            for i in 0..3 { assert!(((lo[i] + hi[i]) * 0.5 - b["pos"][i].as_f64().unwrap()).abs() < 1e-3, "{piece} {b}"); }
        }
    }
}

#[test]
fn ramps_walk_with_minecraft_step_height_and_rise_384() {
    let mut e = engine();
    let r = player(&mut e, Some("stairs"), [0.9, -0.43, 0.0], true, false);
    let aabbs = r["spawn"][0]["aabbs"].as_array().expect("placed");
    let mut tops: Vec<f64> = aabbs.iter().map(|a| a[4].as_f64().unwrap()).collect();
    tops.sort_by(|a, b| a.total_cmp(b));
    assert!((tops.last().unwrap() - 3.84).abs() < 1e-3, "{tops:?}");
    for w in tops.windows(2) { assert!(w[1] - w[0] <= 0.6, "{tops:?}"); }
}

#[test]
fn a_floor_on_a_wall_collapses_when_the_wall_goes() {
    let mut e = engine();
    let look = [0.97, -0.24, 0.0];
    player(&mut e, Some("wall"), look, true, false);
    // a floor on top of the wall: aim higher
    let r = player(&mut e, Some("floor"), [0.6, 0.8, 0.0], true, false);
    assert_eq!(r["spawn"].as_array().map(|a| a.len()), Some(1), "floor placed: {r}");
    assert_eq!(e.piece_count(), 2);
    for _ in 0..100 { call(&mut e, json!({"cmd":"tick","dt":0.05})); }
    // remove the wall by pickaxe
    for _ in 0..400 { player(&mut e, None, [1., 0., 0.], false, true); call(&mut e, json!({"cmd":"tick","dt":0.05})); if e.piece_count() < 2 { break; } }
    // the floor is no longer supported: it falls within the collapse delays
    for _ in 0..40 { call(&mut e, json!({"cmd":"tick","dt":0.05})); }
    assert_eq!(e.piece_count(), 0);
}

#[test]
fn export_import_round_trip() {
    let mut e = engine();
    player(&mut e, Some("wall"), [0.97, -0.24, 0.0], true, false);
    let saved = call(&mut e, json!({"cmd":"export"}))["export"].clone();
    let mut f = engine();
    let r = call(&mut f, json!({"cmd":"import","pieces":saved}));
    assert_eq!(r["spawn"].as_array().unwrap().len(), 1, "{r}");
    let snap = call(&mut f, json!({"cmd":"snapshot"}));
    assert_eq!(snap["spawn"].as_array().unwrap().len(), 1);
}

#[test]
fn saved_pieces_keep_their_position_far_away() {
    let base = [-12_696_762., -12_464_777.];
    let mut e = engine();
    let r = player_at(&mut e, base, Some("wall"), [0.97, -0.24, 0.0], true, false);
    let before = r["spawn"][0]["aabbs"][0].clone();
    let saved = call(&mut e, json!({"cmd":"export"}))["export"].clone();
    let mut f = engine();
    let r = call(&mut f, json!({"cmd":"import","pieces":saved}));
    let after = r["spawn"][0]["aabbs"][0].clone();
    for i in 0..6 { assert!((before[i].as_f64().unwrap() - after[i].as_f64().unwrap()).abs() < 1e-3, "{before} vs {after}"); }
}
