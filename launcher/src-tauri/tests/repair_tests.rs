use std::fs;

use launcher_core::{
    ArchiveMetadata, Channel, InstallManager, ManagedFile, Manifest, PromotionRequest, plan_repair,
    prepare_repair_staging,
};
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

#[test]
fn repaired_staging_preserves_real_unmanaged_files_through_atomic_promotion() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    let manifest = manifest(&[
        ("Voxtera.exe", b"new-game"),
        ("assets/managed.bin", b"new-managed"),
    ]);
    write(&live.join("Voxtera.exe"), b"old-game");
    write(&live.join("assets/managed.bin"), b"corrupt");
    write(&live.join("mods/player-created.mod"), b"keep-unmanaged");
    write(&live.join("userdata/save.ron"), b"keep-save");
    let staging = manager.create_staging().unwrap();
    write(&staging.join("Voxtera.exe"), b"new-game");
    write(&staging.join("assets/managed.bin"), b"new-managed");

    let confirmed = plan_repair(&live, &manifest, 2).unwrap().confirm();
    let copied = prepare_repair_staging(&live, &staging, &manifest, &confirmed).unwrap();
    assert_eq!(copied, 1);
    manager
        .promote(PromotionRequest {
            staging_dir: staging,
            failure_point: None,
        })
        .unwrap();

    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"new-game");
    assert_eq!(
        fs::read(live.join("assets/managed.bin")).unwrap(),
        b"new-managed"
    );
    assert_eq!(
        fs::read(live.join("mods/player-created.mod")).unwrap(),
        b"keep-unmanaged"
    );
    assert_eq!(
        fs::read(live.join("userdata/save.ron")).unwrap(),
        b"keep-save"
    );
}

#[cfg(windows)]
#[test]
fn repair_planner_marks_managed_file_under_junction_invalid() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target");
    let live = temp.path().join("game");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("Voxtera.exe"), b"game").unwrap();
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&live)
        .arg(&target)
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());

    let plan = plan_repair(&live, &manifest(&[("Voxtera.exe", b"game")]), 1).unwrap();

    assert_eq!(plan.invalid_files, vec!["Voxtera.exe"]);
}

#[cfg(windows)]
#[test]
fn repair_staging_junction_is_rejected_before_copying_unmanaged_outside() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let outside = temp.path().join("outside");
    let manager = InstallManager::new(live.clone());
    let manifest = manifest(&[("Voxtera.exe", b"new-game")]);
    write(&live.join("Voxtera.exe"), b"old-game");
    write(&live.join("mods/player.mod"), b"must-not-escape");
    let staging = manager.create_staging().unwrap();
    write(&staging.join("Voxtera.exe"), b"new-game");
    fs::create_dir(&outside).unwrap();
    write(&outside.join("sentinel.txt"), b"unchanged");
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(staging.join("mods"))
        .arg(&outside)
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());
    let confirmed = plan_repair(&live, &manifest, 1).unwrap().confirm();

    assert!(prepare_repair_staging(&live, &staging, &manifest, &confirmed).is_err());
    assert_eq!(
        fs::read(outside.join("sentinel.txt")).unwrap(),
        b"unchanged"
    );
    assert!(!outside.join("player.mod").exists());
}
