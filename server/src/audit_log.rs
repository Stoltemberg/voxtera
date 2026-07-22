//! Audit log for Voxtera server.
//!
//! Logs admin actions, PvP kills, trades, and login/logout events to
//! `server/data_dir/audit.log` in a human-readable format.

use std::io::Write;
use std::path::Path;
use std::time::Instant;
use tracing::{info, warn};

use common::uuid::Uuid;

/// ECS resource that holds the audit log file handle and buffer.
pub struct AuditLog {
    path: std::path::PathBuf,
    buffer: Vec<String>,
    last_flush: Instant,
}

impl AuditLog {
    pub fn new(data_dir: &Path) -> Self {
        let path = data_dir.join("audit.log");
        info!(?path, "Audit log initialized");
        Self {
            path,
            buffer: Vec::new(),
            last_flush: Instant::now(),
        }
    }

    /// Log an entry with timestamp.
    pub fn log(&mut self, entry: &str) {
        let ts = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let line = format!("[{}] {}", ts, entry);
        self.buffer.push(line);

        // Flush every 5 seconds or when buffer exceeds 50 entries
        if self.last_flush.elapsed().as_secs() >= 5 || self.buffer.len() >= 50 {
            self.flush();
        }
    }

    /// Flush buffered entries to disk.
    pub fn flush(&mut self) {
        if self.buffer.is_empty() {
            return;
        }
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
        {
            Ok(mut file) => {
                for entry in self.buffer.drain(..) {
                    let _ = writeln!(file, "{}", entry);
                }
                self.last_flush = Instant::now();
            },
            Err(e) => {
                warn!(?e, "Failed to write audit log");
                self.buffer.clear();
            },
        }
    }

    // -- Helper methods for common log patterns --

    pub fn log_admin_action(&mut self, admin: &str, action: &str, target: &str) {
        self.log(&format!("ADMIN | {} | {} | target={}", admin, action, target));
    }

    pub fn log_pvp_kill(&mut self, killer: &str, victim: &str, weapon: &str) {
        self.log(&format!("PVP   | {} killed {} with {}", killer, victim, weapon));
    }

    pub fn log_trade(&mut self, player_a: &str, player_b: &str, items: &str) {
        self.log(&format!("TRADE | {} <-> {} | {}", player_a, player_b, items));
    }

    pub fn log_login(&mut self, uuid: &Uuid, alias: &str) {
        self.log(&format!("LOGIN | {} ({})", alias, uuid));
    }

    pub fn log_logout(&mut self, uuid: &Uuid, alias: &str) {
        self.log(&format!("LOGOUT| {} ({})", alias, uuid));
    }
}
