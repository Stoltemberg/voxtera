use std::{
    io,
    path::Path,
    process::{Child, Command},
    sync::Mutex,
};

use crate::{LauncherError, Manifest, manifest::GAME_EXECUTABLE, verify_file};

pub trait ManagedProcess: Send {
    fn id(&self) -> u32;
    fn try_wait(&mut self) -> io::Result<Option<i32>>;
}

pub trait ProcessRunner: Send + Sync {
    fn spawn(
        &self,
        executable: &Path,
        working_directory: &Path,
    ) -> io::Result<Box<dyn ManagedProcess>>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemProcessRunner;

impl ManagedProcess for Child {
    fn id(&self) -> u32 { Child::id(self) }

    fn try_wait(&mut self) -> io::Result<Option<i32>> {
        Child::try_wait(self).map(|status| status.and_then(|status| status.code()))
    }
}

impl ProcessRunner for SystemProcessRunner {
    fn spawn(
        &self,
        executable: &Path,
        working_directory: &Path,
    ) -> io::Result<Box<dyn ManagedProcess>> {
        Command::new(executable)
            .current_dir(working_directory)
            .spawn()
            .map(|child| Box::new(child) as Box<dyn ManagedProcess>)
    }
}

pub struct GameLauncher<R> {
    runner: R,
    process: Mutex<Option<Box<dyn ManagedProcess>>>,
}

impl<R> GameLauncher<R>
where
    R: ProcessRunner,
{
    pub fn new(runner: R) -> Self {
        Self {
            runner,
            process: Mutex::new(None),
        }
    }

    pub fn launch<F>(
        &self,
        install_dir: &Path,
        manifest: &Manifest,
        confirm_first_launch: F,
    ) -> Result<u32, LauncherError>
    where
        F: FnOnce() -> Result<(), LauncherError>,
    {
        manifest
            .validate()
            .map_err(|_| LauncherError::integrity())?;
        let executable_entry = manifest
            .files
            .iter()
            .find(|file| file.path == GAME_EXECUTABLE)
            .ok_or_else(LauncherError::integrity)?;
        let executable = install_dir.join(GAME_EXECUTABLE);
        verify_file(&executable, executable_entry.size, &executable_entry.sha256)
            .map_err(|_| LauncherError::integrity())?;

        let mut process = self
            .process
            .lock()
            .map_err(|_| LauncherError::launch_failed())?;
        if let Some(running) = process.as_mut() {
            match running.try_wait() {
                Ok(None) => return Err(LauncherError::game_running()),
                Ok(Some(_)) => *process = None,
                Err(_) => return Err(LauncherError::launch_failed()),
            }
        }

        let spawned = self
            .runner
            .spawn(&executable, install_dir)
            .map_err(|_| LauncherError::launch_failed())?;
        let pid = spawned.id();
        *process = Some(spawned);
        confirm_first_launch()?;
        Ok(pid)
    }
}
