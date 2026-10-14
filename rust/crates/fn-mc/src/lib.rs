//! # fn-mc: the Minecraft bridge
//!
//! The fn-mechanics building and pickaxe mechanics behind a small JSON command API ([`engine::Cmd`] in, [`engine::Out`] back), built as a
//! WebAssembly module (`wasm32-unknown-unknown`) that the Fabric mod runs with Chicory, so one mod jar works on every server OS.
//!
//! Wasm ABI (all integers are i32 / i64):
//! * `fnmc_new() -> handle` / `fnmc_drop(handle)`: one engine per Minecraft dimension;
//! * `fnmc_alloc(len) -> ptr` / `fnmc_free(ptr, len)`: buffers in the module's memory;
//! * `fnmc_call(handle, ptr, len) -> i64`: run the UTF-8 JSON command at `ptr..ptr+len` (the module frees it); returns `(out_ptr << 32) | out_len`
//!   of the JSON answer, which the caller frees with `fnmc_free(out_ptr, out_len)`.
//! * import `fnmc.solid_box(min_x, min_y, min_z, max_x, max_y, max_z: f64) -> i32`: 1 when any block with collision intersects the box.
pub mod engine;
pub mod host;
pub mod shapes;
pub mod space;

pub use engine::{Cmd, Engine, Out};

use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static ENGINES: RefCell<(i32, HashMap<i32, Engine>)> = RefCell::new((1, HashMap::new()));
}

#[no_mangle]
pub extern "C" fn fnmc_new() -> i32 {
    ENGINES.with(|e| { let mut e = e.borrow_mut(); let h = e.0; e.0 += 1; e.1.insert(h, Engine::default()); h })
}

#[no_mangle]
pub extern "C" fn fnmc_drop(handle: i32) { ENGINES.with(|e| { e.borrow_mut().1.remove(&handle); }); }

#[no_mangle]
pub extern "C" fn fnmc_alloc(len: i32) -> i32 {
    let mut v: Vec<u8> = Vec::with_capacity(len.max(1) as usize);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p as usize as i32
}

/// # Safety
/// `ptr` / `len` must come from `fnmc_alloc` or a `fnmc_call` answer.
#[no_mangle]
pub unsafe extern "C" fn fnmc_free(ptr: i32, len: i32) {
    if ptr == 0 { return; }
    drop(Vec::from_raw_parts(ptr as usize as *mut u8, 0, len.max(1) as usize));
}

/// # Safety
/// `ptr..ptr+len` must be a buffer from `fnmc_alloc` holding UTF-8 JSON; it is freed here.
#[no_mangle]
pub unsafe extern "C" fn fnmc_call(handle: i32, ptr: i32, len: i32) -> i64 {
    let input = Vec::from_raw_parts(ptr as usize as *mut u8, len as usize, len.max(1) as usize);
    let text = String::from_utf8_lossy(&input).into_owned();
    drop(input);
    let answer = ENGINES.with(|e| match e.borrow_mut().1.get_mut(&handle) {
        Some(engine) => engine.handle_json(&text),
        None => "{\"error\":\"no such engine\"}".to_owned(),
    });
    let mut bytes = answer.into_bytes();
    bytes.shrink_to_fit();
    let (p, l) = (bytes.as_mut_ptr() as usize as i64, bytes.len() as i64);
    let cap = bytes.capacity();
    std::mem::forget(bytes);
    debug_assert_eq!(cap as i64, l.max(1).max(cap as i64));
    (p << 32) | (l & 0xffff_ffff)
}
