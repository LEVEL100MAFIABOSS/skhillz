//! `cargo run -p fn-mechanics --features paks --bin fn-mechanics-bake -- <Fortnite 1.8 install> [out.json]`
//! Writes mechanics-data.json (or `out.json`): every table the mechanics need, so host games never open the paks.
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(install) = args.get(1) else {
        eprintln!("usage: fn-mechanics-bake <Fortnite 1.8 install dir> [out.json]");
        std::process::exit(2);
    };
    let out = args.get(2).map(String::as_str).unwrap_or("mechanics-data.json");
    let data = match fn_mechanics::bake::bake(std::path::Path::new(install)) {
        Ok(d) => d,
        Err(e) => { eprintln!("bake failed: {e}"); std::process::exit(1); }
    };
    if let Err(e) = data.save(out) { eprintln!("write failed: {e}"); std::process::exit(1); }
    println!("{out}: {} piece classes, {} edit classes, {} weapons, {} loot tier rows, {} loot package rows, {} traps",
        data.pieces.classes().count(), data.edit.classes.len(), data.weapons.len(), data.loot.tiers.len(), data.loot.packages.len(), data.traps.len());
    for n in &data.notes { println!("note: {n}"); }
}
