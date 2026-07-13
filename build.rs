fn main() {
    // Set the minimum macOS deployment target. This must be kept in sync
    // with the LSMinimumSystemVersion in gui/build_app.sh Info.plist and
    // the "Requirements" table in README.md.
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=13.0");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
