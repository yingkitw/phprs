//! Ensures all registered `[[example]]` targets compile (API drift guard).

use std::ffi::OsString;
use std::process::Command;

fn cargo_executable() -> OsString {
    std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"))
}

#[test]
fn rust_examples_compile() {
    let ok = Command::new(cargo_executable())
        .args(["build", "--examples", "-q"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("spawn cargo build --examples");
    assert!(ok.success(), "cargo build --examples failed");
}
