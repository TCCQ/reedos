/// This module provides wrappers for the hook proc macro in the other
/// crate. Namely it supplies types and whatnot

use alloc::boxed::Box;

/// What does an installed hook do? Should it mutate and pass on the arguements, or should it consume the call and return early?
pub enum HookReturn<I, O> {
    Compose(I),
    Consume(O),
}

pub enum HookError {}

// requires fully qualified path?
//
// leans heavily on type inference
macro_rules! insert_hook {
    ($loc:path, $hook:expr) => {
        unsafe {
            let mut held = ($loc).lock();
            held.push($hook);
        }
    }
}

pub fn test_insert() {
    let closure = |(first, second)| {
        log!(Debug, "Called from inside a hook! args: {}, {}", first, second);
        HookReturn::Consume(0xDEAD)
    };

    log!(Debug, "testing first call");
    crate::regular_function(0, 1);
    insert_hook!(crate::test_hook, Box::new(closure));
    log!(Debug, "testing second call");
    crate::regular_function(0,2);
}
