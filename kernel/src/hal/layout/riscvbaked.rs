//! riscv linkerscript baked in locations.

use core::ptr::addr_of_mut;
// use core::cell::OnceCell;

pub type ProxyLoc = ();
pub static LOCATIONS: ProxyLoc = ();

// -------------------------------------------------------------------
// These are supplied by the linkerscript at the moment and is thus
// stateless, fully compiletime, and does not require device tree
// parsing or any other kind of discovery. This method does assume
// that the memory placement is known at compile time (seems
// reasonable for a kernel), and that the linkerscript assumptions
// about memory are accurate. Thus at the moment no one compilation
// will work for multiple boards. But at the moment it's a good
// middleground.

// This ugly two macro setup is gross, but I can't use just one to do
// both cause each invocation would need to straddle both. And you
// can't split impls so there is no workaround here.
macro_rules! linker_var {
    (
        $linker_name: ident
    ) => {
        extern "C" { static mut $linker_name: usize; }
    }
}

macro_rules! trait_wrapper {
    (
        $linker_name: ident,
        $rust_name: ident
    ) => {
        #[doc="Get the associated linker variable as a pointer"]
        fn $rust_name(&self) -> *mut usize {
            unsafe {addr_of_mut!($linker_name)}
        }
    }
}

linker_var!(_text_start);
linker_var!(_text_end);

linker_var!(_bss_start);
linker_var!(_bss_end);

linker_var!(_rodata_start);
linker_var!(_rodata_end);

linker_var!(_data_start);
linker_var!(_data_end);

linker_var!(_stacks_start);
linker_var!(_stacks_end);

linker_var!(_intstacks_start);
linker_var!(_intstacks_end);

linker_var!(_memory_start);
linker_var!(_memory_end);

impl super::HALSections for ProxyLoc {
    fn sections_setup(&self) {
        // This is intentionally empty
    }

    trait_wrapper!(_text_start, text_start);
    trait_wrapper!(_text_end, text_end);

    trait_wrapper!(_bss_start, bss_start);
    trait_wrapper!(_bss_end, bss_end);

    trait_wrapper!(_rodata_start, rodata_start);
    trait_wrapper!(_rodata_end, rodata_end);

    trait_wrapper!(_data_start, data_start);
    trait_wrapper!(_data_end, data_end);

    trait_wrapper!(_stacks_start, stacks_start);
    trait_wrapper!(_stacks_end, stacks_end);

    trait_wrapper!(_intstacks_start, intstacks_start);
    trait_wrapper!(_intstacks_end, intstacks_end);

    trait_wrapper!(_memory_start, memory_start);
    trait_wrapper!(_memory_end, memory_end);
}
