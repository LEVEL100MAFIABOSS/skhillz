//! # fn-mechanics: the Fortnite 1.8 gameplay mechanics as a drop-in library
//!
//! One dependency for a host game. Every module is pure Rust logic with no renderer, audio, window or OS dependency: the host feeds
//! input and time in and reads positions, events and numbers out, then draws / plays them however it likes.
//!
//! | module            | what it is                                                                                         |
//! |-------------------|----------------------------------------------------------------------------------------------------|
//! | [`movement`]      | the UE 4.16 `UCharacterMovementComponent` port: walk / sprint / crouch / jump / slopes / step-up, skydive + glider |
//! | [`camera`]        | the Athena third-person camera (boom, collision probes, ADS / melee modes)                         |
//! | [`combat`]        | pickaxe swing montage state machine, melee trace, harvesting, reticle                               |
//! | [`weapons`]       | weapon definitions, recoil, ADS, reload / ammo, quickbar and inventory rules                         |
//! | [`hitreg`]        | gun firing: spread / bloom, pellets, damage falloff, headshots, projectiles, explosives              |
//! | [`building`]      | build grid, placement, structural support + collapse, damage, editing, doors                         |
//! | [`rules`]         | match rules: fall damage, storm phases + damage, pickups, chests / loot rolls, traps                |
//! | [`net`]           | private-lobby wire protocol, snapshot interpolation, host-authoritative match state                  |
//! | [`hull`], [`terrain`] | convex-hull and heightfield collision the above use                                              |
//! | [`live`]          | `FN_*` tuning overrides (env vars or a tuning.toml), e.g. `FN_MOVE_SPEED_RUN`                        |
//!
//! Units: the movement controller and camera work in metres (Y-up for the camera); building, hit registration and the rules work in
//! UE centimetres (Z-up), as in the original. [`units`] has the conversions.
//!
//! Data: building, guns, loot and traps need tables that lived in Fortnite's paks. Bake them once into a JSON file
//! ([`data::MechanicsData`], feature `paks`, binary `fn-mechanics-bake`) and load that file in any game; or use
//! [`placeholder::catalog`] to try building without it.
pub use fn_building as building;
pub use fn_camera as camera;
pub use fn_combat as combat;
pub use fn_hitreg as hitreg;
pub use fn_hull as hull;
pub use fn_live as live;
pub use fn_movement as movement;
pub use fn_net as net;
pub use fn_rules as rules;
pub use fn_terrain as terrain;
pub use fn_weapons as weapons;
pub use glam;

pub mod data;
pub mod placeholder;
pub mod units;
#[cfg(feature = "paks")]
pub mod bake;

pub use data::MechanicsData;
