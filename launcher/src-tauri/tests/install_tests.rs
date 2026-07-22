use std::fs;

use launcher_core::{FailurePoint, InstallManager, PromotionRequest};

fn write(path: &std::path::Path, contents: &[u8]) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

fn journal_path_value(journal: &std::path::Path, field: &str) -> std::path::PathBuf {
    let value: serde_json::Value = serde_json::from_slice(&fs::read(journal).unwrap()).unwrap();
    std::path::PathBuf::from(value[field].as_str().unwrap())
}

fn promoted_fixture() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    InstallManager,
    launcher_core::PromotionReceipt,
) {
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
    (temp, live, manager, receipt)
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
fn recovery_reconciles_crash_between_live_rename_and_phase_update() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    write(&live.join("Voxtera.exe"), b"old-game");
    let staging = manager.create_staging().unwrap();
    write(&staging.join("Voxtera.exe"), b"new-game");

    assert!(
        manager
            .promote(PromotionRequest {
                staging_dir: staging,
                failure_point: Some(FailurePoint::CrashAfterLiveRenameBeforeJournal),
            })
            .is_err()
    );
    assert!(!live.exists());

    manager.recover().unwrap();
    manager.recover().unwrap();

    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"old-game");
}

#[test]
fn recovery_cancels_promotion_crash_before_live_rename() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    write(&live.join("Voxtera.exe"), b"old-game");
    let staging = manager.create_staging().unwrap();
    write(&staging.join("Voxtera.exe"), b"new-game");

    assert!(
        manager
            .promote(PromotionRequest {
                staging_dir: staging.clone(),
                failure_point: Some(FailurePoint::CrashBeforeLiveRename),
            })
            .is_err()
    );
    manager.recover().unwrap();

    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"old-game");
    assert!(!staging.exists());
}

#[test]
fn recovery_cancels_promotion_crash_before_new_live_rename() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    write(&live.join("Voxtera.exe"), b"old-game");
    let staging = manager.create_staging().unwrap();
    write(&staging.join("Voxtera.exe"), b"new-game");

    assert!(
        manager
            .promote(PromotionRequest {
                staging_dir: staging,
                failure_point: Some(FailurePoint::CrashBeforeNewLiveRename),
            })
            .is_err()
    );
    manager.recover().unwrap();

    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"old-game");
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

#[test]
fn staging_preserved_root_is_rejected_even_when_old_install_has_no_matching_root() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    write(&live.join("Voxtera.exe"), b"old-game");
    let staging = manager.create_staging().unwrap();
    write(&staging.join("Voxtera.exe"), b"new-game");
    write(&staging.join("userdata/injected.ron"), b"injected");

    assert!(
        manager
            .promote(PromotionRequest {
                staging_dir: staging,
                failure_point: None,
            })
            .is_err()
    );
    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"old-game");
    assert!(!live.join("userdata").exists());
}

#[test]
fn rollback_preserved_copy_failure_keeps_promoted_live_and_rollback() {
    let (_temp, live, manager, receipt) = promoted_fixture();

    assert!(
        manager
            .rollback_with_failure(&receipt, Some(FailurePoint::FailRollbackPreservedCopy))
            .is_err()
    );
    manager.recover().unwrap();

    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"new-game");
    assert!(receipt.rollback_dir.exists());
}

#[test]
fn rollback_recovery_handles_every_destructive_rename_window() {
    for point in [
        FailurePoint::CrashBeforeRollbackLiveRename,
        FailurePoint::CrashAfterRollbackLiveRenameBeforeJournal,
        FailurePoint::CrashBeforeRollbackCandidateRename,
        FailurePoint::CrashAfterRollbackCandidateRenameBeforeJournal,
    ] {
        let (_temp, live, manager, receipt) = promoted_fixture();
        assert!(
            manager
                .rollback_with_failure(&receipt, Some(point))
                .is_err()
        );

        manager.recover().unwrap();
        manager.recover().unwrap();

        assert_eq!(
            fs::read(live.join("Voxtera.exe")).unwrap(),
            b"new-game",
            "failed recovery at {point:?}"
        );
        assert!(receipt.rollback_dir.exists(), "rollback lost at {point:?}");
        assert_eq!(
            fs::read(live.join("userdata/save.ron")).unwrap(),
            b"save-before-update"
        );
    }
}

#[test]
fn confirmation_recovery_is_idempotent_in_every_window() {
    for point in [
        FailurePoint::CrashBeforeConfirmationRename,
        FailurePoint::CrashAfterConfirmationRenameBeforeJournal,
        FailurePoint::CrashAfterConfirmationCleanup,
    ] {
        let (_temp, live, manager, receipt) = promoted_fixture();
        assert!(
            manager
                .confirm_first_launch_with_failure(&receipt, Some(point))
                .is_err()
        );

        manager.recover().unwrap();
        manager.recover().unwrap();

        assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"new-game");
        assert!(
            !receipt.rollback_dir.exists(),
            "rollback remains at {point:?}"
        );
        assert!(
            !receipt.journal_path.exists(),
            "journal remains at {point:?}"
        );
    }
}

#[test]
fn recovery_rejects_confirmation_journal_with_mismatched_phase() {
    let (_temp, live, manager, receipt) = promoted_fixture();
    assert!(
        manager
            .confirm_first_launch_with_failure(
                &receipt,
                Some(FailurePoint::CrashBeforeConfirmationRename),
            )
            .is_err()
    );
    let journal = fs::read_to_string(&receipt.journal_path).unwrap();
    let corrupted = journal.replace("confirmation_move_intent", "rollback_applied");
    fs::write(&receipt.journal_path, corrupted).unwrap();

    assert!(manager.recover().is_err());
    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"new-game");
    assert!(receipt.rollback_dir.exists());
}

#[test]
fn promotion_recovery_resumes_after_cleanup_before_journal_removal() {
    let temp = tempfile::tempdir().unwrap();
    let live = temp.path().join("game");
    let manager = InstallManager::new(live.clone());
    write(&live.join("Voxtera.exe"), b"old-game");
    let staging = manager.create_staging().unwrap();
    write(&staging.join("Voxtera.exe"), b"new-game");
    assert!(
        manager
            .promote(PromotionRequest {
                staging_dir: staging,
                failure_point: Some(FailurePoint::CrashAfterNewLiveRenamed),
            })
            .is_err()
    );
    let journal = temp.path().join("game.transaction.json");
    let recorded_staging = journal_path_value(&journal, "staging_dir");
    let rollback = temp.path().join("game.rollback");
    fs::rename(&live, &recorded_staging).unwrap();
    fs::rename(&rollback, &live).unwrap();
    fs::remove_dir_all(&recorded_staging).unwrap();

    manager.recover().unwrap();
    manager.recover().unwrap();

    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"old-game");
    assert!(!journal.exists());
}

#[test]
fn rollback_recovery_resumes_after_candidate_cleanup_before_journal_rewrite() {
    let (_temp, live, manager, receipt) = promoted_fixture();
    assert!(
        manager
            .rollback_with_failure(&receipt, Some(FailurePoint::CrashBeforeRollbackLiveRename),)
            .is_err()
    );
    let candidate = journal_path_value(&receipt.journal_path, "staging_dir");
    fs::remove_dir_all(candidate).unwrap();

    manager.recover().unwrap();
    manager.recover().unwrap();

    assert_eq!(fs::read(live.join("Voxtera.exe")).unwrap(), b"new-game");
    assert!(receipt.rollback_dir.exists());
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
