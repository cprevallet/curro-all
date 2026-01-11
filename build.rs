// build.rs
use std;
use toml;

fn main() {
    // 1. Tell Cargo to rerun this script if Cargo.toml changes
    //    This ensures the version is updated on subsequent builds.
    println!("cargo:rerun-if-changed=Cargo.toml");

    // 2. Read the Cargo.toml content
    let cargo_toml = std::fs::read_to_string("Cargo.toml").expect("Failed to read Cargo.toml");

    // 3. Parse the TOML and extract the version
    let value: toml::Value = toml::from_str(&cargo_toml).expect("Failed to parse Cargo.toml");
    let package = value
        .get("package")
        .and_then(|v| v.as_table())
        .expect("Cargo.toml must have a [package] section");

    let version_str = package
        .get("version")
        .and_then(|v| v.as_str())
        .expect("Package must have a 'version' key");

    // 4. Set the environment variable `CRATE_VERSION` for the main crate
    println!("cargo:rustc-env=CRATE_VERSION={}", version_str);
}
