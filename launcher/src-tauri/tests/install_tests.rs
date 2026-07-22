use std::fs;

use launcher_core::{FailurePoint, InstallManager, PromotionRequest};

fn write(path: &std::path::Path, contents: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

#[test]
fn failed_promotion_restores_previous_installation_and_player_data() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    let staging = manager.create_staging().unwrap();
    write(&live.join("Voxtera.exe"), b"old-game");
    write(&live.join("userdata/save.ron"), b"player-save");
    write(&staging.join("Voxtera.exe"), b"new-game");

    let result = manager.promote(PromotionRequest {
        staging_dir: staging,
        failure_point: Some(FailurePoint::AfterLiveMoved),
    });

    assert!(result.is_err());
    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"old-game");
    assert_eq!(
        fs::read(live.join("userdata/save.ron")).unwrap(),
        b"player-save"
    );
}

#[test]
fn promotion_requires_launcher_owned_same_parent_staging() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    let foreign = temp.path().join("nested").join("stage");
    fs::create_dir_all(&foreign).unwrap();
    write(&foreign.join("Voxtera.exe"), b"new-game");

    assert!(
        manager
            .promote(PromotionRequest {
                staging_dir: foreign.clone(),
                failure_point: None,
            })
            .is_err()
    );
    assert!(foreign.exists());
    assert!(!live.exists());
}

#[test]
fn promotion_retains_rollback_and_confirmation_removes_only_rollback() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    write(&live.join("Voxtera.exe"), b"old-game");
    write(&live.join("settings/video.ron"), b"player-settings");
    let unrelated = temp.path().join("game.staging-not-owned");
    write(&unrelated.join("keep.txt"), b"keep");
    let staging = manager.create_staging().unwrap();
    write(&staging.join("Voxtera.exe"), b"new-game");

    let receipt = manager
        .promote(PromotionRequest {
            staging_dir: staging,
            failure_point: None,
        })
        .unwrap();

    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"new-game");
    assert_eq!(
        fs::read(live.join("settings/video.ron")).unwrap(),
        b"player-settings"
    );
    assert!(receipt.rollback_dir.exists());
    assert!(unrelated.exists());
    manager.confirm_first_launch(&receipt).unwrap();
    assert!(!receipt.rollback_dir.exists());
    assert!(unrelated.exists());
}

#[test]
fn explicit_rollback_restores_old_game_and_current_player_data() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    write(&live.join("Voxtera.exe"), b"old-game");
    write(&live.join("userdata/save.ron"), b"save-before-update");
    let staging = manager.create_staging().unwrap();
    write(&staging.join("Voxtera.exe"), b"new-game");
    let receipt = manager
        .promote(PromotionRequest {
            staging_dir: staging,
            failure_point: None,
        })
        .unwrap();
    write(&live.join("userdata/save.ron"), b"save-after-update");

    manager.rollback(&receipt).unwrap();

    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"old-game");
    assert_eq!(
        fs::read(live.join("userdata/save.ron")).unwrap(),
        b"save-after-update"
    );
}

#[test]
fn startup_recovery_restores_live_after_interrupted_first_rename() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    write(&live.join("Voxtera.exe"), b"old-game");
    write(&live.join("screenshots/old.png"), b"screenshot");
    let staging = manager.create_staging().unwrap();
    write(&staging.join("Voxtera.exe"), b"new-game");

    let result = manager.promote(PromotionRequest {
        staging_dir: staging,
        failure_point: Some(FailurePoint::CrashAfterLiveMoved),
    });
    assert!(result.is_err());
    assert!(!live.exists());

    InstallManager::new(live.clone()).recover().unwrap();

    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"old-game");
    assert_eq!(
        fs::read(live.join("screenshots/old.png")).unwrap(),
        b"screenshot"
    );
}

#[test]
fn startup_recovery_cleans_owned_staging_after_crash_before_first_rename() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    write(&live.join("Voxtera.exe"), b"old-game");
    let staging = manager.create_staging().unwrap();
    write(&staging.join("Voxtera.exe"), b"new-game");

    let result = manager.promote(PromotionRequest {
        staging_dir: staging.clone(),
        failure_point: Some(FailurePoint::CrashAfterJournalPrepared),
    });
    assert!(result.is_err());

    InstallManager::new(live.clone()).recover().unwrap();

    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"old-game");
    assert!(!staging.exists());
}

#[test]
fn startup_recovery_rolls_back_crash_after_new_live_rename() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    write(&live.join("Voxtera.exe"), b"old-game");
    write(&live.join("userdata/save.ron"), b"player-save");
    let staging = manager.create_staging().unwrap();
    write(&staging.join("Voxtera.exe"), b"new-game");

    let result = manager.promote(PromotionRequest {
        staging_dir: staging,
        failure_point: Some(FailurePoint::CrashAfterNewLiveRenamed),
    });
    assert!(result.is_err());
    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"new-game");

    InstallManager::new(live.clone()).recover().unwrap();

    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"old-game");
    assert_eq!(
        fs::read(live.join("userdata/save.ron")).unwrap(),
        b"player-save"
    );
}

#[test]
fn failed_promotion_after_new_live_rename_restores_old_installation_immediately() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    write(&live.join("Voxtera.exe"), b"old-game");
    write(&live.join("settings/audio.ron"), b"player-settings");
    let staging = manager.create_staging().unwrap();
    write(&staging.join("Voxtera.exe"), b"new-game");

    let result = manager.promote(PromotionRequest {
        staging_dir: staging,
        failure_point: Some(FailurePoint::AfterNewLiveRenamed),
    });

    assert!(result.is_err());
    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"old-game");
    assert_eq!(
        fs::read(live.join("settings/audio.ron")).unwrap(),
        b"player-settings"
    );
}

#[cfg(windows)]
#[test]
fn locked_live_executable_leaves_installation_unchanged() {
    use std::os::windows::fs::OpenOptionsExt;

    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    write(&live.join("Voxtera.exe"), b"old-game");
    let locked = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(live.join("Voxtera.exe"))
        .unwrap();
    let staging = manager.create_staging().unwrap();
    write(&staging.join("Voxtera.exe"), b"new-game");

    let result = manager.promote(PromotionRequest {
        staging_dir: staging,
        failure_point: None,
    });
    drop(locked);

    assert!(result.is_err());
    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"old-game");
}
