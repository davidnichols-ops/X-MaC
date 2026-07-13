fn main() {
    // Set the minimum macOS deployment target. This must be kept in sync
    // with the LSMinimumSystemVersion in gui/build_app.sh Info.plist and
    // the "Requirements" table in README.md.
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=13.0");
    }

    // Embed build metadata for --version --verbose
    let git_hash = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let build_date = chrono::Local::now().format("%Y-%m-%d").to_string();

    println!("cargo:rustc-env=XMAC_GIT_HASH={}", git_hash);
    println!("cargo:rustc-env=XMAC_BUILD_DATE={}", build_date);

    println!("cargo:rerun-if-changed=build.rs");
}
