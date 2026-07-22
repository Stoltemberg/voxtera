use std::{
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{archive::is_link_or_reparse, config::replace_file_atomic};

const PRESERVED_PATHS: [&str; 3] = ["userdata", "screenshots", "settings"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailurePoint {
    CrashAfterJournalPrepared,
    AfterLiveMoved,
    CrashAfterLiveMoved,
    AfterNewLiveRenamed,
    CrashAfterNewLiveRenamed,
}

#[derive(Debug, Clone)]
pub struct PromotionRequest {
    pub staging_dir: PathBuf,
    pub failure_point: Option<FailurePoint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromotionReceipt {
    pub installation_dir: PathBuf,
    pub rollback_dir: PathBuf,
    pub journal_path: PathBuf,
    pub transaction_id: Uuid,
}

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error("installation path is unsafe: {0}")]
    UnsafePath(String),
    #[error("the game appears to be running")]
    GameRunning,
    #[error("an update is already waiting for first-launch confirmation")]
    PendingConfirmation,
    #[error("installation transaction failed")]
    Io(#[source] std::io::Error),
    #[error("installation journal is invalid")]
    InvalidJournal,
    #[error("injected promotion failure")]
    InjectedFailure,
}

#[derive(Debug, Clone)]
pub struct InstallManager {
    installation_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TransactionPhase {
    Prepared,
    LiveMoved,
    Promoted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionJournal {
    transaction_id: Uuid,
    staging_dir: PathBuf,
    phase: TransactionPhase,
    had_previous_installation: bool,
}

impl InstallManager {
    pub fn new(installation_dir: PathBuf) -> Self { Self { installation_dir } }

    pub fn create_staging(&self) -> Result<PathBuf, InstallError> {
        let parent = self.parent()?;
        fs::create_dir_all(parent).map_err(InstallError::Io)?;
        if is_link_or_reparse(parent)
            .map_err(|error| InstallError::UnsafePath(format!("installation parent: {error}")))?
        {
            return Err(InstallError::UnsafePath(
                "installation parent is a reparse point".to_owned(),
            ));
        }
        let staging = parent.join(format!(
            "{}.staging-{}",
            self.install_name()?,
            Uuid::new_v4()
        ));
        fs::create_dir(&staging).map_err(InstallError::Io)?;
        Ok(staging)
    }

    pub fn promote(&self, request: PromotionRequest) -> Result<PromotionReceipt, InstallError> {
        self.recover()?;
        self.validate_owned_staging(&request.staging_dir)?;
        validate_real_tree(&request.staging_dir)?;
        self.preflight_locked_executable()?;

        let rollback = self.rollback_dir()?;
        if rollback.exists() {
            return Err(InstallError::PendingConfirmation);
        }
        let journal_path = self.journal_path()?;
        let mut journal = TransactionJournal {
            transaction_id: Uuid::new_v4(),
            staging_dir: request.staging_dir.clone(),
            phase: TransactionPhase::Prepared,
            had_previous_installation: self.installation_dir.exists(),
        };
        self.write_journal(&journal)?;
        if request.failure_point == Some(FailurePoint::CrashAfterJournalPrepared) {
            return Err(InstallError::InjectedFailure);
        }

        if journal.had_previous_installation {
            validate_real_tree(&self.installation_dir)?;
            fs::rename(&self.installation_dir, &rollback).map_err(|error| {
                let _ = remove_file_if_present(&journal_path);
                if error.kind() == std::io::ErrorKind::PermissionDenied {
                    InstallError::GameRunning
                } else {
                    InstallError::Io(error)
                }
            })?;
        }
        journal.phase = TransactionPhase::LiveMoved;
        if let Err(error) = self.write_journal(&journal) {
            let _ = self.restore_after_failed_promotion(
                &request.staging_dir,
                &rollback,
                journal.had_previous_installation,
            );
            return Err(error);
        }

        match request.failure_point {
            Some(FailurePoint::CrashAfterLiveMoved) => return Err(InstallError::InjectedFailure),
            Some(FailurePoint::AfterLiveMoved) => {
                self.restore_after_failed_promotion(
                    &request.staging_dir,
                    &rollback,
                    journal.had_previous_installation,
                )?;
                return Err(InstallError::InjectedFailure);
            },
            Some(
                FailurePoint::CrashAfterJournalPrepared
                | FailurePoint::AfterNewLiveRenamed
                | FailurePoint::CrashAfterNewLiveRenamed,
            )
            | None => {},
        }

        let promotion_result = (|| {
            if journal.had_previous_installation {
                copy_preserved(&rollback, &request.staging_dir, false)?;
            }
            fs::rename(&request.staging_dir, &self.installation_dir).map_err(InstallError::Io)?;
            if matches!(
                request.failure_point,
                Some(FailurePoint::AfterNewLiveRenamed | FailurePoint::CrashAfterNewLiveRenamed)
            ) {
                return Err(InstallError::InjectedFailure);
            }
            journal.phase = TransactionPhase::Promoted;
            self.write_journal(&journal)
        })();
        if let Err(error) = promotion_result {
            if matches!(error, InstallError::InjectedFailure)
                && request.failure_point == Some(FailurePoint::CrashAfterNewLiveRenamed)
            {
                return Err(error);
            }
            self.restore_after_failed_promotion(
                &request.staging_dir,
                &rollback,
                journal.had_previous_installation,
            )?;
            return Err(error);
        }

        Ok(PromotionReceipt {
            installation_dir: self.installation_dir.clone(),
            rollback_dir: rollback,
            journal_path,
            transaction_id: journal.transaction_id,
        })
    }

    pub fn recover(&self) -> Result<(), InstallError> {
        let journal_path = self.journal_path()?;
        let bytes = match fs::read(&journal_path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(InstallError::Io(error)),
        };
        let journal: TransactionJournal =
            serde_json::from_slice(&bytes).map_err(|_| InstallError::InvalidJournal)?;
        self.validate_owned_staging(&journal.staging_dir)?;
        let rollback = self.rollback_dir()?;

        match journal.phase {
            TransactionPhase::Prepared => {
                self.cleanup_owned_staging(&journal.staging_dir)?;
                remove_file_if_present(&journal_path)?;
            },
            TransactionPhase::LiveMoved => {
                if self.installation_dir.exists() {
                    if journal.staging_dir.exists() {
                        return Err(InstallError::InvalidJournal);
                    }
                    fs::rename(&self.installation_dir, &journal.staging_dir)
                        .map_err(InstallError::Io)?;
                }
                if journal.had_previous_installation {
                    if !rollback.exists() {
                        return Err(InstallError::InvalidJournal);
                    }
                    fs::rename(&rollback, &self.installation_dir).map_err(InstallError::Io)?;
                }
                self.cleanup_owned_staging(&journal.staging_dir)?;
                remove_file_if_present(&journal_path)?;
            },
            TransactionPhase::Promoted => {
                if !self.installation_dir.exists()
                    || (journal.had_previous_installation && !rollback.exists())
                {
                    return Err(InstallError::InvalidJournal);
                }
            },
        }
        Ok(())
    }

    pub fn rollback(&self, receipt: &PromotionReceipt) -> Result<(), InstallError> {
        self.validate_receipt(receipt)?;
        if !receipt.rollback_dir.exists() {
            return Err(InstallError::InvalidJournal);
        }
        validate_real_tree(&self.installation_dir)?;
        validate_real_tree(&receipt.rollback_dir)?;
        let displaced = self.owned_sibling("staging", Uuid::new_v4())?;
        fs::rename(&self.installation_dir, &displaced).map_err(InstallError::Io)?;
        if let Err(error) = fs::rename(&receipt.rollback_dir, &self.installation_dir) {
            let _ = fs::rename(&displaced, &self.installation_dir);
            return Err(InstallError::Io(error));
        }
        copy_preserved(&displaced, &self.installation_dir, true)?;
        self.cleanup_owned_staging(&displaced)?;
        remove_file_if_present(&receipt.journal_path)
    }

    pub fn confirm_first_launch(&self, receipt: &PromotionReceipt) -> Result<(), InstallError> {
        self.validate_receipt(receipt)?;
        if receipt.rollback_dir.exists() {
            validate_real_tree(&receipt.rollback_dir)?;
            fs::remove_dir_all(&receipt.rollback_dir).map_err(InstallError::Io)?;
        }
        remove_file_if_present(&receipt.journal_path)
    }

    fn restore_after_failed_promotion(
        &self,
        staging: &Path,
        rollback: &Path,
        had_previous: bool,
    ) -> Result<(), InstallError> {
        if self.installation_dir.exists() {
            if staging.exists() {
                return Err(InstallError::InvalidJournal);
            }
            fs::rename(&self.installation_dir, staging).map_err(InstallError::Io)?;
        }
        if had_previous {
            fs::rename(rollback, &self.installation_dir).map_err(InstallError::Io)?;
        }
        self.cleanup_owned_staging(staging)?;
        remove_file_if_present(&self.journal_path()?)
    }

    fn cleanup_owned_staging(&self, path: &Path) -> Result<(), InstallError> {
        self.validate_owned_staging(path)?;
        if path.exists() {
            validate_real_tree(path)?;
            fs::remove_dir_all(path).map_err(InstallError::Io)?;
        }
        Ok(())
    }

    fn validate_owned_staging(&self, staging: &Path) -> Result<(), InstallError> {
        let expected_parent = self.parent()?;
        if staging.parent() != Some(expected_parent) {
            return Err(InstallError::UnsafePath(
                "staging is not a sibling of the installation".to_owned(),
            ));
        }
        let name = staging
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| InstallError::UnsafePath("non-Unicode staging name".to_owned()))?;
        let prefix = format!("{}.staging-", self.install_name()?);
        let suffix = name
            .strip_prefix(&prefix)
            .ok_or_else(|| InstallError::UnsafePath("staging is not launcher-owned".to_owned()))?;
        Uuid::parse_str(suffix)
            .map_err(|_| InstallError::UnsafePath("invalid staging identifier".to_owned()))?;
        Ok(())
    }

    fn validate_receipt(&self, receipt: &PromotionReceipt) -> Result<(), InstallError> {
        if receipt.installation_dir != self.installation_dir
            || receipt.rollback_dir != self.rollback_dir()?
            || receipt.journal_path != self.journal_path()?
        {
            return Err(InstallError::UnsafePath(
                "receipt belongs to another installation".to_owned(),
            ));
        }
        let journal: TransactionJournal =
            serde_json::from_slice(&fs::read(&receipt.journal_path).map_err(InstallError::Io)?)
                .map_err(|_| InstallError::InvalidJournal)?;
        if journal.transaction_id != receipt.transaction_id
            || journal.phase != TransactionPhase::Promoted
        {
            return Err(InstallError::InvalidJournal);
        }
        Ok(())
    }

    fn preflight_locked_executable(&self) -> Result<(), InstallError> {
        let executable = self.installation_dir.join("Voxtera.exe");
        if !executable.exists() {
            return Ok(());
        }
        OpenOptions::new()
            .read(true)
            .write(true)
            .open(executable)
            .map(|_| ())
            .map_err(|_| InstallError::GameRunning)
    }

    fn write_journal(&self, journal: &TransactionJournal) -> Result<(), InstallError> {
        let journal_path = self.journal_path()?;
        let temp_path = journal_path.with_extension("json.tmp");
        let file = File::create(&temp_path).map_err(InstallError::Io)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer_pretty(&mut writer, journal)
            .map_err(|_| InstallError::InvalidJournal)?;
        writer.flush().map_err(InstallError::Io)?;
        writer.get_ref().sync_all().map_err(InstallError::Io)?;
        drop(writer);
        replace_file_atomic(&temp_path, &journal_path).map_err(InstallError::Io)
    }

    fn parent(&self) -> Result<&Path, InstallError> {
        self.installation_dir.parent().ok_or_else(|| {
            InstallError::UnsafePath("installation has no parent directory".to_owned())
        })
    }

    fn install_name(&self) -> Result<&str, InstallError> {
        self.installation_dir
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| InstallError::UnsafePath("invalid installation name".to_owned()))
    }

    fn rollback_dir(&self) -> Result<PathBuf, InstallError> {
        Ok(self
            .parent()?
            .join(format!("{}.rollback", self.install_name()?)))
    }

    fn journal_path(&self) -> Result<PathBuf, InstallError> {
        Ok(self
            .parent()?
            .join(format!("{}.transaction.json", self.install_name()?)))
    }

    fn owned_sibling(&self, kind: &str, id: Uuid) -> Result<PathBuf, InstallError> {
        Ok(self
            .parent()?
            .join(format!("{}.{}-{}", self.install_name()?, kind, id)))
    }
}

fn validate_real_tree(root: &Path) -> Result<(), InstallError> {
    let metadata = fs::symlink_metadata(root).map_err(InstallError::Io)?;
    if !metadata.is_dir()
        || is_link_or_reparse(root)
            .map_err(|error| InstallError::UnsafePath(format!("unsafe tree: {error}")))?
    {
        return Err(InstallError::UnsafePath(
            "installation tree root is unsafe".to_owned(),
        ));
    }
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|error| {
            InstallError::UnsafePath(format!("installation traversal failed: {error}"))
        })?;
        if entry.file_type().is_symlink()
            || is_link_or_reparse(entry.path())
                .map_err(|error| InstallError::UnsafePath(format!("unsafe entry: {error}")))?
        {
            return Err(InstallError::UnsafePath(
                "installation contains a link or reparse point".to_owned(),
            ));
        }
    }
    Ok(())
}

