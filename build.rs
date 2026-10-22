//! Bake the version and the source commit into the binary.

fn main() {
    let version = std::env::var("TUTORIAL_STEPPER_VERSION").unwrap_or_else(|_| {
        std::env::var("CARGO_PKG_VERSION").expect("cargo sets the package version")
    });
    let commit = std::env::var("TUTORIAL_STEPPER_COMMIT").unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=TUTORIAL_STEPPER_VERSION_RESOLVED={version}");
    println!("cargo:rustc-env=TUTORIAL_STEPPER_COMMIT_RESOLVED={commit}");
    println!("cargo:rerun-if-env-changed=TUTORIAL_STEPPER_VERSION");
    println!("cargo:rerun-if-env-changed=TUTORIAL_STEPPER_COMMIT");
}
