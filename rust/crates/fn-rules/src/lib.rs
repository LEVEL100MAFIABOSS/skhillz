//! MECHANICS KIT: the Battle Royale match rules of the 1.8 runtime, lifted out of `fn-game` (its app facade) so they can run in any game.
//! Pure logic, no renderer / audio / OS. Each module is the original file unchanged except where marked `MECHANICS KIT`.
//!
//! * [`fall_damage`]   `AthenaFallingDamage` curve, falling-start apex, shield-first damage split
//! * [`storm_state`] / [`storm_zones`] / [`storm_volumes`]  safe zone phases (radii, timings), zone state and storm volumes
//! * [`pickups`]       world item logic: toss physics, touch / Use pickup, fly-to-pawn
//! * [`containers`]    chests / ammo boxes: search, spawn selection, Athena loot roll (tables from mechanics-data.json or, feature `paks`, a 1.8 install)
//! * [`traps`]         trap arm / fire / damage / reload timings (stats from mechanics-data.json or, feature `paks`, a 1.8 install)
//! * [`rarity`]        `EFortRarity` names
pub mod containers;
pub mod fall_damage;
pub mod pickups;
pub mod rarity;
pub mod storm_state;
pub mod storm_volumes;
pub mod storm_zones;
pub mod traps;
