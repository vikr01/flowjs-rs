//! Copy flow_parser.js to OUT_DIR.
//!
//! Lookup order:
//! 1. node_modules/flow-parser/flow_parser.js  (local dev after `bun install`)
//! 2. vendor/flow_parser.js                    (committed fallback; used by published crate)

use std::path::{Path, PathBuf};
use std::process::Command;

const JS_NPM: &str = "node_modules/flow-parser/flow_parser.js";
const JS_VENDOR: &str = "vendor/flow_parser.js";
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

    assert!(
        status.success(),
        "`bun install` failed with status {status}"
    );
}

fn main() {
    println!("cargo::rerun-if-changed=package.json");
    println!("cargo::rerun-if-changed={JS_VENDOR}");

    let crate_dir =
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
    let dst = out_dir.join(OUT_FILENAME);

    if dst.exists() {
        return;
    }

    // Prefer node_modules (dev); fall back to vendored copy (published crate).
    let src = {
        let npm = crate_dir.join(JS_NPM);
        if npm.exists() {
            npm
        } else {
            let vendor = crate_dir.join(JS_VENDOR);
            if !vendor.exists() {
                install_deps(&crate_dir);
                // After install, use npm path.
                let npm2 = crate_dir.join(JS_NPM);
                assert!(
                    npm2.exists(),
                    "flow_parser.js not found at {} after install",
                    npm2.display()
                );
                npm2
            } else {
                vendor
            }
        }
    };

    std::fs::copy(&src, &dst).unwrap_or_else(|e| {
        panic!("failed to copy {} → {}: {e}", src.display(), dst.display());
    });
}
