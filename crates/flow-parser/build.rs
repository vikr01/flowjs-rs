//! Install flow-parser from npm via bun and copy the JS file to OUT_DIR.

use std::path::{Path, PathBuf};
use std::process::Command;

const JS_FILE: &str = "node_modules/flow-parser/flow_parser.js";
const OUT_FILENAME: &str = "flow_parser.js";

fn install_deps(crate_dir: &Path) {
    let status = Command::new("bun")
        .arg("install")
        .current_dir(crate_dir)
        .status()
        .unwrap_or_else(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                panic!("bun not found on PATH — install it: https://bun.sh");
            }
            panic!("failed to run `bun install`: {e}");
        });

    assert!(status.success(), "`bun install` failed with status {status}");
}

fn main() {
    println!("cargo::rerun-if-changed=package.json");

    let crate_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"),
    );
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
    let dst = out_dir.join(OUT_FILENAME);

    if dst.exists() {
        return;
    }

    let src = crate_dir.join(JS_FILE);
    if !src.exists() {
        install_deps(&crate_dir);
    }

    assert!(
        src.exists(),
        "flow_parser.js not found at {} after install",
        src.display()
    );

    std::fs::copy(&src, &dst).unwrap_or_else(|e| {
        panic!(
            "failed to copy {} → {}: {e}",
            src.display(),
            dst.display()
        );
    });
}
