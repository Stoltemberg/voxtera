use std::fs;

use launcher_core::{
    ArchiveMetadata, Channel, ManagedFile, Manifest, build_manifest, manifest_json,
};
use semver::Version;

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

fn valid_manifest() -> Manifest {
    Manifest {
        schema_version: 1,
        version: Version::parse("0.2.3").unwrap(),
        channel: Channel::Preview,
        archive: ArchiveMetadata {
            name: "Voxtera-windows-x64.zip".to_owned(),
            size: 42,
            sha256: HASH.to_owned(),
        },
        executable: "Voxtera.exe".to_owned(),
        files: vec![
            ManagedFile {
                path: "Voxtera.exe".to_owned(),
                size: 12,
                sha256: HASH.to_owned(),
            },
            ManagedFile {
                path: "assets/voxel/a.vox".to_owned(),
                size: 30,
                sha256: HASH.to_owned(),
            },
        ],
        preserved_paths: vec!["userdata/".to_owned(), "screenshots/".to_owned()],
        minimum_launcher_version: Version::parse("0.3.0-preview.1").unwrap(),
    }
}

#[test]
fn cli_requires_an_explicit_minimum_launcher_version() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_voxtera-manifest"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(
        help.contains("--minimum-launcher-version <VERSION>"),
        "{help}"
    );
}

#[test]
fn generator_preserves_the_explicit_minimum_launcher_version() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Voxtera.exe"), b"game").unwrap();
    let archive_root = tempfile::tempdir().unwrap();
    let archive = archive_root.path().join("Voxtera-windows-x64.zip");
    fs::write(&archive, b"archive").unwrap();
    let minimum = Version::parse("0.2.1").unwrap();

    let manifest = build_manifest(
        root.path(),
        &archive,
        Version::parse("0.2.3").unwrap(),
        minimum.clone(),
    )
    .unwrap();

    assert_eq!(manifest.minimum_launcher_version, minimum);
}

#[test]
fn accepts_schema_one_semver_and_required_preserved_rules() {
    valid_manifest().validate().unwrap();
}

#[test]
fn rejects_unsupported_schema() {
    let mut manifest = valid_manifest();
    manifest.schema_version = 2;
    assert_eq!(manifest.validate().unwrap_err().code(), "manifest_contract");
}

#[test]
fn rejects_noncanonical_versions_when_deserializing() {
    let json = manifest_json(&valid_manifest()).unwrap();
    let json = json.replace("\"0.2.3\"", "\"v0.2.3\"");
    assert!(serde_json::from_str::<Manifest>(&json).is_err());
}

#[test]
fn rejects_hashes_that_are_not_lowercase_64_character_hex() {
    for invalid in [
        "ABCDEF6789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "0123",
        "g123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    ] {
        let mut manifest = valid_manifest();
        manifest.files[0].sha256 = invalid.to_owned();
        assert_eq!(manifest.validate().unwrap_err().code(), "manifest_contract");
    }
}

#[test]
fn rejects_any_archive_name_other_than_the_release_contract_name() {
    let mut manifest = valid_manifest();
    manifest.archive.name = "voxtera.zip".to_owned();
    assert_eq!(manifest.validate().unwrap_err().code(), "manifest_contract");
}

#[test]
fn rejects_non_normalized_relative_and_duplicate_managed_paths() {
    for invalid in [
        "assets\\a.vox",
        "assets//a.vox",
        "./assets/a.vox",
        "assets/../Voxtera.exe",
        "../Voxtera.exe",
        "/Voxtera.exe",
        r"C:\Games\Voxtera.exe",
        r"\\server\share\Voxtera.exe",
    ] {
        let mut manifest = valid_manifest();
        manifest.files[1].path = invalid.to_owned();
        assert_eq!(
            manifest.validate().unwrap_err().code(),
            "manifest_contract",
            "{invalid}"
        );
    }

    let mut manifest = valid_manifest();
    manifest.files[1].path = "voxtera.EXE".to_owned();
    assert_eq!(manifest.validate().unwrap_err().code(), "manifest_contract");
}

