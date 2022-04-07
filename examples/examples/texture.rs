#![cfg(target_os = "android")]

#[ndk_glue::main(backtrace = "on", logger(level = "info", tag = "ash.texture"))]
fn main() {
    examples::impls::texture::main()
}
