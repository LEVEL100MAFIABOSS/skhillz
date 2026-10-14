//! The smallest "host game" for fn-mechanics: a fixed 30 Hz loop that drives the player, camera, building, pickaxe, storm and fall damage.
//! Everything marked HOST is what your engine replaces (input polling, drawing, sound). Run it headless:
//!
//!     cargo run -p fn-mechanics --example minimal_host                        # placeholder building data
//!     cargo run -p fn-mechanics --example minimal_host -- mechanics-data.json # baked 1.8 data
use fn_mechanics::building::damage::{self, BuildingDamage, Hit};
use fn_mechanics::building::grid::Rot;
use fn_mechanics::building::pieces::{Material, PieceKind};
use fn_mechanics::building::placement::{self, BuildPreview, PlacementEnv, QueryResult, ViewInput};
use fn_mechanics::building::structural::{FlatGround, StructuralEnv};
use fn_mechanics::building::world::BuildingWorld;
use fn_mechanics::camera::{self, Camera};
use fn_mechanics::combat::{Notify, Swing, CHEST_HEIGHT, MELEE_RADIUS, MELEE_REACH};
use fn_mechanics::glam::{Vec2, Vec3};
use fn_mechanics::movement::Controller;
use fn_mechanics::rules::fall_damage::{fall_damage, FallTracker};
use fn_mechanics::rules::storm_zones::{health_after_storm_tick, SafeZones, ZoneEvent};
use fn_mechanics::{placeholder, units, MechanicsData};

const DT: f32 = 1. / 30.;

/// HOST: fill this from your keyboard / pad each tick.
#[derive(Default, Clone, Copy)]
struct Input { move_dir: Vec2, jump: bool, sprint: bool, fire: bool, build_wall: bool, look_yaw: f32, look_pitch: f32 }

/// A scripted player instead of a real one, so the example runs without a window.
fn script(t: f32) -> Input {
    let mut i = Input { look_pitch: -10., ..Default::default() };
    if t < 3. { i.move_dir = Vec2::new(0., 1.); i.sprint = t > 1.5; }  // jog, then sprint, forward
    if (1.0..1.1).contains(&t) { i.jump = true; }                       // one jump
    if (3.5..3.6).contains(&t) { i.build_wall = true; }                  // place a wall ahead (key held ~3 ticks)
    if t > 4. && t < 7. { i.move_dir = Vec2::new(0., 1.); }              // walk into it: the wall blocks the pawn
    if t > 6. && t < 12. { i.fire = true; }                             // hold fire: pickaxe swings
    i
}

