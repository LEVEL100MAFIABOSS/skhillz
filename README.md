# FN Mechanics for Minecraft (26.3, Fabric)

Fortnite 1.8 building and pickaxe mechanics from FortniteRuntime-rs, running inside Minecraft Java 26.3. The Rust mechanics kit
(`rust/`) is compiled to WebAssembly and runs inside the mod with Chicory, so the mechanics are the same tested code as the kit,
and one jar works on any server OS.

Fortnite pieces and Minecraft blocks co-exist: pieces snap to Fortnite's own 5.12 m x 3.84 m grid, Minecraft blocks count as ground
that supports them, and pieces never replace or break blocks.

## Install (server and every player)

1. Fabric Loader 0.19.5+ for Minecraft 26.3, and Fabric API 0.162.0+26.3.
2. `fn-mechanics-mc-<version>.jar` in the `mods` folder of the server **and** of each player.
3. Java 25.

## Controls (rebind under Options > Controls > Key Binds > FN Mechanics)

| Key | Action |
|---|---|
| Z / X / C / V | Wall / Floor / Ramp / Cone: enters build mode with that piece |
| Left click (build mode) | Place the piece shown by the glass preview (blue = can place, red = can't) |
| Right click (build mode) | Next material: Wood, Brick, Metal |
| R | Rotate |
| B | Leave build mode |
| Hold attack with any pickaxe | Fortnite pickaxe swings on pieces in front of you (25 damage per hit to player builds) |

Pieces build up over time like in Fortnite (wood 4 s, brick 12 s, metal 20 s), have 1.8 health (walls 200 / 300 / 400), and anything
left without support collapses.

## Commands

| Command | |
|---|---|
| `/fnmc info` | Piece count and the building grid height here |
| `/fnmc grid here` / `/fnmc grid <y>` | Put building level 0 at your feet / at height y (default: where the first person builds) |
| `/fnmc clear` | Remove every piece in this dimension |

Pieces are saved per dimension in `<world>/fnmc/*.json`.

## Real 1.8 data (optional)

By default the mod uses the kit's placeholder piece catalog (real 1.8 health / build times / costs). For the baked tables, run the
kit's `fn-mechanics-bake` (see `rust/README.md`) and put `mechanics-data.json` at `config/fnmc/mechanics-data.json` on the server.

## Building it

GitHub Actions (`.github/workflows/build.yml`) does everything and runs the game tests in a real 26.3 server; the jar is the `fn-mechanics-mc`
artifact of the run. Locally:

```
cd rust
rustup target add wasm32-unknown-unknown
cargo build -p fn-mc --release --target wasm32-unknown-unknown
copy target\wasm32-unknown-unknown\release\fn_mc.wasm ..\src\main\resources\fnmc\
cd ..
gradlew build
```

## How it fits together

| Part | Where |
|---|---|
| Mechanics (building, structural collapse, damage, pickaxe montage) | `rust/crates/fn-building`, `fn-combat` (unchanged kit code) |
| Bridge: JSON commands in, spawn / remove / hit / preview events out; piece shapes in Minecraft space | `rust/crates/fn-mc` |
| WebAssembly host (Chicory) + block query callback | `src/main/java/net/fnmc/wasm/WasmEngine.java` |
| Per-dimension world: block displays, sounds, saving | `src/main/java/net/fnmc/server/BuildManager.java` |
| Piece collision for every entity (client and server) | `Colliders.java` + `mixin/EntityCollideMixin.java` |
| Keys and input | `src/client/java/net/fnmc/client/FnMcClient.java` |
| Game tests (run by `gradlew build`) | `src/gametest/java/net/fnmc/test/FnMcGameTest.java` |

## Not yet

Editing (G), guns, storm, fall damage, materials cost (building is free for now), piece health bars. Pieces are drawn as block displays,
so ramps and cones look like tilted / stepped boxes.
