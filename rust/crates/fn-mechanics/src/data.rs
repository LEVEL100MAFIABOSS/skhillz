//! `mechanics-data.json`: every table the mechanics read from Fortnite's paks, in one file a host game ships and loads.
//! Made by `fn-mechanics-bake` (feature `paks`) from a 1.8 install; a host can also write or edit it by hand (e.g. its own pieces).
use crate::building::edit::data::EditDb;
use crate::building::pieces::PieceCatalog;
use crate::hitreg::GunHit;
use crate::rules::containers::LootTables;
use crate::rules::traps::TrapDef;
use serde_json::{json, Value};
use std::path::Path;

/// Version of the file layout below.
pub const FORMAT: u32 = 1;

#[derive(Clone, Debug, Default)]
pub struct MechanicsData {
    /// Where the data came from (install path / "placeholder" / free text).
    pub source: String,
    /// Building pieces: the 12 (piece, material) defs with health, build time, cost, collision and connectivity, plus edit variants.
    pub pieces: PieceCatalog,
    /// Editing: every editable class, its tile pattern and the tile layouts.
    pub edit: EditDb,
    /// Every Battle Royale ranged weapon: stat row (damage, falloff, spread, recoil, fire rate...), fire ability, projectile.
    pub weapons: Vec<GunHit>,
    /// Athena loot tier / package tables (chests, ammo boxes, floor loot).
    pub loot: LootTables,
    /// Every trap definition (timings, damage / launch, target box).
    pub traps: Vec<TrapDef>,
    /// Anything that could not be read while baking.
    pub notes: Vec<String>,
}

impl MechanicsData {
    pub fn to_json(&self) -> Value {
        json!({
            "format": FORMAT,
            "source": self.source,
            "pieces": self.pieces.to_json(),
            "edit": self.edit.to_json(),
            "weapons": self.weapons.iter().map(GunHit::to_json).collect::<Vec<_>>(),
            "loot": self.loot.to_json(),
            "traps": serde_json::to_value(&self.traps).expect("traps serialize"),
            "notes": self.notes,
        })
    }

    /// Missing sections load as empty, so a hand-written file can carry only what the host uses.
    pub fn from_json(v: &Value) -> Result<MechanicsData, String> {
        let format = v.get("format").and_then(Value::as_u64).unwrap_or(FORMAT as u64);
        if format > FORMAT as u64 { return Err(format!("mechanics-data format {format} is newer than this library ({FORMAT})")); }
        let mut d = MechanicsData { source: v.get("source").and_then(Value::as_str).unwrap_or_default().to_owned(), ..Default::default() };
        if let Some(p) = v.get("pieces") { d.pieces = PieceCatalog::from_json(p)?; }
        if let Some(e) = v.get("edit") { d.edit = EditDb::from_json(e)?; }
        if let Some(w) = v.get("weapons").and_then(Value::as_array) { d.weapons = w.iter().map(GunHit::from_json).collect::<Result<_, _>>()?; }
        if let Some(l) = v.get("loot") { d.loot = LootTables::from_json(l)?; }
        if let Some(t) = v.get("traps") { d.traps = serde_json::from_value(t.clone()).map_err(|e| format!("traps: {e}"))?; }
        if let Some(n) = v.get("notes") { d.notes = serde_json::from_value(n.clone()).unwrap_or_default(); }
        Ok(d)
    }

    pub fn from_str(text: &str) -> Result<MechanicsData, String> {
        let v: Value = serde_json::from_str(text).map_err(|e| format!("mechanics-data: {e}"))?;
        MechanicsData::from_json(&v)
    }

    pub fn load(path: impl AsRef<Path>) -> Result<MechanicsData, String> {
        let p = path.as_ref();
        let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        MechanicsData::from_str(&text)
    }

    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), String> {
        let p = path.as_ref();
        let text = serde_json::to_string_pretty(&self.to_json()).map_err(|e| e.to_string())?;
        std::fs::write(p, text).map_err(|e| format!("{}: {e}", p.display()))
    }

    /// A weapon by its item definition package (`/Game/Athena/Items/Weapons/WID_...`) or the short `WID_...` name.
    pub fn weapon(&self, wid: &str) -> Option<&GunHit> {
        self.weapons.iter().find(|g| g.wid == wid || g.wid.rsplit('/').next() == Some(wid))
    }
}