#[test]
fn rejects_missing_or_unsafe_preserved_rules() {
    for rules in [
        vec!["userdata/".to_owned()],
        vec!["userdata/".to_owned(), "../screenshots/".to_owned()],
        vec!["userdata/".to_owned(), r"C:\screenshots\".to_owned()],
    ] {
        let mut manifest = valid_manifest();
        manifest.preserved_paths = rules;
        assert_eq!(manifest.validate().unwrap_err().code(), "manifest_contract");
    }
}

#[test]
fn rejects_managed_files_inside_preserved_paths() {
    let mut manifest = valid_manifest();
    manifest.files[1].path = "userdata/save.ron".to_owned();
    assert_eq!(manifest.validate().unwrap_err().code(), "manifest_contract");
}

#[test]
fn rejects_a_minimum_launcher_version_newer_than_the_generating_launcher() {
    let mut manifest = valid_manifest();
    manifest.minimum_launcher_version = Version::parse("99.0.0").unwrap();
    assert_eq!(manifest.validate().unwrap_err().code(), "manifest_contract");
}

#[test]
fn identical_trees_produce_byte_identical_golden_manifests() {
    let left = tempfile::tempdir().unwrap();
    let right = tempfile::tempdir().unwrap();
    for (root, paths) in [
        (left.path(), ["assets/z.vox", "Voxtera.exe", "assets/a.vox"]),
        (right.path(), [
            "assets/a.vox",
            "Voxtera.exe",
            "assets/z.vox",
        ]),
    ] {
        for path in paths {
            let target = root.join(path);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, path.as_bytes()).unwrap();
        }
        for excluded in [
            "cache/chunk.bin",
            "logs/latest.log",
            "launcher/state.json",
            "userdata/save.ron",
            "screenshots/a.png",
            "download.part",
            "update.tmp",
        ] {
            let target = root.join(excluded);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, b"excluded").unwrap();
        }
    }
    let archives = tempfile::tempdir().unwrap();
    let archive = archives.path().join("Voxtera-windows-x64.zip");
    fs::write(&archive, b"deterministic archive bytes").unwrap();
    let version = Version::parse("0.2.3").unwrap();

    let minimum = Version::parse("0.3.0-preview.1").unwrap();
    let left_json = manifest_json(
        &build_manifest(left.path(), &archive, version.clone(), minimum.clone()).unwrap(),
    )
    .unwrap();
    let right_json =
        manifest_json(&build_manifest(right.path(), &archive, version, minimum).unwrap()).unwrap();

    assert_eq!(left_json, right_json);
    assert_eq!(left_json, include_str!("fixtures/manifest.golden.json"));
    assert!(left_json.ends_with('\n'));
    assert!(!left_json.contains("excluded"));
    assert!(left_json.find("assets/a.vox").unwrap() < left_json.find("assets/z.vox").unwrap());
}

#[test]
fn generator_rejects_the_wrong_archive_name() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Voxtera.exe"), b"game").unwrap();
    let archive = root.path().join("game.zip");
    fs::write(&archive, b"archive").unwrap();

    assert_eq!(
        build_manifest(
            root.path(),
            &archive,
            Version::parse("0.2.3").unwrap(),
            Version::parse("0.3.0-preview.1").unwrap(),
        )
        .unwrap_err()
        .code(),
        "manifest_contract"
    );
}

#[cfg(windows)]
#[test]
fn generator_refuses_symlinks() {
    use std::{
        os::windows::fs::symlink_dir,
        process::{Command, Stdio},
    };

    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("outside.bin"), b"outside").unwrap();
    let link = root.path().join("linked");
    if let Err(error) = symlink_dir(outside.path(), &link) {
        if error.raw_os_error() == Some(1314) {
            let status = Command::new("cmd")
                .args(["/c", "mklink", "/J"])
                .arg(&link)
                .arg(outside.path())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();
            assert!(status.success(), "could not create a junction fallback");
        } else {
            panic!("could not create symlink: {error}");
        }
    }
    fs::write(root.path().join("Voxtera.exe"), b"game").unwrap();
    let archive_root = tempfile::tempdir().unwrap();
    let archive = archive_root.path().join("Voxtera-windows-x64.zip");
    fs::write(&archive, b"archive").unwrap();

    assert_eq!(
        build_manifest(
            root.path(),
            &archive,
            Version::parse("0.2.3").unwrap(),
            Version::parse("0.3.0-preview.1").unwrap(),
        )
        .unwrap_err()
        .code(),
        "manifest_contract"
    );
}
