
// The main point here is to specify a custom linkerscript
fn main() {
    println!("cargo:rustc-link-arg=-T./kernel/kernel.ld")
}
