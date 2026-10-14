# Changes versus the original FortniteRuntime-rs crates

Identical copies: `fn-live`, `fn-hull`, `fn-terrain`, `fn-weapons`, `fn-net`, `fn-pak`, `fn-ueio`, `fn-mesh`.

Test-only: `fn-movement` (3), `fn-camera` (3), `fn-combat` (6) and `fn-testdata` (1) parity tests that read the old FortniteRuntime tree's
reports are marked `#[ignore]` (run them with `--ignored` and `FN_OLD_ROOT`). `fn-uasset`: the unused `rayon` dev-dependency is dropped.

`fn-building`
- Feature `paks` (off by default) holds every function that reads the paks (`PieceCatalog::load`, `EditDb::load`, curve / class / mesh loaders,
  visual curve loaders) and the tests that need `FN_TEST_ROOT`. `fn-pak`, `fn-uasset`, `fn-mesh`, `fn-ueio` became optional dependencies.
- Serde derives on the data types (`PieceKind`, `Material`, `ClassData`, `PieceDef`, `PieceCatalog`, `EditDb`, `EditClass`, `ClassBuild`,
  `TileLayout`, `FlashCurve`, `Pattern`, `Tiles`); `PieceCatalog::{from_parts, insert_class, classes, to_json, from_json}`, `EditDb::{to_json, from_json}`.
- Removed the PNG verification tools (`examples/core_diagram.rs`, the `verification_diagram` test); they stay in the original repo.

`fn-hitreg`
- Depends on `src/pakload.rs` (feature `paks`) instead of `fn-pose`: a copy of the three helpers it used (`loader::package`,
  `loadout_stats::data_table_row`, `ClipSourceDir`), so the animation stack is no longer pulled in.
- Feature `paks` holds the WID / projectile / explosive / surface / body-setup loaders, their tests and the two dev binaries.
- Serde derives on `GunHit`, `StatRow`, `FireAbility`, `TargetSelection`, `Presentation`, `ProjectileData`, `ExplosiveDef`, `Kind`, `BodySetup`,
  `Sphyl`, `PropSurface`; `GunHit::{to_json, from_json}`.

`fn-rules` (new crate, files moved from `fn-game`)
- `fall_damage.rs`, `storm_state.rs`, `storm_zones.rs`, `storm_volumes.rs`, `pickups.rs` unchanged apart from `paks` gates on pak tests.
- `containers.rs`, `traps.rs`: pak loaders behind feature `paks`, serde derives on the table types, `LootTables::{from_parts, to_json, from_json}`.
- `rarity.rs`: `rarity_of` / `RARITY_NAMES` copied from `fn-game/pickup_items.rs` (the rest of that file loads cosmetics).
- Not moved: `sim.rs` / `sim_tick.rs` / `drop_in.rs` / `jump_air.rs` / `ads_view.rs` (they drive the animation graph and settings crates).

`fn-mechanics` (new crate): re-exports, `MechanicsData` (mechanics-data.json), `units`, `placeholder`, `bake` + `fn-mechanics-bake`
(feature `paks`), `examples/minimal_host.rs`, `tests/kit.rs`.
