fn main() {
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=12.0");
    println!("cargo:rerun-if-changed=build.rs");
}
