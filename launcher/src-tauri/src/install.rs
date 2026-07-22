use std::{
    fs::{self, File, OpenOptions},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use walkdir::WalkDir;

use crate::{config::replace_file_atomic, integrity::is_link_or_reparse};

const PRESERVED_PATHS: [&str; 3] = ["userdata", "screenshots", "settings"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailurePoint {
    CrashAfterJournalPrepared,
    CrashBeforeLiveRename,
    CrashAfterLiveRenameBeforeJournal,
    AfterLiveMoved,
    CrashAfterLiveMoved,
    CrashBeforeNewLiveRename,
    AfterNewLiveRenamed,
    CrashAfterNewLiveRenamed,
    FailRollbackPreservedCopy,
    CrashBeforeRollbackLiveRename,
    CrashAfterRollbackLiveRenameBeforeJournal,
    CrashBeforeRollbackCandidateRename,
    CrashAfterRollbackCandidateRenameBeforeJournal,
    CrashBeforeConfirmationRename,
    CrashAfterConfirmationRenameBeforeJournal,
    CrashAfterConfirmationCleanup,
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
    #[error("injected transaction failure")]
    InjectedFailure,
}

#[derive(Debug, Clone)]
pub struct InstallManager {
    installation_dir: PathBuf,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TransactionOperation {
    Promotion,
    Rollback,
    Confirmation,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TransactionPhase {
    PromotionLiveMoveIntent,
    PromotionLiveMoved,
    PromotionNewLiveMoveIntent,
    Promoted,
    RollbackPrepared,
    RollbackLiveMoveIntent,
    RollbackLiveMoved,
    RollbackCandidateMoveIntent,
    RollbackApplied,
    ConfirmationMoveIntent,
    ConfirmationMoved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionJournal {
    transaction_id: Uuid,
    operation: TransactionOperation,
    phase: TransactionPhase,
    staging_dir: PathBuf,
    auxiliary_dir: Option<PathBuf>,
    had_previous_installation: bool,
}

impl InstallManager {
    pub fn new(installation_dir: PathBuf) -> Self { Self { installation_dir } }

    pub fn create_staging(&self) -> Result<PathBuf, InstallError> {
        let parent = self.parent()?;
        fs::create_dir_all(parent).map_err(InstallError::Io)?;
        if is_link_or_reparse(parent).map_err(InstallError::Io)? {
            return Err(InstallError::UnsafePath(
                "installation parent is a reparse point".to_owned(),
            ));
        }
        let staging = self.owned_sibling("staging", Uuid::new_v4())?;
        fs::create_dir(&staging).map_err(InstallError::Io)?;
        Ok(staging)
    }

    pub fn promote(&self, request: PromotionRequest) -> Result<PromotionReceipt, InstallError> {
        self.recover()?;
        self.validate_owned_path(&request.staging_dir, "staging")?;
        validate_real_tree(&request.staging_dir)?;
        ensure_preserved_destinations_absent(&request.staging_dir)?;
        self.preflight_locked_executable()?;

        let rollback = self.rollback_dir()?;
        if rollback.exists() {
            return Err(InstallError::PendingConfirmation);
        }
        let had_previous = self.installation_dir.exists();
        if had_previous {
            validate_real_tree(&self.installation_dir)?;
        }
        let mut journal = TransactionJournal {
            transaction_id: Uuid::new_v4(),
            operation: TransactionOperation::Promotion,
            phase: if had_previous {
                TransactionPhase::PromotionLiveMoveIntent
            } else {
                TransactionPhase::PromotionNewLiveMoveIntent
            },
            staging_dir: request.staging_dir.clone(),
            auxiliary_dir: None,
            had_previous_installation: had_previous,
        };
        self.write_journal(&journal)?;

        if matches!(
            request.failure_point,
            Some(FailurePoint::CrashAfterJournalPrepared | FailurePoint::CrashBeforeLiveRename)
        ) {
            return Err(InstallError::InjectedFailure);
        }

        if had_previous {
            fs::rename(&self.installation_dir, &rollback).map_err(|error| {
                if error.kind() == std::io::ErrorKind::PermissionDenied {
                    InstallError::GameRunning
                } else {
                    InstallError::Io(error)
                }
            })?;
            if request.failure_point == Some(FailurePoint::CrashAfterLiveRenameBeforeJournal) {
                return Err(InstallError::InjectedFailure);
            }
            journal.phase = TransactionPhase::PromotionLiveMoved;
            if let Err(error) = self.write_journal(&journal) {
                let _ = self.recover();
                return Err(error);
            }
            if request.failure_point == Some(FailurePoint::CrashAfterLiveMoved) {
                return Err(InstallError::InjectedFailure);
            }
            if request.failure_point == Some(FailurePoint::AfterLiveMoved) {
                self.recover()?;
                return Err(InstallError::InjectedFailure);
            }
            if let Err(error) = copy_preserved(&rollback, &request.staging_dir) {
                self.recover()?;
                return Err(error);
            }
            journal.phase = TransactionPhase::PromotionNewLiveMoveIntent;
            if let Err(error) = self.write_journal(&journal) {
                let _ = self.recover();
                return Err(error);
            }
        }

        if request.failure_point == Some(FailurePoint::CrashBeforeNewLiveRename) {
            return Err(InstallError::InjectedFailure);
        }
        fs::rename(&request.staging_dir, &self.installation_dir).map_err(InstallError::Io)?;
        if request.failure_point == Some(FailurePoint::CrashAfterNewLiveRenamed) {
            return Err(InstallError::InjectedFailure);
        }
        if request.failure_point == Some(FailurePoint::AfterNewLiveRenamed) {
            self.recover()?;
            return Err(InstallError::InjectedFailure);
        }
        journal.phase = TransactionPhase::Promoted;
        if let Err(error) = self.write_journal(&journal) {
            let _ = self.recover();
            return Err(error);
        }

        self.receipt(journal.transaction_id)
    }

    pub fn recover(&self) -> Result<(), InstallError> {
        let Some(journal) = self.read_journal()? else {
            return Ok(());
        };
        match journal.operation {
            TransactionOperation::Promotion => self.recover_promotion(&journal),
            TransactionOperation::Rollback => self.recover_rollback(&journal),
            TransactionOperation::Confirmation => self.recover_confirmation(&journal),
        }
    }

    pub fn rollback(&self, receipt: &PromotionReceipt) -> Result<(), InstallError> {
        self.rollback_with_failure(receipt, None)
    }

    pub fn rollback_with_failure(
        &self,
        receipt: &PromotionReceipt,
        failure_point: Option<FailurePoint>,
    ) -> Result<(), InstallError> {
        self.validate_receipt(receipt)?;
        validate_real_tree(&self.installation_dir)?;
        validate_real_tree(&receipt.rollback_dir)?;

        let candidate = self.create_staging()?;
        let displaced = self.owned_sibling("staging", Uuid::new_v4())?;
        let mut journal = TransactionJournal {
            transaction_id: receipt.transaction_id,
            operation: TransactionOperation::Rollback,
            phase: TransactionPhase::RollbackPrepared,
            staging_dir: candidate.clone(),
            auxiliary_dir: Some(displaced.clone()),
            had_previous_installation: true,
        };
        self.write_journal(&journal)?;

        let copy_result = (|| {
            copy_tree_excluding_preserved(&receipt.rollback_dir, &candidate)?;
            if failure_point == Some(FailurePoint::FailRollbackPreservedCopy) {
                return Err(InstallError::InjectedFailure);
            }
            copy_preserved(&self.installation_dir, &candidate)
        })();
        if let Err(error) = copy_result {
            self.recover()?;
            return Err(error);
        }

        journal.phase = TransactionPhase::RollbackLiveMoveIntent;
        self.write_journal(&journal)?;
        if failure_point == Some(FailurePoint::CrashBeforeRollbackLiveRename) {
            return Err(InstallError::InjectedFailure);
        }
        fs::rename(&self.installation_dir, &displaced).map_err(InstallError::Io)?;
        if failure_point == Some(FailurePoint::CrashAfterRollbackLiveRenameBeforeJournal) {
            return Err(InstallError::InjectedFailure);
        }

        journal.phase = TransactionPhase::RollbackLiveMoved;
        self.write_journal(&journal)?;
        journal.phase = TransactionPhase::RollbackCandidateMoveIntent;
        self.write_journal(&journal)?;
        if failure_point == Some(FailurePoint::CrashBeforeRollbackCandidateRename) {
            return Err(InstallError::InjectedFailure);
        }
        fs::rename(&candidate, &self.installation_dir).map_err(InstallError::Io)?;
        if failure_point == Some(FailurePoint::CrashAfterRollbackCandidateRenameBeforeJournal) {
            return Err(InstallError::InjectedFailure);
        }

        journal.phase = TransactionPhase::RollbackApplied;
        self.write_journal(&journal)?;
        self.finish_applied_rollback(&journal)
    }

    pub fn confirm_first_launch(&self, receipt: &PromotionReceipt) -> Result<(), InstallError> {
        self.confirm_first_launch_with_failure(receipt, None)
    }

    pub fn confirm_first_launch_with_failure(
        &self,
        receipt: &PromotionReceipt,
        failure_point: Option<FailurePoint>,
    ) -> Result<(), InstallError> {
        self.validate_receipt(receipt)?;
        let cleanup = self.owned_sibling("cleanup", Uuid::new_v4())?;
        let mut journal = TransactionJournal {
            transaction_id: receipt.transaction_id,
            operation: TransactionOperation::Confirmation,
            phase: TransactionPhase::ConfirmationMoveIntent,
            staging_dir: cleanup.clone(),
            auxiliary_dir: None,
            had_previous_installation: true,
        };
        self.write_journal(&journal)?;
        if failure_point == Some(FailurePoint::CrashBeforeConfirmationRename) {
            return Err(InstallError::InjectedFailure);
        }
        fs::rename(&receipt.rollback_dir, &cleanup).map_err(InstallError::Io)?;
        if failure_point == Some(FailurePoint::CrashAfterConfirmationRenameBeforeJournal) {
            return Err(InstallError::InjectedFailure);
        }
        journal.phase = TransactionPhase::ConfirmationMoved;
        self.write_journal(&journal)?;
        remove_owned_tree(&cleanup)?;
        if failure_point == Some(FailurePoint::CrashAfterConfirmationCleanup) {
            return Err(InstallError::InjectedFailure);
        }
        remove_file_if_present(&receipt.journal_path)
    }

    fn recover_promotion(&self, journal: &TransactionJournal) -> Result<(), InstallError> {
        self.validate_owned_path(&journal.staging_dir, "staging")?;
        if journal.auxiliary_dir.is_some() {
            return Err(InstallError::InvalidJournal);
        }
        let rollback = self.rollback_dir()?;
        match journal.phase {
            TransactionPhase::PromotionLiveMoveIntent => {
                match (
                    self.installation_dir.exists(),
                    rollback.exists(),
                    journal.staging_dir.exists(),
                ) {
                    (true, false, true) => {},
                    (false, true, true) => {
                        fs::rename(&rollback, &self.installation_dir).map_err(InstallError::Io)?;
                    },
                    (true, false, false) => {},
                    _ => return Err(InstallError::InvalidJournal),
                }
                self.cleanup_owned(&journal.staging_dir, "staging")?;
                remove_file_if_present(&self.journal_path()?)
            },
            TransactionPhase::PromotionLiveMoved | TransactionPhase::PromotionNewLiveMoveIntent => {
                self.cancel_uncommitted_promotion(journal, &rollback)
            },
            TransactionPhase::Promoted => {
                if !self.installation_dir.exists()
                    || journal.staging_dir.exists()
                    || (journal.had_previous_installation && !rollback.exists())
                {
                    return Err(InstallError::InvalidJournal);
                }
                Ok(())
            },
            _ => Err(InstallError::InvalidJournal),
        }
    }

    fn cancel_uncommitted_promotion(
        &self,
        journal: &TransactionJournal,
        rollback: &Path,
    ) -> Result<(), InstallError> {
        let live = self.installation_dir.exists();
        let staging = journal.staging_dir.exists();
        let old = rollback.exists();
        if journal.had_previous_installation {
            match (live, old, staging) {
                (false, true, true) => {
                    fs::rename(rollback, &self.installation_dir).map_err(InstallError::Io)?;
                },
                (true, true, false) => {
                    fs::rename(&self.installation_dir, &journal.staging_dir)
                        .map_err(InstallError::Io)?;
                    fs::rename(rollback, &self.installation_dir).map_err(InstallError::Io)?;
                },
                (true, false, true) | (true, false, false) => {},
                _ => return Err(InstallError::InvalidJournal),
            }
        } else {
            match (live, old, staging) {
                (false, false, true) => {},
                (true, false, false) => {
                    fs::rename(&self.installation_dir, &journal.staging_dir)
                        .map_err(InstallError::Io)?;
                },
                (false, false, false) => {},
                _ => return Err(InstallError::InvalidJournal),
            }
        }
        self.cleanup_owned(&journal.staging_dir, "staging")?;
        remove_file_if_present(&self.journal_path()?)
    }

    fn recover_rollback(&self, journal: &TransactionJournal) -> Result<(), InstallError> {
        self.validate_owned_path(&journal.staging_dir, "staging")?;
        let displaced = journal
            .auxiliary_dir
            .as_deref()
            .ok_or(InstallError::InvalidJournal)?;
        self.validate_owned_path(displaced, "staging")?;
        match journal.phase {
            TransactionPhase::RollbackPrepared
            | TransactionPhase::RollbackLiveMoveIntent
            | TransactionPhase::RollbackLiveMoved
            | TransactionPhase::RollbackCandidateMoveIntent => {
                self.cancel_uncommitted_rollback(journal, displaced)
            },
            TransactionPhase::RollbackApplied => self.finish_applied_rollback(journal),
            _ => Err(InstallError::InvalidJournal),
        }
    }

    fn cancel_uncommitted_rollback(
        &self,
        journal: &TransactionJournal,
        displaced: &Path,
    ) -> Result<(), InstallError> {
        let live = self.installation_dir.exists();
        let candidate = journal.staging_dir.exists();
        let displaced_exists = displaced.exists();
        match (live, candidate, displaced_exists) {
            (true, true, false) => {},
            (false, true, true) => {
                fs::rename(displaced, &self.installation_dir).map_err(InstallError::Io)?;
            },
            (true, false, true) => {
                fs::rename(&self.installation_dir, &journal.staging_dir)
                    .map_err(InstallError::Io)?;
                fs::rename(displaced, &self.installation_dir).map_err(InstallError::Io)?;
            },
            (true, false, false) => {},
            _ => return Err(InstallError::InvalidJournal),
        }
        self.cleanup_owned(&journal.staging_dir, "staging")?;
        if displaced.exists() {
            self.cleanup_owned(displaced, "staging")?;
        }
        self.write_promoted_journal(journal.transaction_id)
    }

    fn finish_applied_rollback(&self, journal: &TransactionJournal) -> Result<(), InstallError> {
        if !self.installation_dir.exists() || journal.staging_dir.exists() {
            return Err(InstallError::InvalidJournal);
        }
        if let Some(displaced) = journal.auxiliary_dir.as_deref()
            && displaced.exists()
        {
            self.cleanup_owned(displaced, "staging")?;
        }
        let rollback = self.rollback_dir()?;
        if rollback.exists() {
            remove_owned_tree(&rollback)?;
        }
        remove_file_if_present(&self.journal_path()?)
    }

    fn recover_confirmation(&self, journal: &TransactionJournal) -> Result<(), InstallError> {
        if !matches!(
            journal.phase,
            TransactionPhase::ConfirmationMoveIntent | TransactionPhase::ConfirmationMoved
        ) {
            return Err(InstallError::InvalidJournal);
        }
        self.validate_owned_path(&journal.staging_dir, "cleanup")?;
        if journal.auxiliary_dir.is_some() || !self.installation_dir.exists() {
            return Err(InstallError::InvalidJournal);
        }
        let rollback = self.rollback_dir()?;
        match (rollback.exists(), journal.staging_dir.exists()) {
            (true, false) => {
                fs::rename(&rollback, &journal.staging_dir).map_err(InstallError::Io)?;
            },
            (false, true) | (false, false) => {},
            (true, true) => return Err(InstallError::InvalidJournal),
        }
        if journal.staging_dir.exists() {
            self.cleanup_owned(&journal.staging_dir, "cleanup")?;
        }
        remove_file_if_present(&self.journal_path()?)
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
        let journal = self.read_journal()?.ok_or(InstallError::InvalidJournal)?;
        if journal.transaction_id != receipt.transaction_id
            || journal.operation != TransactionOperation::Promotion
            || journal.phase != TransactionPhase::Promoted
        {
            return Err(InstallError::InvalidJournal);
        }
        Ok(())
    }

    fn write_promoted_journal(&self, transaction_id: Uuid) -> Result<(), InstallError> {
        self.write_journal(&TransactionJournal {
            transaction_id,
            operation: TransactionOperation::Promotion,
            phase: TransactionPhase::Promoted,
            staging_dir: self.owned_sibling("staging", transaction_id)?,
            auxiliary_dir: None,
            had_previous_installation: true,
        })
    }

    fn receipt(&self, transaction_id: Uuid) -> Result<PromotionReceipt, InstallError> {
        Ok(PromotionReceipt {
            installation_dir: self.installation_dir.clone(),
            rollback_dir: self.rollback_dir()?,
            journal_path: self.journal_path()?,
            transaction_id,
        })
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

    fn read_journal(&self) -> Result<Option<TransactionJournal>, InstallError> {
        match fs::read(self.journal_path()?) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map(Some)
                .map_err(|_| InstallError::InvalidJournal),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(InstallError::Io(error)),
        }
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

    fn cleanup_owned(&self, path: &Path, kind: &str) -> Result<(), InstallError> {
        self.validate_owned_path(path, kind)?;
        if path.exists() {
            remove_owned_tree(path)?;
        }
        Ok(())
    }

    fn validate_owned_path(&self, path: &Path, kind: &str) -> Result<(), InstallError> {
        if path.parent() != Some(self.parent()?) {
            return Err(InstallError::UnsafePath(
                "transaction path is not an installation sibling".to_owned(),
            ));
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| InstallError::UnsafePath("non-Unicode transaction path".to_owned()))?;
        let prefix = format!("{}.{}-", self.install_name()?, kind);
        let suffix = name.strip_prefix(&prefix).ok_or_else(|| {
            InstallError::UnsafePath("transaction path is not launcher-owned".to_owned())
        })?;
        Uuid::parse_str(suffix)
            .map_err(|_| InstallError::UnsafePath("invalid transaction identifier".to_owned()))?;
        Ok(())
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

fn ensure_preserved_destinations_absent(destination: &Path) -> Result<(), InstallError> {
    for preserved in PRESERVED_PATHS {
        match fs::symlink_metadata(destination.join(preserved)) {
            Ok(_) => {
                return Err(InstallError::UnsafePath(
                    "staging attempted to provide a preserved path".to_owned(),
                ));
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
            Err(error) => return Err(InstallError::Io(error)),
        }
    }
    Ok(())
}

fn copy_preserved(source: &Path, destination: &Path) -> Result<(), InstallError> {
    ensure_preserved_destinations_absent(destination)?;
    for preserved in PRESERVED_PATHS {
        let source_root = source.join(preserved);
        if !source_root.exists() {
            continue;
        }
        validate_real_tree(&source_root)?;
        copy_tree(&source_root, &destination.join(preserved))?;
    }
    Ok(())
}

fn copy_tree_excluding_preserved(source: &Path, destination: &Path) -> Result<(), InstallError> {
    validate_real_tree(source)?;
    ensure_preserved_destinations_absent(destination)?;
    for entry in WalkDir::new(source)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry
                .path()
                .strip_prefix(source)
                .ok()
                .and_then(|path| path.components().next())
                .and_then(|component| component.as_os_str().to_str())
                .is_none_or(|root| !PRESERVED_PATHS.contains(&root.to_ascii_lowercase().as_str()))
        })
    {
        let entry = entry.map_err(|error| {
            InstallError::UnsafePath(format!("rollback source traversal failed: {error}"))
        })?;
        copy_entry(source, destination, &entry)?;
    }
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), InstallError> {
    fs::create_dir(destination).map_err(InstallError::Io)?;
    for entry in WalkDir::new(source).min_depth(1).follow_links(false) {
        let entry = entry.map_err(|error| {
            InstallError::UnsafePath(format!("tree copy traversal failed: {error}"))
        })?;
        copy_entry(source, destination, &entry)?;
    }
    Ok(())
}

fn copy_entry(
    source: &Path,
    destination: &Path,
    entry: &walkdir::DirEntry,
) -> Result<(), InstallError> {
    if entry.file_type().is_symlink()
        || is_link_or_reparse(entry.path()).map_err(InstallError::Io)?
    {
        return Err(InstallError::UnsafePath(
            "copy source contains a link or reparse point".to_owned(),
        ));
    }
    let relative = entry
        .path()
        .strip_prefix(source)
        .map_err(|_| InstallError::UnsafePath("copy source escaped its root".to_owned()))?;
    let target = destination.join(relative);
    if entry.file_type().is_dir() {
        fs::create_dir(&target).map_err(InstallError::Io)?;
    } else if entry.file_type().is_file() {
        if target.exists() {
            return Err(InstallError::UnsafePath(
                "transaction copy would overwrite a file".to_owned(),
            ));
        }
        fs::copy(entry.path(), target).map_err(InstallError::Io)?;
    } else {
        return Err(InstallError::UnsafePath(
            "copy source contains a special entry".to_owned(),
        ));
    }
    Ok(())
}

fn validate_real_tree(root: &Path) -> Result<(), InstallError> {
    let metadata = fs::symlink_metadata(root).map_err(InstallError::Io)?;
    if !metadata.is_dir() || is_link_or_reparse(root).map_err(InstallError::Io)? {
        return Err(InstallError::UnsafePath(
            "installation tree root is unsafe".to_owned(),
        ));
    }
    for entry in WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|error| {
            InstallError::UnsafePath(format!("installation traversal failed: {error}"))
        })?;
        if entry.file_type().is_symlink()
            || is_link_or_reparse(entry.path()).map_err(InstallError::Io)?
        {
            return Err(InstallError::UnsafePath(
                "installation contains a link or reparse point".to_owned(),
            ));
        }
    }
    Ok(())
}

fn remove_owned_tree(path: &Path) -> Result<(), InstallError> {
    validate_real_tree(path)?;
    fs::remove_dir_all(path).map_err(InstallError::Io)
}

fn remove_file_if_present(path: &Path) -> Result<(), InstallError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(InstallError::Io(error)),
    }
}
