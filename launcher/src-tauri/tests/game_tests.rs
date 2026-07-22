use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use launcher_core::{
    ArchiveMetadata, Channel, GameLauncher, LauncherErrorCode, ManagedFile, ManagedProcess,
    Manifest, ProcessRunner,
};
use semver::Version;
use sha2::{Digest, Sha256};

#[derive(Default)]
struct FakeRunner {
    calls: Arc<Mutex<Vec<(PathBuf, PathBuf)>>>,
    fail_spawn: bool,
}

struct FakeProcess {
    running: bool,
}

impl ManagedProcess for FakeProcess {
    fn id(&self) -> u32 { 42 }

    fn try_wait(&mut self) -> io::Result<Option<i32>> { Ok((!self.running).then_some(0)) }
}

impl ProcessRunner for FakeRunner {
    fn spawn(
        &self,
        executable: &Path,
        working_directory: &Path,
    ) -> io::Result<Box<dyn ManagedProcess>> {
        self.calls
            .lock()
            .unwrap()
            .push((executable.to_path_buf(), working_directory.to_path_buf()));
        if self.fail_spawn {
            return Err(io::Error::new(io::ErrorKind::PermissionDenied, "blocked"));
        }
        Ok(Box::new(FakeProcess { running: true }))
    }
}

fn hash(bytes: &[u8]) -> String { hex::encode(Sha256::digest(bytes)) }

fn manifest_for(executable: &[u8]) -> Manifest {
    Manifest {
        schema_version: 1,
        version: Version::parse("0.3.0-preview.1").unwrap(),
        channel: Channel::Preview,
        archive: ArchiveMetadata {
            name: "Voxtera-windows-x64.zip".to_owned(),
            size: 1,
            sha256: "0".repeat(64),
        },
        executable: "Voxtera.exe".to_owned(),
        files: vec![ManagedFile {
            path: "Voxtera.exe".to_owned(),
            size: executable.len() as u64,
            sha256: hash(executable),
        }],
        preserved_paths: vec![
            "userdata/".to_owned(),
            "screenshots/".to_owned(),
            "settings/".to_owned(),
        ],
        minimum_launcher_version: Version::parse("0.1.0").unwrap(),
    }
}

#[test]
fn launches_only_the_verified_voxtera_executable_from_the_exact_install_directory() {
    let temp = tempfile::tempdir().unwrap();
    let install_dir = temp.path().join("game");
    fs::create_dir(&install_dir).unwrap();
    fs::write(install_dir.join("Voxtera.exe"), b"verified-game").unwrap();
    fs::write(install_dir.join("other.exe"), b"not-the-game").unwrap();
    let runner = FakeRunner::default();
    let calls = Arc::clone(&runner.calls);
    let launcher = GameLauncher::new(runner);

    let pid = launcher
        .launch(&install_dir, &manifest_for(b"verified-game"), || Ok(()))
        .unwrap();

    assert_eq!(pid, 42);
    assert_eq!(calls.lock().unwrap().as_slice(), &[(
        install_dir.join("Voxtera.exe"),
        install_dir
    )]);
}

#[test]
fn missing_or_replaced_executable_is_rejected_before_spawn() {
    for contents in [None, Some(b"replaced-game".as_slice())] {
        let temp = tempfile::tempdir().unwrap();
        let install_dir = temp.path().join("game");
        fs::create_dir(&install_dir).unwrap();
        if let Some(contents) = contents {
            fs::write(install_dir.join("Voxtera.exe"), contents).unwrap();
        }
        let runner = FakeRunner::default();
        let calls = Arc::clone(&runner.calls);
        let launcher = GameLauncher::new(runner);

        let error = launcher
            .launch(&install_dir, &manifest_for(b"verified-game"), || Ok(()))
            .unwrap_err();

        assert_eq!(error.code, LauncherErrorCode::Integrity);
        assert!(calls.lock().unwrap().is_empty());
    }
}

#[test]
fn duplicate_launch_is_blocked_while_the_game_process_is_running() {
    let temp = tempfile::tempdir().unwrap();
    let install_dir = temp.path().join("game");
    fs::create_dir(&install_dir).unwrap();
    fs::write(install_dir.join("Voxtera.exe"), b"verified-game").unwrap();
    let runner = FakeRunner::default();
    let launcher = GameLauncher::new(runner);
    let manifest = manifest_for(b"verified-game");
    launcher.launch(&install_dir, &manifest, || Ok(())).unwrap();

    let error = launcher
        .launch(&install_dir, &manifest, || Ok(()))
        .unwrap_err();

    assert_eq!(error.code, LauncherErrorCode::GameRunning);
}

#[test]
fn pending_rollback_is_confirmed_only_after_a_successful_spawn() {
    let temp = tempfile::tempdir().unwrap();
    let install_dir = temp.path().join("game");
    fs::create_dir(&install_dir).unwrap();
    fs::write(install_dir.join("Voxtera.exe"), b"verified-game").unwrap();
    let confirmations = Arc::new(Mutex::new(0));

    let failed = GameLauncher::new(FakeRunner {
        fail_spawn: true,
        ..FakeRunner::default()
    });
    let count = Arc::clone(&confirmations);
    failed
        .launch(&install_dir, &manifest_for(b"verified-game"), move || {
            *count.lock().unwrap() += 1;
            Ok(())
        })
        .unwrap_err();
    assert_eq!(*confirmations.lock().unwrap(), 0);

    let successful = GameLauncher::new(FakeRunner::default());
    let count = Arc::clone(&confirmations);
    successful
        .launch(&install_dir, &manifest_for(b"verified-game"), move || {
            *count.lock().unwrap() += 1;
            Ok(())
        })
        .unwrap();
    assert_eq!(*confirmations.lock().unwrap(), 1);
}
