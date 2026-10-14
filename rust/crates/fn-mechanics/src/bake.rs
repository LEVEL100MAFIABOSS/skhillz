//! Feature `paks`: reads every table [`MechanicsData`] holds from a Fortnite 1.8 install, with the original runtime's loaders.
use crate::building::edit::data::EditDb;
use crate::building::pieces::PieceCatalog;
use crate::data::MechanicsData;
use crate::hitreg::pakload::ClipSourceDir;
use crate::rules::containers::LootTables;
use std::path::Path;

/// `install` = the Fortnite 1.8 install (the folder holding `FortniteGame`, or the `Paks` folder itself).
pub fn bake(install: &Path) -> Result<MechanicsData, String> {
    let set = fn_pak::PakSet::open_install(install).map_err(|e| format!("{}: {e}", install.display()))?;
    let mut notes = Vec::new();
    let mut pieces = PieceCatalog::load(&set)?;
    let edit = EditDb::load(&set)?;
    // edit variants: load their collision and cube now, so the host never needs the paks
    for c in &edit.classes { pieces.ensure_class(&set, &c.class_path); }
    notes.append(&mut pieces.notes);
    let src = ClipSourceDir::Pak(&set);
    let mut weapons = Vec::new();
    for wid in crate::hitreg::br_weapon_wids(&set) {
        match crate::hitreg::load(&src, &wid) { Ok(g) => weapons.push(g), Err(e) => notes.push(format!("{wid}: {e}")) }
    }
    let loot = LootTables::load(&set).unwrap_or_else(|e| { notes.push(format!("loot tables: {e}")); LootTables::default() });
    let (traps, mut trap_notes) = crate::rules::traps::load_all(&set);
    notes.append(&mut trap_notes);
    Ok(MechanicsData { source: install.display().to_string(), pieces, edit, weapons, loot, traps, notes })
}
