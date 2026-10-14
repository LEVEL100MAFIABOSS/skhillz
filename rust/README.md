# fn-mechanics kit

The gameplay core of FortniteRuntime-rs (the Fortnite 1.8 rewrite), cut loose from its renderer, audio, UI and asset pipeline, so
another Rust game can use it as a library or mod. Same language and same code as the original: the crates are copies, changed
only where [CHANGES.md](CHANGES.md) says.

## What you get

| crate | mechanics |
|---|---|
| `fn-movement` | UE 4.16 `CharacterMovementComponent` port: jog 410 / sprint 550 / crouch / jump / slopes / step-up / perching, skydive + glider |
| `fn-camera` | Athena third-person camera: boom, collision probes, pitch-dependent offsets, ADS and melee modes |
| `fn-combat` | pickaxe swing montage (combo windows, notify timings), melee sweeps, harvesting, reticle |
| `fn-weapons` | weapon definitions, recoil, ADS, reload / ammo, quickbar + inventory rules |
| `fn-hitreg` | gun firing: bloom / spread, pellets, damage falloff, headshot zones, projectiles, rockets + grenades, building damage |
| `fn-building` | 512 x 384 build grid, preview + placement, structural support and collapse, piece damage, editing, doors |
| `fn-rules` | fall damage, storm phases + storm damage, pickups, chests / ammo boxes + loot rolls, traps |
| `fn-net` | private-lobby wire protocol, snapshot interpolation, host-authoritative match state |
| `fn-hull`, `fn-terrain` | convex-hull and heightfield collision used by the above |
| `fn-live` | `FN_*` tuning overrides (env vars or a `tuning.toml`, see `tuning.example.toml`) |
| **`fn-mechanics`** | **the one dependency a game adds**: re-exports all of the above, the data file, unit conversions, a placeholder building catalog |

Not included on purpose (your game brings its own): rendering, the animation graph / poses, audio, UI / HUD, cosmetics, meshes and textures.
The original repo keeps all of those.

## Add it to a game

```toml
# your game's Cargo.toml
[dependencies]
fn-mechanics = { path = "path/to/mechanics-kit/crates/fn-mechanics" }
```

```rust
use fn_mechanics::movement::Controller;
use fn_mechanics::glam::Vec2;

let mut pawn = Controller::default();
// every tick (the original runs a fixed 30 Hz tick):
pawn.step(Vec2::new(0., 1.), /*jump*/ false, /*sprint*/ true, 1. / 30.);
```

`crates/fn-mechanics/examples/minimal_host.rs` is a complete host loop (about 150 lines) that wires movement, camera, building, pickaxe, storm and
fall damage together, with `HOST:` comments where your engine's input / drawing goes. Run it headless:

```
cargo run -p fn-mechanics --example minimal_host
```

Expected output: a wall is built 4.5 m ahead (10 wood), the pawn walks into it and is blocked, eight pickaxe swings destroy it.

### Per-tick order (what the example does)

1. read input
2. `camera.update(...)`: camera yaw / pitch / eye
3. movement direction from the camera's forward / right, then `controller.step(...)`; give it `hulls` for anything solid (built pieces, your level)
4. `FallTracker` before / after the step, then `fall_damage(...)` for each landing
5. `BuildingWorld::tick`, `BuildPreview::update`, `placement::place` on the build key
6. `Swing::step(fire)`; on `Notify::TriggerAbility` trace with `damage::trace` and `BuildingDamage::apply_hit`
7. `SafeZones::tick(now, pawn)` and `health_after_storm_tick` on `ZoneEvent::Damage`
8. draw (your engine)

### Units

Controller and camera: metres. Building, guns, rules: UE centimetres, Z up. `fn_mechanics::units` converts
(`controller_to_ue`, `game_to_ue`, `ue_to_game`, ...).

## Data: bake once, ship a JSON file

Building pieces, edit patterns, every BR gun's stat row, loot tables and traps lived in the 1.8 `.pak` files. The kit can read
them (the original loaders, behind the `paks` feature) and write one file your game loads instead:

```
cargo run -p fn-mechanics --features paks --bin fn-mechanics-bake -- "<Fortnite 1.8 install>" mechanics-data.json
```

```rust
let data = fn_mechanics::MechanicsData::load("mechanics-data.json")?;
let wall = data.pieces.def(PieceKind::Wall, Material::Wood);
let pump = data.weapon("WID_Shotgun_Standard_Athena_C_Ore_T03");
```

Without baked data, `fn_mechanics::placeholder::catalog()` gives the 12 build pieces with the real 1.8 health / build-time / cost
numbers, but only the wall has the real collision box; the floor, stairs and roof use stand-in shapes. Every section of the file is
optional, so you can also write your own pieces, weapons or loot by hand in the same format (`MechanicsData::to_json` shows it).

Note: a baked `mechanics-data.json` is derived from Epic's game files. Think about that before you publish it with a game.

## Tests

```
cargo test --workspace                          # 433 pass, no Fortnite files needed
cargo test --workspace --features fn-mechanics/paks
FN_TEST_ROOT="<1.8 install>" cargo test --workspace --features fn-mechanics/paks   # + the tests against the real paks
FN_OLD_ROOT="<old FortniteRuntime tree>" cargo test --workspace -- --ignored          # + the parity tests vs. the old reports
```
