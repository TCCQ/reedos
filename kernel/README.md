# reedos

You could say reed-oh-ESS, but we like to think it's barely FreeDOS minus the
'f'. Like rEE-doss.

See [Contribution Guidelines](CONTRIBUTING.md) if you're interested in getting
involved.

### Notes

We currently support Rust's `GlobalAlloc` in order to use the `alloc` crate. We
do so by wrapping page allocation and finer grained virtual memory allocation
into a `Global Allocator` struct which implements Rust's `GlobalAlloc` trait. As
an example, this is valid `reedos` kernel code:

```rust
use alloc::collections;
{
    // Simple test. It works!
    let mut one = Box::new(5);

    // Slightly more interesting... it also works! Look at this with GDB
    // and watch for the zone headers + chunk headers indicating 'in use' and
    // 'chunk size'. Then watch the headers as these go out of scope.
    let mut one_vec: Box<collections::VecDeque<u32>> = Box::default();
    one_vec.push_back(555);
    one_vec.push_front(111);
}

{
    // Now, more than a page.
    let mut big: Box<[u64; 513]> = Box::new([0x8BADF00D; 513]);
}
```

## Setup

In order to get started with this project you'll need the following:

- Rust (currently on nightly branch)
- QEMU compiled for riscv
- `riscv-gnu-toolchain` (don't forget to add to PATH)
- `rustup target add riscv64gc-unkown-none-elf`
- `rustup component add rust-src`
    - `rustup` should be working with the nightly toolchain as an override for this repo (should be automatic)

If you have [Nix](https://nixos.org/download.html) installed, you should be able
to get all of these by running `nix develop` (can be automatically loaded when
you enter the directory if you have direnv).

## Usage

The build process for the reedos binary is entirely handled by cargo. Simply `cargo build` should suffice. For running or flashing reedos, see (saneboot)(https://github.com/tccq/saneboot)

You can exit QEMU by pressing <kbd>Ctrl</kbd> + <kbd>a</kbd>, then <kbd>x</kbd>.

- <kbd>Ctrl</kbd> + <kbd>a</kbd>, <kbd>c</kbd> gives a console, but you will
  find `gdb` much more helpful.

### Debug tools

You may find the following debug tools (that you have mostly already installed) helpful:

- `riscv64-unknown-elf-{objdump, gdb}` ← Recommend viewing docs material on
  becoming a GDB power user.
- In QEMU with `-nographic`, use <kbd>Ctrl</kbd> + <kbd>a</kbd>, then
  <kbd>c</kbd> to get to the console, then run `help` to see available commands.

### Docs

- Use `make docs` or `cargo doc --open` to build and open the crate
  documentation in a browser.
- Make sure to read the documentation in `docs/` too!

### References

- [ISA Manual](https://riscv.org/technical/specifications/) (Go to "Volume 2, Privileged Specification")
- [Rustonomicon](https://doc.rust-lang.org/nomicon/)
- [Embedonomicon](https://docs.rust-embedded.org/embedonomicon/index.html)
- [Interrupt Cookbook](https://www.starfivetech.com/uploads/sifive-interrupt-cookbook-v1p2.pdf)
- [MIT's XV6-RISCV](https://github.com/mit-pdos/xv6-riscv)
- [Marz's OSDEV Blog OS](https://osblog.stephenmarz.com/index.html)
- [Phil-Opp's Blog OS](https://os.phil-opp.com/)
