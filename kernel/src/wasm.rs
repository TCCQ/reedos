//! This module contains information mostly. I see no way to really
//! "wrap" or otherwise generalize the functionality that wasmi
//! provides without having to re-invent everything that wasmi does,
//! or re-export everything that it uses. In that case, it is better
//! to be transparent about what is being called / used, so users
//! should just import wasmi themselves.

// attempt to provide niceties to cover repeated use cases

// TODO do we need to provide Dynamic memory to the internals of wasm? From a wasm perspective, no, since it is against the point. But from a kernel extension perspective, probably. Where should owned PhysPageExtents be stored? In the instance Store?

//! Things to know. Stolen mostly from the wasmi docs's example, as comprehensive docs are scarce
//!
//! A key insight is that despite providing virtuallization,
//! currently, there is no async / parallel nature to this setup. So
//! if a wasm module's start contains a waiting loop, you are
//! softlocked. The mantra for usage of wasm should be "on demand". At
//! least until we have some sort of kernel async or something going.
//!
//! The parts of running sandboxed wasm are:
//!
//! Engine: Defines what happens under the hood. Probably you want the default.
//! Use wasmi::Engine::default() to acquire one
//!
//! Module: A module is the basic block of wasm. With engine in hand you can compile and validate a *bytecode only* wasm module with
//! wasmi::Module::new(&engine, {something readable. Such as a slice reference})
//!
//! Store: Data shared between host and wasm are located in Stores. All Wasm objects operate in a store.
//! Stores can be acquired with wasmi::Store::new(&engine, initial_val)
//!
//! To access host state in a store, wasmi::Func::wrap can be used to expose a host function suitable to be called as an import in the wasm module.
//! See the example in test_wasm.
//! TODO why do we need stores? I guess to seperate rust globals from wasm module specific stuff? Host functions via wrap should still be able to effect global scope and call out to other parts of the host rust environment.
//!
//! To instantiate a module, we need to link the imports, then fetch exports.
//! To do that we need a wasmi::linker obtained with a <Linker<matching_store_internal_type>>::new(&engine)
//! Then we can linker.define("module_name", "import_name", wrapped (likely function) value)
//!
//! An instance is a sandboxed copy of a abstract module that is verified and has everything it needs to be run.
//! Obtain one with linker.instatiate(&mut store, &module)
//! Start it with (prev_line).start(&mut store)
//!
//! In the reverse direction, the host can call into the module by obtaining a handle with
//! instance.get_tpyed_func::<[OMITED TYPE NONSENSE]>(&store, "function_name")
//! and fire it off with
//! function_handle.cal(&mut store, func_arg1)

extern crate wasmi;
use wasmi::*;

// -------------------------------------------------------------------
//
// Wasmi tests

// This is the test given by wasmi docs, which doesn't even work. We need to compile our wasm externally first I guess.

pub fn test_wasm() {
    // First step is to create the Wasm execution engine with some config.
    // In this example we are using the default configuration.
    let engine = Engine::default();
    // let wat = r#"
    //     (module
    //         (import "host" "hello" (func $host_hello (param i32)))
    //         (func (export "hello")
    //             (call $host_hello (i32.const 3))
    //         )
    //     )
    // "#;
    // // Wasmi does not yet support parsing `.wat` so we have to convert
    // // out `.wat` into `.wasm` before we compile and validate it.
    // let wasm = wat::parse_str(&wat)?;
    let wasm = include_bytes!("wasm_files/hello_test.wasm");
    let module = match Module::new(&engine, &wasm[..]) {
        Ok(m) => m,
        Err(_) => panic!("wasm module init failed"),
    };

    // All Wasm objects operate within the context of a `Store`.
    // Each `Store` has a type parameter to store host-specific data,
    // which in this case we are using `42` for.
    type HostState = u32;
    let mut store = Store::new(&engine, 42);
    let host_hello = Func::wrap(&mut store, |caller: Caller<'_, HostState>, param: i32| {
        println!("Got {:?} from WebAssembly", param);
        println!("My host state is: {}", caller.data());
    });

    // In order to create Wasm module instances and link their imports
    // and exports we require a `Linker`.
    let mut linker = <Linker<HostState>>::new(&engine);
    // Instantiation of a Wasm module requires defining its imports and then
    // afterwards we can fetch exports by name, as well as asserting the
    // type signature of the function with `get_typed_func`.
    //
    // Also before using an instance created this way we need to start it.
    match linker.define("host", "hello", host_hello) {
        Ok(_) => {},
        Err(_) => panic!("wasm linker define failed"),
    };

    let mut catch = || {
        Ok::<Instance, Error>(linker
            .instantiate(&mut store, &module)?
            .start(&mut store)?)
    };
    let instance = match catch() {
        Ok(i) => i,
        Err(_) => panic!("wasm linker inst or start error"),
    };
    let hello = match instance.get_typed_func::<(), ()>(&store, "hello") {
        Ok(f) => f,
        Err(_) => panic!("wasm failed to get typed function"),
    };

    // And finally we can call the wasm!
    match hello.call(&mut store, ()) {
        Ok(_) => {},
        Err(_) => panic!("failed to call wasm. Should have printed"),
    };
    log!(Debug, "Previous line should have been from inside WebAssembly");
}
