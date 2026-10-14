//! MECHANICS KIT: the item rarity helper traps need, copied from fn-game's `pickup_items` (that module also loads cosmetic meshes and
//! sounds, which a host game supplies itself).
/// `EFortRarity` names in enum order (index = rarity value).
pub const RARITY_NAMES: [&str; 10] = ["Handmade", "Ordinary", "Sturdy", "Quality", "Fine", "Elegant", "Masterwork", "Epic", "Badass", "Legendary"];
/// Rarity index of an `EFortRarity::<Name>` (or bare name); unknown names are 0.
pub fn rarity_of(v: &str) -> u8 { let t = v.rsplit("::").next().unwrap_or(v); RARITY_NAMES.iter().position(|n| *n == t).unwrap_or(0) as u8 }
