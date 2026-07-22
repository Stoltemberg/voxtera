use std::{fs, io::Write, path::Path};

use launcher_core::{
    ArchiveError, ArchiveLimits, ArchiveMetadata, Channel, ManagedFile, Manifest,
    extract_to_staging, verify_file,
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

fn eocd_offset(bytes: &[u8]) -> usize {
    bytes
        .windows(4)
        .rposition(|window| window == b"PK\x05\x06")
        .unwrap()
}

fn central_header_offset(bytes: &[u8]) -> usize {
    bytes
        .windows(4)
        .position(|window| window == b"PK\x01\x02")
        .unwrap()
}

fn rewrite_archive(archive: &Path, mutate: impl FnOnce(&mut [u8], usize)) -> Manifest {
    let mut bytes = fs::read(archive).unwrap();
    let eocd = eocd_offset(&bytes);
    mutate(&mut bytes, eocd);
    fs::write(archive, &bytes).unwrap();
    manifest_for(archive, &[("Voxtera.exe", b"game")])
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
    for hostile in [
        "../escape.txt",
        "/absolute.txt",
        r"C:\absolute.txt",
        "COM¹.txt",
        "COM².txt",
        "COM³.txt",
        "LPT¹.txt",
        "LPT².txt",
        "LPT³.txt",
    ] {
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
fn zip_preflight_rejects_zip64_entry_count_before_parser_allocation() {
    let (_archive_temp, archive) = zip_fixture(&[("Voxtera.exe", b"game", None)]);
    let manifest = rewrite_archive(&archive, |bytes, eocd| {
        bytes[eocd + 8..eocd + 12].copy_from_slice(&[0xff; 4]);
    });
    let root = tempfile::tempdir().unwrap();
    let staging = root.path().join("game.staging-test");

    let error =
        extract_to_staging(&archive, &staging, &manifest, ArchiveLimits::default()).unwrap_err();

    assert!(matches!(error, ArchiveError::Preflight(_)));
    assert!(!staging.exists());
}

#[test]
fn zip_preflight_rejects_large_non_zip64_entry_count_before_parser_allocation() {
    let (_archive_temp, archive) = zip_fixture(&[("Voxtera.exe", b"game", None)]);
    let manifest = rewrite_archive(&archive, |bytes, eocd| {
        let count = 20_000_u16.to_le_bytes();
        bytes[eocd + 8..eocd + 10].copy_from_slice(&count);
        bytes[eocd + 10..eocd + 12].copy_from_slice(&count);
    });
    let root = tempfile::tempdir().unwrap();
    let staging = root.path().join("game.staging-test");

    let error =
        extract_to_staging(&archive, &staging, &manifest, ArchiveLimits::default()).unwrap_err();

    assert!(matches!(
        error,
        ArchiveError::Preflight(message) if message.contains("entry ceiling")
    ));
    assert!(!staging.exists());
}

#[test]
fn zip_preflight_rejects_oversized_central_directory_before_parser_allocation() {
    let (_archive_temp, archive) = zip_fixture(&[("Voxtera.exe", b"game", None)]);
    let manifest = rewrite_archive(&archive, |bytes, eocd| {
        bytes[eocd + 12..eocd + 16].copy_from_slice(&(9_u32 * 1024 * 1024).to_le_bytes());
    });
    let root = tempfile::tempdir().unwrap();
    let staging = root.path().join("game.staging-test");

    let error =
        extract_to_staging(&archive, &staging, &manifest, ArchiveLimits::default()).unwrap_err();

    assert!(matches!(error, ArchiveError::Preflight(_)));
    assert!(!staging.exists());
}

#[test]
fn extraction_rejects_unix_mode_with_unknown_zero_type_bits() {
    let (_archive_temp, archive) = zip_fixture(&[("Voxtera.exe", b"game", None)]);
    let manifest = rewrite_archive(&archive, |bytes, _| {
        let central = central_header_offset(bytes);
        bytes[central + 5] = 3;
        bytes[central + 38..central + 42].copy_from_slice(&((0o777_u32) << 16).to_le_bytes());
    });
    let root = tempfile::tempdir().unwrap();
    let staging = root.path().join("game.staging-test");

    let error =
        extract_to_staging(&archive, &staging, &manifest, ArchiveLimits::default()).unwrap_err();

    assert!(matches!(error, ArchiveError::Unsafe(_)));
    assert!(!staging.exists());
}

#[test]
fn extraction_accepts_windows_entry_without_unix_mode() {
    let (_archive_temp, archive) = zip_fixture(&[("Voxtera.exe", b"game", None)]);
    let manifest = rewrite_archive(&archive, |bytes, _| {
        let central = central_header_offset(bytes);
        bytes[central + 5] = 0;
        bytes[central + 38..central + 42].copy_from_slice(&0_u32.to_le_bytes());
    });
    let root = tempfile::tempdir().unwrap();
    let staging = root.path().join("game.staging-test");

    extract_to_staging(&archive, &staging, &manifest, ArchiveLimits::default()).unwrap();

    assert_eq!(fs::read(staging.join("Voxtera.exe")).unwrap(), b"game");
}

#[cfg(windows)]
#[test]
fn verify_file_does_not_follow_a_parent_junction() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("target");
    let junction = temp.path().join("junction");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("managed.bin"), b"managed").unwrap();
    let status = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&junction)
        .arg(&target)
        .stdout(std::process::Stdio::null())
        .status()
        .unwrap();
    assert!(status.success());

    assert!(verify_file(&junction.join("managed.bin"), 7, &sha256(b"managed")).is_err());
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
