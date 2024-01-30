//! General riscv stuff

// #[cfg(feature = "riscv")]
// global_asm!(include_str!("../../asm/riscv/macro.s"));
// TODO I can't seem to get this to be universal like I want, but that's fine

// TODO this is technically not universal for riscv, and should
// instead be under it's own tag, or a riscv-qemu-virt tag or
// something
pub mod plic;

// -------------------------------------------------------------------
// Shim for misc busted riscv stuff

#[no_mangle]
pub fn fmod(_a: f64, _b: f64) -> f64 {
    todo!("No fmod support for riscv?");
}

#[no_mangle]
pub fn fmodf(_a: f32, _b: f32) -> f32 {
    todo!("No fmod support for riscv?");
}
