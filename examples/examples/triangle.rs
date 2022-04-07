#![cfg(target_os = "android")]

#[ndk_glue::main(backtrace = "on", logger(level = "info", tag = "ash.triangle"))]
fn main() {
    examples::impls::triangle::main()
}
