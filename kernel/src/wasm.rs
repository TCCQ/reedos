//! This module should contain all the stuff for wrapping the wasm
//! execution evironment that makes up our safe extension support.

use alloc::boxed::Box;

extern crate wasmi;
use wasmi::*;

use wasmi::errors::*;

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
