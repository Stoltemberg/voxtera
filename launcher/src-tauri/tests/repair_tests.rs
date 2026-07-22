use std::fs;

use launcher_core::{ArchiveMetadata, Channel, ManagedFile, Manifest, plan_repair};
use semver::Version;
use sha2::{Digest, Sha256};

fn sha256(bytes: &[u8]) -> String { hex::encode(Sha256::digest(bytes)) }

fn manifest(files: &[(&str, &[u8])]) -> Manifest {
    let mut files = files
        .iter()
        .map(|(path, contents)| ManagedFile {
            path: (*path).to_owned(),
            size: contents.len() as u64,
            sha256: sha256(contents),
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Manifest {
        schema_version: 1,
        version: Version::new(1, 0, 0),
        channel: Channel::Preview,
        archive: ArchiveMetadata {
            name: "Voxtera-windows-x64.zip".to_owned(),
            size: 1,
            sha256: "a".repeat(64),
        },
        executable: "Voxtera.exe".to_owned(),
        files,
        preserved_paths: vec![
            "userdata/".to_owned(),
            "screenshots/".to_owned(),
            "settings/".to_owned(),
        ],
        minimum_launcher_version: Version::new(0, 1, 0),
    }
}

fn write(path: &std::path::Path, contents: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

#[test]
fn repair_lists_missing_and_changed_managed_files_but_skips_valid_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("game");
    let manifest = manifest(&[
        ("Voxtera.exe", b"game"),
        ("assets/changed.bin", b"expected"),
        ("assets/missing.bin", b"missing"),
        ("assets/valid.bin", b"valid"),
    ]);
    write(&root.join("Voxtera.exe"), b"game");
    write(&root.join("assets/changed.bin"), b"corrupt");
    write(&root.join("assets/valid.bin"), b"valid");

    let plan = plan_repair(&root, &manifest, 2).unwrap();

    assert_eq!(plan.checked_files, 4);
    assert_eq!(plan.invalid_files, vec![
        "assets/changed.bin",
        "assets/missing.bin"
    ]);
    assert_eq!(plan.invalid_bytes, 15);
    assert!(!plan.is_clean());
    assert_eq!(plan.confirm().invalid_files.len(), 2);
}

#[test]
fn repair_ignores_preserved_and_unmanaged_files_and_never_proposes_deletion() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("game");
    let manifest = manifest(&[("Voxtera.exe", b"game")]);
    write(&root.join("Voxtera.exe"), b"game");
    write(&root.join("userdata/save.ron"), b"save");
    write(&root.join("screenshots/shot.png"), b"shot");
    write(&root.join("settings/video.ron"), b"settings");
    write(&root.join("unmanaged/private.txt"), b"private");

    let plan = plan_repair(&root, &manifest, 4).unwrap();

    assert!(plan.is_clean());
    assert!(plan.invalid_files.is_empty());
    assert!(plan.deletions.is_empty());
}

#[test]
fn repair_rejects_zero_worker_concurrency_and_invalid_manifest() {
    let temp = tempfile::tempdir().unwrap();
    let mut invalid = manifest(&[("Voxtera.exe", b"game")]);
    invalid.files[0].path = "../escape".to_owned();

    assert!(plan_repair(temp.path(), &invalid, 2).is_err());
    let valid = manifest(&[("Voxtera.exe", b"game")]);
    assert!(plan_repair(temp.path(), &valid, 0).is_err());
}
