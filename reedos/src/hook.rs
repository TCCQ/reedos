/// This module provides wrappers for the hook proc macro in the other
/// crate. Namely it supplies types and whatnot

/// What does an installed hook do? Should it mutate and pass on the arguements, or should it consume the call and return early?
pub enum HookReturn<I, O> {
    Compose(I),
    Consume(O),
}