fn copy_preserved(source: &Path, destination: &Path, overwrite: bool) -> Result<(), InstallError> {
    for preserved in PRESERVED_PATHS {
        let source_root = source.join(preserved);
        if !source_root.exists() {
            continue;
        }
        validate_real_tree(&source_root)?;
        let destination_root = destination.join(preserved);
        if destination_root.exists() && !overwrite {
            return Err(InstallError::UnsafePath(
                "staging attempted to provide a preserved path".to_owned(),
            ));
        }
        fs::create_dir_all(&destination_root).map_err(InstallError::Io)?;
        for entry in WalkDir::new(&source_root).min_depth(1).follow_links(false) {
            let entry = entry.map_err(|error| {
                InstallError::UnsafePath(format!("preserved path traversal failed: {error}"))
            })?;
            let relative = entry
                .path()
                .strip_prefix(&source_root)
                .map_err(|_| InstallError::UnsafePath("preserved path escaped".to_owned()))?;
            let target = destination_root.join(relative);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&target).map_err(InstallError::Io)?;
            } else if entry.file_type().is_file() {
                if !overwrite && target.exists() {
                    return Err(InstallError::UnsafePath(
                        "preserved file would be overwritten".to_owned(),
                    ));
                }
                fs::copy(entry.path(), target).map_err(InstallError::Io)?;
            } else {
                return Err(InstallError::UnsafePath(
                    "preserved path contains a special entry".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn remove_file_if_present(path: &Path) -> Result<(), InstallError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(InstallError::Io(error)),
    }
}
