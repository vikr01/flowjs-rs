// Fetch `flow-parser` from the npm registry and extract `flow_parser.js`.
//
// Downloads the package tarball directly — no node, bun, or package manager
// required. The extracted file lands in `OUT_DIR` for `include_str!` at
// compile time.
//
// Skips the download when the file already exists in `OUT_DIR`.

use flate2::read::GzDecoder;
use std::io::Read;
use std::path::PathBuf;
use tar::Archive;

const NPM_PACKAGE: &str = "flow-parser";
const NPM_VERSION: &str = "0.266.1";
const ENTRY_PATH: &str = "package/flow_parser.js";
const OUT_FILENAME: &str = "flow_parser.js";

// ── Registry URL ────────────────────────────────────────────────────────

fn tarball_url() -> String {
    format!(
        "https://registry.npmjs.org/{NPM_PACKAGE}/-/{NPM_PACKAGE}-{NPM_VERSION}.tgz"
    )
}

// ── Download and extract ────────────────────────────────────────────────

fn fetch_flow_parser(dst: &PathBuf) {
    let url = tarball_url();

    let response = ureq::get(&url).call().unwrap_or_else(|e| {
        panic!("failed to fetch {url}: {e}");
    });

    let reader = response.into_body().into_reader();
    let gz = GzDecoder::new(reader);
    let mut archive = Archive::new(gz);

    for entry in archive.entries().expect("failed to read tarball entries") {
        let mut entry = entry.expect("failed to read tarball entry");
        let path = entry.path().expect("failed to read entry path");

        if path.to_str() == Some(ENTRY_PATH) {
            let mut contents = String::new();
            entry
                .read_to_string(&mut contents)
                .expect("failed to read flow_parser.js from tarball");
            std::fs::write(dst, contents).expect("failed to write flow_parser.js");
            return;
        }
    }

    panic!("{ENTRY_PATH} not found in {NPM_PACKAGE}@{NPM_VERSION} tarball");
}

// ── Entry point ─────────────────────────────────────────────────────────

fn main() {
    println!("cargo::rerun-if-changed=build.rs");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let dst = out_dir.join(OUT_FILENAME);

    if dst.exists() {
        return;
    }

    fetch_flow_parser(&dst);
}