fn main() {
    // --- data: baked tables if given, else the placeholder catalog ---
    let data = match std::env::args().nth(1) {
        Some(p) => MechanicsData::load(&p).unwrap_or_else(|e| panic!("{e}")),
        None => MechanicsData { source: "placeholder".into(), pieces: placeholder::catalog(), ..Default::default() },
    };
    println!("data: {} ({} piece defs, {} weapons)", data.source, data.pieces.defs().len(), data.weapons.len());

    // --- state the host owns ---
    let mut pawn = Controller::default();
    let mut cam = Camera::default();
    let mut swing = Swing::default();
    let mut fall = FallTracker::default();
    let mut storm = SafeZones::new(0., 7);
    let (mut health, mut shield, max_health) = (100f32, 50f32, 100f32);
    let mut world = BuildingWorld::new();
    let mut build_damage = BuildingDamage::new();
    let mut preview = BuildPreview::default();
    preview.set_piece(PieceKind::Wall);
    preview.material = Material::Wood;
    let env = PlacementEnv { ground_z: Some(0.), ..Default::default() };
    let ground = FlatGround(Some(0.));
    let mut wood = 100u32;
    let mut yaw = 0f32;
    let (mut build_held, mut hull_rev) = (false, u64::MAX);

    let mut t = 0f32;
    while t < 12. {
        t += DT;
        let input = script(t); // HOST: poll real input here

        // --- look + camera (metres, Y up) ---
        yaw += input.look_yaw;
        let pitch = input.look_pitch;
        let feet_game = units::controller_to_game(pawn.position, pawn.height);
        let view = cam.update(&camera::BASE, camera::Input { feet: feet_game, crouched: pawn.crouched, yaw_deg: yaw, pitch_deg: pitch, dt: DT },
            |a, b, r| camera::trace_ground(a, b, r)); // HOST: add your level's collision to this sweep

        // --- movement: the stick is relative to the camera ---
        let fwd = Vec2::new(view.forward.x, view.forward.z).normalize_or_zero();
        let right = Vec2::new(view.right.x, view.right.z).normalize_or_zero();
        let wish = fwd * input.move_dir.y + right * input.move_dir.x;
        // built pieces block the pawn: hand their collision to the controller (controller space = UE cm / 100)
        if world.revision() != hull_rev {
            hull_rev = world.revision();
            pawn.hulls = world.iter().flat_map(|p| placement::piece_hulls(&data.pieces, p)).map(|h| h.mapped(|v| v / units::CM_PER_M, false)).collect();
        }
        fall.before_step(pawn.height, pawn.airborne());
        pawn.update_facing(yaw, DT);
        pawn.step(wish, input.jump, input.sprint, DT);
        fall.after_step(pawn.height, pawn.airborne(), pawn.landings());
        for h in fall.take_landings() {
            let hit = fall_damage(h, health, shield, max_health);
            if hit.any() { health -= hit.health_damage; shield -= hit.shield_damage; println!("{t:5.2}s fall {:.0} cm: -{:.0}", h, hit.damage); }
        }

        // --- building (UE cm, Z up) ---
        world.tick(DT);
        let eye_ue = units::game_to_ue(view.eye);
        let f_ue = units::game_to_ue(view.forward).normalize();
        let rot = Rot { pitch: f_ue.z.asin().to_degrees(), yaw: f_ue.y.atan2(f_ue.x).to_degrees(), roll: 0. };
        let pawn_ue = units::controller_to_ue(pawn.position, pawn.height);
        let q = preview.update(&world, &data.pieces, &env, &ViewInput { camera: eye_ue, view: rot, pawn: pawn_ue });
        let build_press = input.build_wall && !build_held; // HOST: one placement per key press
        build_held = input.build_wall;
        if build_press {
            preview.prepare_place();
            match placement::place(&mut world, &data.pieces, &env, &preview, wood) {
                Ok((id, cost)) => { wood -= cost; println!("{t:5.2}s built wall #{id} at {:.0?} (wood left {wood})", preview.loc); }
                Err(r) => println!("{t:5.2}s cannot build: {r:?} (preview said {q:?})"),
            }
        }

        // --- pickaxe: the montage fires a hit trace at its TriggerAbility notify ---
        if let Some(Notify::TriggerAbility { .. }) = swing.step(input.fire, DT) {
            let senv = StructuralEnv::with_collision(&ground);
            // the melee sweep starts at the chest and reaches MELEE_REACH along the pawn's horizontal aim (fn-combat constants, metres)
            let chest = pawn_ue + Vec3::Z * CHEST_HEIGHT * units::CM_PER_M;
            let aim = Vec3::new(f_ue.x, f_ue.y, 0.).normalize_or_zero();
            if let Some((id, _, _)) = damage::trace(&world, chest, aim, MELEE_REACH * units::CM_PER_M, MELEE_RADIUS * units::CM_PER_M) {
                if let Some(r) = build_damage.apply_hit(&mut world, id, Hit::Pickaxe { weak_spot: false }, &senv) {
                    println!("{t:5.2}s pickaxe hit #{id}: -{} -> {:.0}{}", r.damage, r.health, if r.destroyed { " DESTROYED" } else { "" });
                }
            }
        }

        // --- storm (UE cm, the pawn's X/Y) ---
        for e in storm.tick(t, Some([pawn_ue.x, pawn_ue.y])) {
            match e {
                ZoneEvent::Damage { fraction } => { health = health_after_storm_tick(health, max_health, fraction); }
                other => println!("{t:5.2}s storm: {other:?}"),
            }
        }

        // HOST: draw the pawn at units::controller_to_game(pawn.render_position(), pawn.render_height()), the camera at view.eye/forward,
        // world.iter() pieces with their class mesh, the preview ghost at preview.loc / preview.rot (green when q == CanAdd).
        let _ = q == QueryResult::CanAdd;
    }
    println!("end: pawn at {:.2?} m, speed {:.2} m/s, health {health:.0}, shield {shield:.0}, pieces standing {}", pawn.position, pawn.velocity.length(), world.len());
}
