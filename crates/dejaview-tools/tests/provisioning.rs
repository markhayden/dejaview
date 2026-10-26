//! Task 1.14: a clean-directory install from the built bundle yields a working executable whose
//! version matches the lock; the harness calls `echo_region` through it offline; a tampered digest
//! fails. Needs `uv` and the bundle from `scripts/build-bundle.sh`, so these run with `--ignored`.
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::time::Duration;

use dejaview_core::tool::Status;
use dejaview_tools::mcp::{Pack, Timeouts};
use dejaview_tools::provision::{Bundle, Installed};
use serde_json::json;

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bundle_dir() -> PathBuf {
    let dir = repo().join("tools/python/dist/bundle-0.0.1");
    assert!(
        dir.join("bundle.toml").exists(),
        "no bundle at {}: run scripts/build-bundle.sh first",
        dir.display()
    );
    dir
}

fn tempdir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "dejaview-provisioning-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
#[ignore = "needs uv and a built bundle (scripts/build-bundle.sh)"]
fn clean_install_serves_echo_region_offline_with_versions_matching_the_lock() {
    let bundle = Bundle::load(&bundle_dir()).unwrap();
    bundle.verify().unwrap();
    let data_home = tempdir("install");
    let installed = bundle.install(&data_home).unwrap();
    assert!(installed.executable.is_file());
    assert_eq!(installed.header.version, "0.0.1");

    // The installed executable reports the bundle's version, and the lock pins the same one.
    let out = std::process::Command::new(&installed.executable)
        .arg("--version")
        .env("UV_OFFLINE", "1")
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        installed.header.version
    );
    let lock = std::fs::read_to_string(installed.dir.join("uv.lock")).unwrap();
    assert!(
        lock.contains("name = \"dejaview-tools\"")
            && lock.contains(&format!("version = \"{}\"", installed.header.version)),
        "the lock pins the installed version"
    );
    assert_eq!(
        Installed::locate(&data_home, "dejaview-tools", "0.0.1")
            .unwrap()
            .dir,
        installed.dir
    );

    // The harness launches it offline and calls the tool through MCP.
    let timeouts = Timeouts {
        startup: Duration::from_secs(60),
        call: Duration::from_secs(30),
        shutdown_grace: Duration::from_secs(2),
    };
    let pack = Pack::connect(
        "python",
        installed.spawn(vec![]),
        timeouts,
        std::env::vars(),
    )
    .unwrap();
    assert_eq!(pack.manifests()[0].name, "echo_region");
    let envelope = pack
        .call(
            "run-1",
            None,
            "echo_region",
            json!({
                "image": "candidate",
                "bbox": [0.1, 0.2, 0.3, 0.4],
                "_dejaview": { "run_id": "run-1", "call_id": "call:1", "images": { "candidate": { "media_type": "image/png", "width": 1, "height": 1, "projection": "flat" } }, "frames": {}, "hints": {} },
            }),
        )
        .unwrap();
    assert_eq!(envelope.status, Status::Ok);
    assert_eq!(envelope.regions.len(), 1);
    assert_eq!(
        envelope.provenance.lock_digest.as_deref(),
        Some(installed.lock_digest.as_str())
    );
}

#[test]
#[ignore = "needs a built bundle (scripts/build-bundle.sh)"]
fn a_tampered_bundle_is_refused_before_anything_is_installed() {
    let copy = tempdir("tampered").join("bundle");
    std::fs::create_dir_all(&copy).unwrap();
    for entry in std::fs::read_dir(bundle_dir()).unwrap() {
        let entry = entry.unwrap();
        std::fs::copy(entry.path(), copy.join(entry.file_name())).unwrap();
    }
    let mut model = std::fs::read_to_string(copy.join("MODEL.md")).unwrap();
    model.push_str("\ntampered\n");
    std::fs::write(copy.join("MODEL.md"), model).unwrap();
    let data_home = tempdir("tampered-home");
    let err = Bundle::load(&copy)
        .unwrap()
        .install(&data_home)
        .unwrap_err();
    assert_eq!(err.class(), "resolve");
    assert!(err.to_string().contains("digest mismatch"), "{err}");
    assert!(
        !data_home.join("dejaview").exists(),
        "nothing was installed"
    );
}
