//! The kit used the way a host game uses it: through `fn_mechanics` only, no Fortnite files.
use fn_mechanics::building::damage::{BuildingDamage, Hit};
use fn_mechanics::building::grid::{self, Rot};
use fn_mechanics::building::pieces::{Material, PieceKind};
use fn_mechanics::building::placement::{self, PlacementEnv, QueryResult};
use fn_mechanics::building::structural::{FlatGround, StructuralEnv};
use fn_mechanics::building::world::BuildingWorld;
use fn_mechanics::glam::{Vec2, Vec3};
use fn_mechanics::hitreg::GunHit;
use fn_mechanics::movement::{params, Controller};
use fn_mechanics::rules::containers::{LootTables, PackageRow};
use fn_mechanics::rules::fall_damage::{fall_damage, FallTracker};
use fn_mechanics::rules::storm_zones::{SafeZones, ZoneEvent};
use fn_mechanics::{placeholder, MechanicsData};

#[test]
fn data_file_round_trips() {
    let mut gun = GunHit::default();
    gun.wid = "/Game/Athena/Items/Weapons/WID_Test".into();
    gun.stats.firing_rate = 5.;
    let loot = LootTables::from_parts(vec![], vec![PackageRow { name: "r".into(), id: "pkg".into(), weight: 1., call: String::new(), item: "/Game/X".into(), count: 3, category: 0 }]);
    let d = MechanicsData { source: "test".into(), pieces: placeholder::catalog(), weapons: vec![gun.clone()], loot, ..Default::default() };
    let text = serde_json::to_string(&d.to_json()).unwrap();
    let back = MechanicsData::from_str(&text).unwrap();
    assert_eq!(back.source, "test");
    assert_eq!(back.pieces.defs().len(), 12);
    for (a, b) in d.pieces.defs().iter().zip(back.pieces.defs()) {
        assert_eq!((a.kind, a.material, a.max_health, a.build_time, a.cost, a.class.cube), (b.kind, b.material, b.max_health, b.build_time, b.cost, b.class.cube));
        assert_eq!(a.class.convexes, b.class.convexes);
        assert!(back.pieces.class(&a.class.class_path).is_some(), "class index rebuilt");
    }
    assert_eq!(back.weapons, vec![gun]);
    assert_eq!(back.weapon("WID_Test").map(|g| g.stats.firing_rate), Some(5.));
    assert_eq!(back.loot.package_rows("pkg").count(), 1, "package index rebuilt");
    // a hand-written file with only some sections loads
    assert!(MechanicsData::from_str(r#"{"source":"mine"}"#).unwrap().weapons.is_empty());
    assert!(MechanicsData::from_str(r#"{"format":99}"#).is_err());
}

#[test]
fn placeholder_numbers_are_the_1_8_values() {
    let cat = placeholder::catalog();
    let d = |k, m| cat.def(k, m).unwrap();
    assert_eq!(d(PieceKind::Wall, Material::Wood).max_health, 200.);
    assert_eq!(d(PieceKind::Floor, Material::Metal).max_health, 370.);
    assert_eq!(d(PieceKind::Wall, Material::Wood).start_health(), 100.);
    assert_eq!(d(PieceKind::Stairs, Material::Stone).build_time, 12.);
}

#[test]
fn place_a_wall_and_pickaxe_it_down() {
    let cat = placeholder::catalog();
    let env = PlacementEnv { ground_z: Some(0.), ..Default::default() };
    let mut world = BuildingWorld::new();
    let def = cat.def(PieceKind::Wall, Material::Wood).unwrap();
    let (loc, rot) = grid::snap_transform(PieceKind::Wall, Vec3::new(300., 0., 50.), Rot::yaw(0.), 0);
    assert_eq!(placement::query(&world, &cat, def, loc, rot.yaw, false, &env), QueryResult::CanAdd);
    let id = placement::spawn_piece(&mut world, def, loc, rot.yaw, false, env.grid_origin);
    // the same slot is now taken
    assert!(matches!(placement::query(&world, &cat, def, loc, rot.yaw, false, &env), QueryResult::ExistingActor(_)));
    // let it finish building (health grows from 50 % to full over the build time)
    for _ in 0..(def.build_time / 0.1) as usize + 2 { world.tick(0.1); }
    assert_eq!(world.get(id).unwrap().health, 200.);
    let ground = FlatGround(Some(0.));
    let senv = StructuralEnv::with_collision(&ground);
    let mut dmg = BuildingDamage::new();
    let mut swings = 0;
    loop {
        swings += 1;
        let r = dmg.apply_hit(&mut world, id, Hit::Pickaxe { weak_spot: false }, &senv).unwrap();
        if r.destroyed { break; }
        assert!(swings < 100);
    }
    // 1.8 pickaxe vs a player building: 50 * 0.5 = 25 per hit -> 8 hits for a full wood wall
    assert_eq!(swings, 8);
    assert!(world.get(id).is_none());
}

#[test]
fn the_pawn_jogs_at_the_recovered_run_speed() {
    let mut c = Controller::default();
    let fwd = Vec2::new(c.facing.to_radians().sin(), -c.facing.to_radians().cos());
    for _ in 0..90 { c.step(fwd, false, false, 1. / 30.); }
    let speed_cm_s = c.velocity.length() * 100.;
    assert!((speed_cm_s - params::SPEED_RUN).abs() < 1., "{speed_cm_s}");
    for _ in 0..90 { c.step(fwd, false, true, 1. / 30.); }
    let sprint = c.velocity.length() * 100.;
    assert!((sprint - params::SPEED_SPRINT).abs() < 1., "{sprint}");
}

#[test]
fn a_thirty_metre_drop_hurts_per_the_athena_curve() {
    let mut c = Controller::default();
    c.step(Vec2::ZERO, false, false, 1. / 30.);
    c.height = 30.;
    let mut fall = FallTracker::default();
    let mut landed = vec![];
    for _ in 0..200 {
        fall.before_step(c.height, c.airborne());
        c.step(Vec2::ZERO, false, false, 1. / 30.);
        fall.after_step(c.height, c.airborne(), c.landings());
        landed.extend(fall.take_landings());
    }
    assert_eq!(landed.len(), 1, "{landed:?}");
    assert!((landed[0] - 3000.).abs() < 5., "{landed:?}");
    // 30 m is past the 23.04 m full-damage key: a fall from there takes everything (shield first)
    let hit = fall_damage(landed[0], 100., 50., 100.);
    assert!(hit.any());
    assert!(hit.shield_damage + hit.health_damage > 0.);
}

#[test]
fn standing_in_the_storm_ticks_damage() {
    let mut z = SafeZones::new(0., 3);
    let far = [-180_000., -150_000.];
    let mut dmg = 0;
    let mut t = 0.;
    while t < 70. { t += 0.05; for e in z.tick(t, Some(far)) { if matches!(e, ZoneEvent::Damage { .. }) { dmg += 1; } } }
    assert!(dmg >= 9, "{dmg}");
}
