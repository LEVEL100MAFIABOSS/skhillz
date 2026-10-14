//! MECHANICS KIT: the three `fn-pose` helpers hit registration used to read cooked packages (`loader::package`,
//! `loadout_stats::data_table_row`, `ClipSourceDir`), copied here so the crate no longer depends on the animation stack.
//! Only built with the `paks` feature (baking mechanics-data.json from a 1.8 install).
use fn_ueio::props::{read_props, Ctx, PropList};
use fn_ueio::{Package, Reader};
use std::path::PathBuf;

/// Where cooked packages come from: an extracted `FortniteGame/Content` tree or an open pak set.
pub enum ClipSourceDir<'a> {
    RawPackages(PathBuf),
    Pak(&'a fn_pak::PakSet),
}

/// Parses a package by long name (`/Game/...`) from raw files / the pak.
pub fn package(src: &ClipSourceDir, long: &str) -> Result<Package, String> {
    match src {
        ClipSourceDir::RawPackages(root) => {
            let base = root.join("FortniteGame/Content").join(long.trim_start_matches("/Game/"));
            let rd = |e: &str| std::fs::read(format!("{}.{e}", base.display())).unwrap_or_default();
            let a = std::fs::read(format!("{}.uasset", base.display())).map_err(|e| format!("{}: {e}", base.display()))?;
            Package::parse(&a, &rd("uexp"), &rd("ubulk")).map_err(|e| format!("{long}: {e}"))
        }
        ClipSourceDir::Pak(set) => {
            let a = set.read_game(long, "uasset").map_err(|e| format!("{long}: {e}"))?;
            let x = set.read_game(long, "uexp").map(|c| c.into_owned()).unwrap_or_default();
            let b = set.read_game(long, "ubulk").map(|c| c.into_owned()).unwrap_or_default();
            Package::parse(&a, &x, &b).map_err(|e| format!("{long}: {e}"))
        }
    }
}

fn package_name(r: &str) -> &str { r.split('.').next().unwrap_or(r) }

/// One row of a `DataTable` export: `i32 n`, then `n x { RowName FName, tagged property list }` after the table's own tagged properties.
pub fn data_table_row(src: &ClipSourceDir, table_package: &str, row: &str) -> Result<PropList, String> {
    let pkg = package(src, package_name(table_package))?;
    let idx = pkg.find_export("DataTable").ok_or_else(|| format!("{table_package}: no DataTable export"))?;
    let (start, end) = pkg.export_range(idx);
    let ctx = Ctx { names: &pkg.names, file_version: pkg.summary.file_version };
    let mut r = Reader::at(&pkg.data[..end], start);
    read_props(&mut r, &ctx).map_err(|e| format!("{table_package}: {e}"))?;
    // UObject::Serialize ends with the object-guid flag (a 4-byte bool; a 16-byte guid follows when set), then the table's own data.
    if r.i32().map_err(|e| e.to_string())? != 0 { r.bytes(16).map_err(|e| e.to_string())?; }
    let n = r.i32().map_err(|e| e.to_string())?;
    if !(0..=100_000).contains(&n) { return Err(format!("{table_package}: implausible row count {n}")); }
    for _ in 0..n {
        let name = r.i32().map_err(|e| e.to_string())?;
        let number = r.i32().map_err(|e| e.to_string())?;
        let base = pkg.names.get(usize::try_from(name).map_err(|_| "negative row name index")?).ok_or("row name out of bounds")?;
        let props = read_props(&mut r, &ctx).map_err(|e| format!("{table_package}: row {base}: {e}"))?;
        let full = if number > 0 { format!("{base}_{}", number - 1) } else { base.clone() };
        if full == row { return Ok(props); }
    }
    Err(format!("{table_package}: no row {row} (of {n})"))
}
