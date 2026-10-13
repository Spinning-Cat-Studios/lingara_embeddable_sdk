// rustc gives an ELF cdylib no DT_SONAME, so a consumer's linker records the
// path it was handed as DT_NEEDED: CMake hands a build-tree-relative one for
// vcpkg's `vcpkg_installed/…`, and the binary then fails to load anywhere
// but that working directory. A SONAME makes DT_NEEDED the bare file name,
// resolved through the consumer's RUNPATH (ADR 8.10.26q D6).
fn main() {
    let os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if os == "linux" || os == "android" {
        println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,liblingara_embed.so");
    }
}
