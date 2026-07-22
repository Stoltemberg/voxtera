use std::{fs, io::Write, path::Path};

use launcher_core::{
    ArchiveLimits, ArchiveMetadata, Channel, ManagedFile, Manifest, extract_to_staging, verify_file,
};
use semver::Version;
use sha2::{Digest, Sha256};
use tempfile::TempDir;
use zip::write::SimpleFileOptions;

fn sha256(bytes: &[u8]) -> String { hex::encode(Sha256::digest(bytes)) }

fn zip_fixture(entries: &[(&str, &[u8], Option<u32>)]) -> (TempDir, std::path::PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("Voxtera-windows-x64.zip");
    let file = fs::File::create(&archive_path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    for (name, contents, unix_mode) in entries {
        let mut options = SimpleFileOptions::default();
        if let Some(mode) = unix_mode {
            options = options.unix_permissions(*mode);
        }
        writer.start_file(*name, options).unwrap();
        writer.write_all(contents).unwrap();
    }
    writer.finish().unwrap();
    (temp, archive_path)
}

fn manifest_for(archive: &Path, files: &[(&str, &[u8])]) -> Manifest {
    let archive_bytes = fs::read(archive).unwrap();
    let mut managed = files
        .iter()
        .map(|(path, contents)| ManagedFile {
            path: (*path).to_owned(),
            size: contents.len() as u64,
            sha256: sha256(contents),
        })
        .collect::<Vec<_>>();
    managed.sort_by(|left, right| left.path.cmp(&right.path));
    Manifest {
        schema_version: 1,
        version: Version::new(1, 0, 0),
        channel: Channel::Preview,
        archive: ArchiveMetadata {
            name: "Voxtera-windows-x64.zip".to_owned(),
            size: archive_bytes.len() as u64,
            sha256: sha256(&archive_bytes),
        },
        executable: "Voxtera.exe".to_owned(),
        files: managed,
        preserved_paths: vec![
            "userdata/".to_owned(),
            "screenshots/".to_owned(),
            "settings/".to_owned(),
        ],
        minimum_launcher_version: Version::new(0, 1, 0),
    }
}

#[test]
fn verify_file_rejects_wrong_size_and_wrong_sha256() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("payload.bin");
    fs::write(&file, b"payload").unwrap();

    assert!(verify_file(&file, 8, &sha256(b"payload")).is_err());
    assert!(verify_file(&file, 7, &sha256(b"different")).is_err());
    assert_eq!(verify_file(&file, 7, &sha256(b"payload")).unwrap().size, 7);
}

#[test]
fn extraction_rejects_absolute_traversal_and_drive_paths_without_escape() {
    for hostile in ["../escape.txt", "/absolute.txt", r"C:\absolute.txt"] {
        let (_archive_temp, archive) =
            zip_fixture(&[("Voxtera.exe", b"game", None), (hostile, b"attack", None)]);
        let manifest = manifest_for(&archive, &[("Voxtera.exe", b"game")]);
        let root = tempfile::tempdir().unwrap();
        let staging = root.path().join("game.staging-test");

        assert!(
            extract_to_staging(&archive, &staging, &manifest, ArchiveLimits::default()).is_err(),
            "hostile entry {hostile:?} was accepted"
        );
        assert!(!root.path().join("escape.txt").exists());
        assert!(!root.path().join("absolute.txt").exists());
        assert!(!staging.join("C").exists());
    }
}

#[test]
fn extraction_rejects_symlinks_and_duplicate_windows_paths() {
    let cases: Vec<Vec<(&str, &[u8], Option<u32>)>> = vec![
        vec![
            ("Voxtera.exe", b"game", None),
            ("link", b"../outside", Some(0o120777)),
        ],
        vec![
            ("Voxtera.exe", b"game", None),
            ("Data/file.txt", b"one", None),
            (r"data\FILE.txt", b"two", None),
        ],
    ];
    for entries in cases {
        let (_archive_temp, archive) = zip_fixture(&entries);
        let manifest = manifest_for(&archive, &[("Voxtera.exe", b"game")]);
        let root = tempfile::tempdir().unwrap();
        let staging = root.path().join("game.staging-test");

        assert!(
            extract_to_staging(&archive, &staging, &manifest, ArchiveLimits::default()).is_err()
        );
        assert!(!root.path().join("outside").exists());
    }
}

#[test]
fn extraction_enforces_cumulative_uncompressed_ceiling() {
    let (_archive_temp, archive) = zip_fixture(&[
        ("Voxtera.exe", b"game", None),
        ("data.bin", &[7_u8; 32], None),
    ]);
    let manifest = manifest_for(&archive, &[
        ("Voxtera.exe", b"game"),
        ("data.bin", &[7_u8; 32]),
    ]);
    let root = tempfile::tempdir().unwrap();
    let staging = root.path().join("game.staging-test");

    let result = extract_to_staging(&archive, &staging, &manifest, ArchiveLimits {
        max_entries: 10,
        max_uncompressed_bytes: 16,
    });

    assert!(result.is_err());
    assert!(!staging.exists());
}

#[test]
fn archive_integrity_is_checked_before_staging_is_created() {
    let (_archive_temp, archive) = zip_fixture(&[("Voxtera.exe", b"game", None)]);
    let root = tempfile::tempdir().unwrap();
    let staging = root.path().join("game.staging-test");
    let mut wrong_size = manifest_for(&archive, &[("Voxtera.exe", b"game")]);
    wrong_size.archive.size += 1;
    assert!(extract_to_staging(&archive, &staging, &wrong_size, ArchiveLimits::default()).is_err());
    assert!(!staging.exists());

    let mut wrong_hash = manifest_for(&archive, &[("Voxtera.exe", b"game")]);
    wrong_hash.archive.sha256 = "0".repeat(64);
    assert!(extract_to_staging(&archive, &staging, &wrong_hash, ArchiveLimits::default()).is_err());
    assert!(!staging.exists());
}
