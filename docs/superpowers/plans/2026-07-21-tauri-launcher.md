# Voxtera Tauri Launcher Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Python/Tkinter launcher with a performant Tauri 2 Windows launcher that safely installs, updates, repairs, rolls back, and launches Voxtera Preview builds.

**Architecture:** A React/TypeScript frontend renders a strict launcher state machine while a standalone Rust crate under `launcher/src-tauri` owns release selection, streaming downloads, integrity, staging, rollback, configuration, and process launch. Typed Tauri commands and throttled events are the only bridge between UI and privileged operations; release artifacts follow the exact contract in the approved design.

**Tech Stack:** Tauri 2.11, Rust 2024, React 19.2, TypeScript 7, Vite 8, Vitest 4, Testing Library 16, reqwest 0.13, Tokio 1.53, serde 1, sha2 0.11, zip 8.6, semver 1.0, tracing 0.1, pnpm 11.

## Global Constraints

- Target only Windows 10/11 x64 for the first release.
- Keep the newest non-draft GitHub release, including pre-releases, as the single `Preview` channel.
- Exact release assets are `VoxteraLauncher-setup.exe`, `Voxtera-windows-x64.zip`, and `voxtera-manifest.json`.
- Never buffer the game archive fully in memory; the download buffer stays below 32 MB.
- Emit visible progress no faster than 4 Hz.
- Validate archive size and SHA-256 before extraction.
- Reject absolute, traversal, and link archive entries.
- Install on the same volume through staging and atomic rename; preserve one rollback until first successful launch.
- Never overwrite or remove `userdata/`, `screenshots/`, or user settings.
- Store config, logs, and cache under `%LOCALAPPDATA%\Voxtera`.
- Offline release failure must not block launching an already valid build.
- Do not add accounts, store, chat, news feeds, launcher self-update, delta patches, or non-Windows packages.
- Preserve the legacy Python launcher until the Tauri launcher passes the live Preview acceptance flow.
- Do not claim Windows code signing unless a real signing identity was used.

---

### Task 1: Standalone Tauri Shell and Shared Brand Foundation

**Files:**
- Modify: `Cargo.toml`
- Modify: `.gitignore`
- Create: `brand/tokens.json`
- Create: `brand/README.md`
- Create: `launcher/package.json`
- Create: `launcher/pnpm-lock.yaml`
- Create: `launcher/tsconfig.json`
- Create: `launcher/vite.config.ts`
- Create: `launcher/index.html`
- Create: `launcher/src/main.tsx`
- Create: `launcher/src/App.tsx`
- Create: `launcher/src/styles/tokens.css`
- Create: `launcher/src/styles/global.css`
- Create: `launcher/src/App.test.tsx`
- Create: `launcher/src/test/setup.ts`
- Create: `launcher/src-tauri/Cargo.toml`
- Create: `launcher/src-tauri/build.rs`
- Create: `launcher/src-tauri/tauri.conf.json`
- Create: `launcher/src-tauri/capabilities/default.json`
- Create: `launcher/src-tauri/src/main.rs`
- Create: `launcher/src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: existing `launcher/voxtera_logo.png` as the legacy source artwork.
- Produces: `launcher_core::run()`, the `pnpm test`, `pnpm build`, `pnpm tauri build --debug` commands, and shared brand values in `brand/tokens.json`.

- [ ] **Step 1: Write the failing frontend shell test**

```tsx
// launcher/src/App.test.tsx
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { App } from './App';

describe('App', () => {
  it('renders the Voxtera launcher shell in Portuguese', () => {
    render(<App />);
    expect(screen.getByRole('heading', { name: 'Voxtera' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Verificar instalação' })).toBeEnabled();
  });
});
```

- [ ] **Step 2: Run the frontend test to verify RED**

Run:

```powershell
Set-Location launcher
pnpm test -- --run src/App.test.tsx
```

Expected: FAIL because `package.json`, `App.tsx`, and the test environment do not exist.

- [ ] **Step 3: Scaffold the pinned frontend and test configuration**

Create `launcher/package.json` with exact scripts and dependency floors:

```json
{
  "name": "voxtera-launcher",
  "private": true,
  "version": "0.3.0-preview.1",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "test": "vitest",
    "lint": "eslint src --max-warnings=0",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "2.11.1",
    "lucide-react": "1.25.0",
    "react": "19.2.8",
    "react-dom": "19.2.8"
  },
  "devDependencies": {
    "@tauri-apps/cli": "2.11.4",
    "@testing-library/jest-dom": "6.9.1",
    "@testing-library/react": "16.3.2",
    "@types/react": "19.2.14",
    "@types/react-dom": "19.2.3",
    "@vitejs/plugin-react": "6.0.3",
    "eslint": "10.7.0",
    "typescript": "7.0.2",
    "vite": "8.1.5",
    "vitest": "4.1.10"
  },
  "packageManager": "pnpm@11.9.0"
}
```

Run `pnpm install --frozen-lockfile=false`, then commit the generated lockfile. Configure Vitest with `jsdom`, `src/test/setup.ts`, and React JSX.

- [ ] **Step 4: Add the minimal shell and shared brand tokens**

`brand/tokens.json` defines canonical values:

```json
{
  "color": {
    "void": "#071014",
    "stone": "#111B20",
    "stoneRaised": "#19262C",
    "ice": "#8BE8FF",
    "iceStrong": "#35CFF4",
    "gold": "#D9A94A",
    "text": "#F2F8FA",
    "muted": "#98A9B0",
    "danger": "#E66A62"
  },
  "radius": { "small": 6, "medium": 10 },
  "motion": { "fastMs": 140, "normalMs": 220 }
}
```

Create a minimal `App` with the tested heading and button. Generate `launcher/src/styles/tokens.css` from the same values manually in this task and add a test that parses both files and rejects drift.

- [ ] **Step 5: Create the standalone Rust crate and Tauri security baseline**

Add `exclude = ["launcher/src-tauri"]` to the root `[workspace]`. Create a standalone `launcher/src-tauri/Cargo.toml` with:

```toml
[package]
name = "voxtera-launcher"
version = "0.3.0-preview.1"
edition = "2024"
license = "GPL-3.0-or-later"
default-run = "voxtera-launcher"

[lib]
name = "launcher_core"
crate-type = ["lib", "cdylib", "staticlib"]

[dependencies]
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.151"
tauri = { version = "2.11.5", features = [] }
thiserror = "2.0.19"

[build-dependencies]
tauri-build = "2.6.3"
```

The default capability grants only core window/event access; it grants no shell, arbitrary filesystem, HTTP, or process plugin permissions. Configure NSIS Windows x64 output and product name `Voxtera Launcher`.

- [ ] **Step 6: Verify shell GREEN and standalone builds**

Run:

```powershell
Set-Location launcher
pnpm test -- --run
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml
pnpm tauri build --debug --no-bundle
```

Expected: frontend tests PASS, Vite build succeeds, Rust tests PASS, and the debug Tauri executable builds without compiling the main Veloren workspace.

- [ ] **Step 7: Commit the foundation**

```powershell
git add Cargo.toml .gitignore brand launcher/package.json launcher/pnpm-lock.yaml launcher/tsconfig.json launcher/vite.config.ts launcher/index.html launcher/src launcher/src-tauri
git commit -m "feat: scaffold Voxtera Tauri launcher"
```

---

### Task 2: Domain Types, Paths, Configuration, and Legacy Migration

**Files:**
- Create: `launcher/src-tauri/src/domain.rs`
- Create: `launcher/src-tauri/src/error.rs`
- Create: `launcher/src-tauri/src/paths.rs`
- Create: `launcher/src-tauri/src/config.rs`
- Create: `launcher/src-tauri/tests/config_tests.rs`
- Modify: `launcher/src-tauri/src/lib.rs`
- Modify: `launcher/src-tauri/Cargo.toml`

**Interfaces:**
- Consumes: `%LOCALAPPDATA%` and optional legacy `launcher/voxtera_config.json` data.
- Produces: `LauncherPaths::discover()`, `LauncherConfig::load_or_migrate(&LauncherPaths)`, `LauncherConfig::save_atomic`, `Channel::Preview`, `InstalledBuild`, and serializable `LauncherError`.

- [ ] **Step 1: Write failing config and path tests**

```rust
#[test]
fn migrates_legacy_config_without_deleting_it() {
    let temp = tempfile::tempdir().unwrap();
    let paths = fixture_paths(temp.path());
    std::fs::write(&paths.legacy_config, r#"{"install_dir":"D:\\Games\\Voxtera","installed_version":"v0.2.3"}"#).unwrap();
    let config = LauncherConfig::load_or_migrate(&paths).unwrap();
    assert_eq!(config.install_dir, PathBuf::from(r"D:\Games\Voxtera"));
    assert_eq!(config.installed_version.as_deref(), Some("v0.2.3"));
    assert!(paths.legacy_config.exists());
    assert!(paths.config_file.exists());
}

#[test]
fn invalid_install_path_cannot_escape_local_data_by_default() {
    let config = LauncherConfig::default_for(&fixture_paths(Path::new(r"C:\Local")));
    assert!(config.install_dir.ends_with(r"Voxtera\game"));
}
```

- [ ] **Step 2: Run config tests to verify RED**

Run `cargo test --manifest-path launcher/src-tauri/Cargo.toml --test config_tests`.

Expected: FAIL because the domain, path, and config modules do not exist.

- [ ] **Step 3: Implement typed domain and error models**

Define exact public types:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Channel { Preview }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LauncherConfig {
    pub schema_version: u32,
    pub install_dir: PathBuf,
    pub installed_version: Option<String>,
    pub bandwidth_limit_kib: Option<u64>,
    pub start_minimized: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstalledBuild {
    pub version: String,
    pub manifest_sha256: String,
    pub first_launch_confirmed: bool,
}
```

`LauncherError` uses stable machine codes such as `config_invalid`, `permission_denied`, `disk_space`, `network`, `integrity`, `unsafe_archive`, `game_running`, and `launch_failed`, plus a safe Portuguese message.

- [ ] **Step 4: Implement path discovery and atomic configuration**

`LauncherPaths` contains `root`, `config_file`, `logs_dir`, `cache_dir`, `default_install_dir`, and `legacy_config`. `save_atomic` writes JSON to `launcher.json.tmp`, flushes it, then replaces `launcher.json`. Migration accepts only the two legacy keys, normalizes the version, and writes schema version 1.

- [ ] **Step 5: Verify config behavior and formatting**

Run:

```powershell
cargo test --manifest-path launcher/src-tauri/Cargo.toml --test config_tests
cargo fmt --manifest-path launcher/src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path launcher/src-tauri/Cargo.toml --all-targets -- -D warnings
```

Expected: all tests PASS and no formatting or lint findings.

- [ ] **Step 6: Commit configuration**

```powershell
git add launcher/src-tauri
git commit -m "feat: add launcher configuration and migration"
```

---

### Task 3: Preview Release Contract and Manifest Generator

**Files:**
- Create: `launcher/src-tauri/src/manifest.rs`
- Create: `launcher/src-tauri/src/release.rs`
- Create: `launcher/src-tauri/src/bin/voxtera-manifest.rs`
- Create: `launcher/src-tauri/tests/fixtures/releases.json`
- Create: `launcher/src-tauri/tests/release_tests.rs`
- Create: `launcher/src-tauri/tests/manifest_tests.rs`
- Modify: `launcher/src-tauri/src/lib.rs`
- Modify: `launcher/src-tauri/Cargo.toml`

**Interfaces:**
- Consumes: GitHub release JSON and a built `dist/` directory.
- Produces: `ReleaseClient::latest_preview() -> Result<PreviewRelease>`, `ReleaseAsset { name, size, url }`, `Manifest::validate()`, `select_release(&[GitHubRelease])`, and CLI `voxtera-manifest --input dist --archive Voxtera-windows-x64.zip --version vX.Y.Z --output voxtera-manifest.json`.

- [ ] **Step 1: Write failing release-selection tests**

```rust
#[test]
fn selects_newest_non_draft_preview_with_exact_assets() {
    let releases = load_fixture("fixtures/releases.json");
    let selected = select_release(&releases).unwrap();
    assert_eq!(selected.version.to_string(), "0.2.3");
    assert!(selected.game_archive.url.ends_with("Voxtera-windows-x64.zip"));
    assert!(selected.manifest.url.ends_with("voxtera-manifest.json"));
}

#[test]
fn rejects_duplicate_launcher_assets() {
    let releases = fixture_with_duplicate("VoxteraLauncher-setup.exe");
    assert_eq!(select_release(&releases).unwrap_err().code(), "release_contract");
}
```

- [ ] **Step 2: Write failing manifest safety tests**

Test schema version 1, semantic version, lowercase 64-character hashes, exact archive name, relative normalized file paths, duplicate paths, traversal, absolute Windows paths, preserved rules, and minimum launcher version.

- [ ] **Step 3: Run release and manifest tests to verify RED**

Run `cargo test --manifest-path launcher/src-tauri/Cargo.toml --test release_tests --test manifest_tests`.

Expected: FAIL because the release and manifest modules do not exist.

- [ ] **Step 4: Implement the release client and validation**

Use one `reqwest::Client` with HTTPS-only redirects, a `VoxteraLauncher/<version>` user agent, 10-second connect timeout, and 30-second request timeout. Request `https://api.github.com/repos/Stoltemberg/voxtera/releases`, ignore drafts, retain pre-releases, parse semver after stripping one `v`, sort descending, and require exactly one of every asset.

Add exact dependencies `reqwest = { version = "0.13.4", default-features = false, features = ["json", "rustls", "stream"] }`, `semver = { version = "1.0.28", features = ["serde"] }`, `sha2 = "0.11.0"`, `hex = "0.4.3"`, `clap = { version = "4.6.4", features = ["derive"] }`, and `walkdir = "2.5.0"`.

- [ ] **Step 5: Implement the deterministic manifest CLI**

Walk `dist/` in normalized lexical order, exclude cache, logs, launcher, `userdata`, `screenshots`, and temporary files, stream SHA-256 for every managed file, and serialize pretty JSON with a trailing newline. The CLI refuses symlinks and paths outside the input root. Add a golden fixture proving two identical trees produce byte-identical manifests.

- [ ] **Step 6: Verify contract tests and CLI output**

Run:

```powershell
cargo test --manifest-path launcher/src-tauri/Cargo.toml --test release_tests --test manifest_tests
cargo run --manifest-path launcher/src-tauri/Cargo.toml --bin voxtera-manifest -- --help
```

Expected: all tests PASS and help lists all five required arguments.

- [ ] **Step 7: Commit the release contract**

```powershell
git add launcher/src-tauri
git commit -m "feat: define Voxtera Preview release contract"
```

---

### Task 4: Resumable Streaming Download and Progress Throttling

**Files:**
- Create: `launcher/src-tauri/src/download.rs`
- Create: `launcher/src-tauri/src/progress.rs`
- Create: `launcher/src-tauri/tests/download_tests.rs`
- Create: `launcher/src-tauri/tests/support/mod.rs`
- Create: `launcher/src-tauri/tests/support/http_server.rs`
- Modify: `launcher/src-tauri/src/lib.rs`
- Modify: `launcher/src-tauri/Cargo.toml`

**Interfaces:**
- Consumes: `DownloadRequest { asset: ReleaseAsset, expected_sha256, cache_path, bandwidth_limit_kib }` and a cancellation token.
- Produces: `DownloadManager::download(request, sink) -> DownloadOutcome`, `DownloadProgress`, `.part` file, and `.part.json` resume metadata.

- [ ] **Step 1: Write failing local HTTP integration tests**

Cover fresh download, valid `206` resume with matching ETag, ignored range returning `200`, changed ETag, cancellation, server disconnect, empty/unknown `Content-Length`, and stale cache metadata.

```rust
#[tokio::test]
async fn resumes_matching_partial_download() {
    let server = RangeServer::new(TEST_BYTES, "etag-v1").await;
    let fixture = DownloadFixture::with_partial(&TEST_BYTES[..4096], "etag-v1");
    let result = fixture.manager.download(fixture.request(server.url()), fixture.sink()).await.unwrap();
    assert_eq!(result.bytes_written, TEST_BYTES.len() as u64);
    assert_eq!(server.last_range(), Some("bytes=4096-".into()));
    assert_eq!(std::fs::read(fixture.part_path()).unwrap(), TEST_BYTES);
}
```

- [ ] **Step 2: Run download tests to verify RED**

Run `cargo test --manifest-path launcher/src-tauri/Cargo.toml --test download_tests`.

Expected: FAIL because the download manager and test server do not exist.

- [ ] **Step 3: Implement streaming and resume decisions**

Use `reqwest` byte streams and an 8 MiB reusable buffer ceiling. Persist resume metadata only after headers validate. Append only on `206` plus matching ETag and expected range; otherwise truncate and restart. Flush the file and metadata before returning success.

Add exact dependencies `tokio = { version = "1.53.1", features = ["fs", "io-util", "macros", "rt-multi-thread", "sync", "time"] }` and `futures-util = "0.3.33"`. Add `axum = "0.8.9"` under `[dev-dependencies]` for the local HTTP fixture.

- [ ] **Step 4: Implement progress throttling and bandwidth limiting**

`ProgressThrottle` emits immediately, no faster than every 250 ms, and once at completion. Speed uses a rolling window; ETA is absent until enough samples exist. The bandwidth limiter sleeps based on bytes and elapsed time without blocking cancellation.

- [ ] **Step 5: Verify focused and Rust-wide tests**

Run:

```powershell
cargo test --manifest-path launcher/src-tauri/Cargo.toml --test download_tests
cargo test --manifest-path launcher/src-tauri/Cargo.toml
```

Expected: range, cancellation, and throttling tests PASS; no test contacts GitHub.

- [ ] **Step 6: Commit the downloader**

```powershell
git add launcher/src-tauri
git commit -m "feat: add resumable launcher downloads"
```

---

### Task 5: Integrity, Safe Staging, Atomic Promotion, Rollback, and Repair

**Files:**
- Create: `launcher/src-tauri/src/integrity.rs`
- Create: `launcher/src-tauri/src/archive.rs`
- Create: `launcher/src-tauri/src/install.rs`
- Create: `launcher/src-tauri/src/repair.rs`
- Create: `launcher/src-tauri/tests/integrity_tests.rs`
- Create: `launcher/src-tauri/tests/install_tests.rs`
- Create: `launcher/src-tauri/tests/repair_tests.rs`
- Modify: `launcher/src-tauri/src/lib.rs`
- Modify: `launcher/src-tauri/Cargo.toml`

**Interfaces:**
- Consumes: validated `Manifest`, downloaded archive, installation root, and preserved rules.
- Produces: `verify_file`, `extract_to_staging`, `InstallManager::promote`, `InstallManager::rollback`, `RepairPlan`, and `PromotionReceipt`.

- [ ] **Step 1: Write failing integrity and archive attack tests**

Create ZIP fixtures for `../escape`, `C:\absolute`, `/absolute`, symlink attributes, duplicate normalized paths, decompression ceiling overflow, wrong size, and wrong SHA-256. Assert no file appears outside the temporary staging root.

- [ ] **Step 2: Write failing install and rollback tests**

```rust
#[test]
fn failed_promotion_restores_previous_installation_and_player_data() {
    let fixture = InstallFixture::valid_old_and_staged_new();
    fixture.inject_failure(FailurePoint::AfterLiveMoved);
    assert!(fixture.manager.promote(fixture.request()).is_err());
    assert_eq!(fixture.read_live("Voxtera.exe"), b"old-game");
    assert_eq!(fixture.read_live("userdata/save.ron"), b"player-save");
}
```

Cover same-volume staging, locked executable, rollback retention, first-launch confirmation, and cleanup of only launcher-owned staging paths.

- [ ] **Step 3: Write failing repair-plan tests**

Prove missing/changed managed files are listed, valid files are skipped, preserved files are ignored, and repair never proposes deletion outside managed paths.

- [ ] **Step 4: Run all new tests to verify RED**

Run `cargo test --manifest-path launcher/src-tauri/Cargo.toml --test integrity_tests --test install_tests --test repair_tests`.

Expected: FAIL because integrity, archive, install, and repair modules do not exist.

- [ ] **Step 5: Implement streaming hashes and safe archive extraction**

Use `zip = { version = "8.6.0", default-features = false, features = ["deflate"] }`. Normalize every entry with Windows semantics before joining. Reject links and unknown special entries. Track cumulative uncompressed bytes and validate every extracted managed file against the manifest.

Add `uuid = { version = "1.24.0", features = ["v4"] }` under `[dependencies]` and `tempfile = "3.27.0"` under `[dev-dependencies]`; production temporary paths come from `LauncherPaths`.

- [ ] **Step 6: Implement promotion and rollback transaction**

Use sibling paths `<install>.staging-<uuid>` and `<install>.rollback`. Write a transaction journal before the first rename and update it after each phase. On startup, recover an incomplete journal deterministically. Copy preserved paths only through a fixed allowlist and refuse reparse points.

- [ ] **Step 7: Implement repair planning**

Hash managed files in bounded worker concurrency, return totals and invalid paths, and require explicit confirmation before downloading the full archive. Repair feeds the normal validated staging and promotion transaction rather than editing live files in place.

- [ ] **Step 8: Verify safety suite**

Run:

```powershell
cargo test --manifest-path launcher/src-tauri/Cargo.toml --test integrity_tests --test install_tests --test repair_tests
cargo clippy --manifest-path launcher/src-tauri/Cargo.toml --all-targets -- -D warnings
```

Expected: all attack, rollback, preservation, and repair tests PASS.

- [ ] **Step 9: Commit safe installation**

```powershell
git add launcher/src-tauri
git commit -m "feat: add atomic Voxtera install and repair"
```

---

### Task 6: Game Process, Launcher Service, Tauri Commands, and Capabilities

**Files:**
- Create: `launcher/src-tauri/src/game.rs`
- Create: `launcher/src-tauri/src/service.rs`
- Create: `launcher/src-tauri/src/commands.rs`
- Create: `launcher/src-tauri/src/events.rs`
- Create: `launcher/src-tauri/tests/game_tests.rs`
- Create: `launcher/src-tauri/tests/service_tests.rs`
- Modify: `launcher/src-tauri/src/lib.rs`
- Modify: `launcher/src-tauri/src/main.rs`
- Modify: `launcher/src-tauri/capabilities/default.json`

**Interfaces:**
- Consumes: services from Tasks 2-5.
- Produces: `LauncherSnapshot { phase, installed_version, available_version, install_dir, local_build_valid, operation, last_error }`; Tauri commands `get_snapshot`, `check_release`, `install_or_update`, `cancel_operation`, `scan_repair`, `repair`, `choose_install_dir`, `save_settings`, `launch_game`, and `open_logs`; event `launcher://progress`.

- [ ] **Step 1: Write failing process-boundary tests**

Use an injected `ProcessRunner`. Prove only `<install_dir>\Voxtera.exe` is launched, the working directory is exact, missing or replaced executable fails, duplicate launch is blocked, and first successful spawn confirms the pending rollback.

- [ ] **Step 2: Write failing service-state tests**

Test offline local-ready behavior, incompatible launcher version, busy-operation rejection, cancellation, update failure preserving installed version, recovery journal on startup, and stable Portuguese error mapping.

- [ ] **Step 3: Run focused tests to verify RED**

Run `cargo test --manifest-path launcher/src-tauri/Cargo.toml --test game_tests --test service_tests`.

Expected: FAIL because game, service, command, and event modules do not exist.

- [ ] **Step 4: Implement the orchestrating service**

`LauncherService` owns one operation mutex and cancellation token. `get_snapshot` is read-only. Install/update/repair all reuse download, validation, staging, and promotion. Network errors become `offline` only when a valid local executable exists; otherwise they become actionable failures.

- [ ] **Step 5: Register narrow Tauri commands**

Commands accept typed data only; paths returned by the folder picker are validated in Rust. Do not enable generic shell or filesystem plugins. `open_logs` opens only the known logs directory through an explicit command implementation.

- [ ] **Step 6: Verify commands and capabilities**

Add a static test that parses `capabilities/default.json` and rejects `shell:allow-execute`, broad filesystem scopes, and arbitrary HTTP plugin permissions.

Run:

```powershell
cargo test --manifest-path launcher/src-tauri/Cargo.toml
pnpm --dir launcher tauri build --debug --no-bundle
```

Expected: all Rust tests PASS and the Tauri binary links.

- [ ] **Step 7: Commit the application service**

```powershell
git add launcher/src-tauri
git commit -m "feat: connect launcher services to Tauri"
```

---

### Task 7: React State Machine and Complete Functional UI

**Files:**
- Create: `launcher/src/domain/launcher.ts`
- Create: `launcher/src/services/tauri.ts`
- Create: `launcher/src/state/launcherReducer.ts`
- Create: `launcher/src/state/LauncherProvider.tsx`
- Create: `launcher/src/components/LauncherFrame.tsx`
- Create: `launcher/src/components/HomeView.tsx`
- Create: `launcher/src/components/ProgressSurface.tsx`
- Create: `launcher/src/components/SettingsView.tsx`
- Create: `launcher/src/components/RepairDialog.tsx`
- Create: `launcher/src/components/ErrorNotice.tsx`
- Create: `launcher/src/components/OfflineNotice.tsx`
- Create: `launcher/src/i18n/pt-BR.ts`
- Create: `launcher/src/state/launcherReducer.test.ts`
- Create: `launcher/src/components/HomeView.test.tsx`
- Create: `launcher/src/components/SettingsView.test.tsx`
- Modify: `launcher/src/App.tsx`

**Interfaces:**
- Consumes: typed Tauri commands and `launcher://progress` from Task 6.
- Produces: exhaustive frontend `LauncherState`, reducer actions, accessible install/update/repair/play/settings workflows, and Portuguese copy inventory.

- [ ] **Step 1: Write failing reducer transition tests**

```ts
it('never enables play while promotion is active', () => {
  const state = reduce(readyState, { type: 'installStarted', operation: 'update' });
  expect(state.phase).toBe('installing');
  expect(selectCanPlay(state)).toBe(false);
});

it('keeps offline play enabled for a valid local build', () => {
  const state = reduce(readyState, { type: 'releaseFailed', error: offlineError });
  expect(state.phase).toBe('offline');
  expect(selectCanPlay(state)).toBe(true);
});
```

Cover every state and invalid event; the reducer must be exhaustive under TypeScript `never` checking.

- [ ] **Step 2: Write failing component workflow tests**

Mock only the typed `LauncherApi`. Verify initial check, install confirmation, update progress, cancel, repair size disclosure, settings validation, offline play, recoverable errors, and disabled duplicate actions.

- [ ] **Step 3: Run frontend tests to verify RED**

Run `pnpm --dir launcher test -- --run`.

Expected: FAIL because the state and component modules do not exist.

- [ ] **Step 4: Implement domain types, reducer, and provider**

Mirror Rust snapshot fields exactly. Subscribe once to progress events, remove the listener on unmount, coalesce progress by operation ID, and never infer installed state from button text.

- [ ] **Step 5: Implement functional views**

Home renders real version/release state and contextual `INSTALAR`, `ATUALIZAR`, or `JOGAR`. Settings validates install path and bandwidth integer before saving. Repair always shows scan results and full-archive download size before confirmation. Error messages expose safe retry/open-log actions only.

- [ ] **Step 6: Verify frontend behavior and accessibility semantics**

Run:

```powershell
pnpm --dir launcher test -- --run
pnpm --dir launcher lint
pnpm --dir launcher build
```

Expected: all component and reducer tests PASS, zero lint warnings, and production frontend build succeeds.

- [ ] **Step 7: Commit functional UI**

```powershell
git add launcher/src
git commit -m "feat: add Voxtera launcher workflows"
```

---

### Task 8: Cinematic Visual System, Responsive Desktop Layout, and Performance

**Files:**
- Create: `brand/voxtera-logo.png`
- Create: `brand/launcher-background.webp`
- Create: `brand/launcher-background.avif`
- Create: `launcher/src/styles/launcher.css`
- Create: `launcher/src/components/StatusGlyph.tsx`
- Create: `launcher/src/components/WindowControls.tsx`
- Create: `launcher/src/components/visual-regression.test.tsx`
- Modify: `launcher/src/styles/global.css`
- Modify: `launcher/src/components/LauncherFrame.tsx`
- Modify: `launcher/src/components/HomeView.tsx`
- Modify: `launcher/src/components/ProgressSurface.tsx`
- Modify: `launcher/src/components/SettingsView.tsx`
- Modify: `launcher/src/components/RepairDialog.tsx`
- Modify: `launcher/src/components/ErrorNotice.tsx`
- Modify: `launcher/src/components/OfflineNotice.tsx`
- Modify: `launcher/src-tauri/tauri.conf.json`

**Interfaces:**
- Consumes: approved cinematic voxel-fantasy direction and functional components from Task 7.
- Produces: final 1100x680 reference composition, shared approved brand assets, responsive minimum window behavior, reduced-motion mode, and visual QA evidence.

- [ ] **Step 1: Create and inspect the production artwork**

Use original project artwork or a new image-generation asset showing a Voxtera voxel landscape with cool ice-blue atmosphere, restrained warm light, clear dark negative space behind UI, and no text baked into the image. Save the master result under `brand/`, then create AVIF/WebP derivatives. Inspect every asset with `view_image` before use.

- [ ] **Step 2: Write failing visual contract tests**

Assert the logo/background assets exist, image dimensions meet the reference viewport, visible CTA copy matches the approved inventory, icons have accessible names, and the CSS includes `prefers-reduced-motion` plus minimum-size layout rules.

- [ ] **Step 3: Run visual contract tests to verify RED**

Run `pnpm --dir launcher test -- --run src/components/visual-regression.test.tsx`.

Expected: FAIL until final assets and visual styles exist.

- [ ] **Step 4: Implement the approved composition**

Use sharp stone/metal panels, ice-blue primary accents, restrained gold status accents, readable text contrast, and a fixed bottom progress surface. Avoid generic pill-heavy SaaS styling. Use Lucide icons only where their geometry matches; otherwise create small production SVG components with `currentColor`.

- [ ] **Step 5: Verify real launcher rendering**

Run `pnpm --dir launcher tauri dev`, capture the reference viewport and minimum supported viewport, and inspect both screenshots with `view_image`. Exercise check, offline, install/update progress, settings, repair, error, and ready states using a local fixture mode compiled only for development.

Create `.superpowers/qa/launcher-fidelity.md` with at least five comparison points: composition, typography, palette, artwork blending, spacing, state legibility, and motion. Remove all fixable mismatches before continuing.

- [ ] **Step 6: Measure performance**

Record first-visible-window time, settled CPU, working set, progress frequency, installer size, and residual processes on the target Windows machine. Save results in `launcher/PERFORMANCE.md`; any target miss includes the measured value and accepted reason.

- [ ] **Step 7: Commit visual and performance work**

```powershell
git add brand launcher/src launcher/src-tauri/tauri.conf.json launcher/PERFORMANCE.md
git commit -m "feat: finish Voxtera launcher experience"
```

---

### Task 9: Release Packaging, Real Fixture Acceptance, and Preview Publication

**Files:**
- Create: `launcher/scripts/build-preview.ps1`
- Create: `launcher/scripts/test-preview.ps1`
- Create: `launcher/RELEASING.md`
- Create: `launcher/tests/fixture-game/Cargo.toml`
- Create: `launcher/tests/fixture-game/src/main.rs`
- Create: `launcher/tests/fixtures/userdata/save.ron`
- Modify: `.gitignore`
- Modify: `launcher/src-tauri/tauri.conf.json`

**Interfaces:**
- Consumes: built `dist/`, manifest CLI, Tauri NSIS output, GitHub CLI authentication.
- Produces: exact release assets, acceptance report, and a live non-draft Preview release consumable by the website plan.

- [ ] **Step 1: Write failing packaging contract checks**

`test-preview.ps1` must fail unless an output directory contains exactly the three required asset names, the manifest validates through `voxtera-manifest`, the archive hash/size matches, and the NSIS asset is a Windows executable. It also rejects accidentally bundled `userdata`, logs, `.part`, or rollback data.

- [ ] **Step 2: Run packaging checks to verify RED**

Run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File launcher/scripts/test-preview.ps1 -Artifacts launcher/artifacts
```

Expected: FAIL because scripts and artifacts do not exist.

- [ ] **Step 3: Implement deterministic build packaging**

`build-preview.ps1` accepts `-Version`, `-GameDist`, and `-Artifacts`. It creates `Voxtera-windows-x64.zip` from managed game files in lexical order, runs the manifest CLI against that completed archive, builds the NSIS launcher, copies it as `VoxteraLauncher-setup.exe`, and invokes `test-preview.ps1`. It never publishes by itself.

- [ ] **Step 4: Execute the real small-fixture acceptance flow**

Build `launcher/tests/fixture-game` into a tiny Windows executable that writes a sentinel file and exits successfully. Serve it through a controlled local Preview API and small archive. Verify fresh install, forced disconnect/resume, checksum rejection, update, corrupted-file repair, promotion failure rollback, preserved save, offline play, and exact child process launch. Record commands and results in `launcher/ACCEPTANCE.md`.

- [ ] **Step 5: Run the complete local release gate**

Run:

```powershell
pnpm --dir launcher test -- --run
pnpm --dir launcher lint
pnpm --dir launcher build
cargo fmt --manifest-path launcher/src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path launcher/src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path launcher/src-tauri/Cargo.toml
pnpm --dir launcher tauri build --bundles nsis
powershell.exe -NoProfile -ExecutionPolicy Bypass -File launcher/scripts/build-preview.ps1 -Version v0.3.0 -GameDist dist -Artifacts launcher/artifacts
```

Expected: every command exits 0, artifacts have exact names, and `ACCEPTANCE.md` contains all PASS results.

- [ ] **Step 6: Publish the Preview release assets**

Confirm the selected tag and release notes, then run:

```powershell
gh release create v0.3.0-preview.1 `
  launcher/artifacts/VoxteraLauncher-setup.exe `
  launcher/artifacts/Voxtera-windows-x64.zip `
  launcher/artifacts/voxtera-manifest.json `
  --repo Stoltemberg/voxtera `
  --prerelease `
  --title "Voxtera v0.3.0 Preview 1" `
  --notes-file launcher/artifacts/release-notes.md
```

Expected: one non-draft pre-release exists with exactly the three required assets. If no code-signing certificate was used, state that the installer is unsigned in `release-notes.md`.

- [ ] **Step 7: Verify the live contract without installing from the internet**

Use `gh api repos/Stoltemberg/voxtera/releases` to verify tag, draft/pre-release flags, asset names, sizes, and immutable URLs. Download only the small manifest and validate it. Do not redownload the full game archive solely for this metadata check.

- [ ] **Step 8: Commit release tooling and evidence**

```powershell
git add .gitignore launcher/scripts launcher/RELEASING.md launcher/ACCEPTANCE.md launcher/tests
git commit -m "build: add Voxtera Preview release pipeline"
```

---

### Task 10: Final Launcher Review and Cutover Guard

**Files:**
- Create: `launcher/README.md`
- Modify: `launcher/voxtera_launcher.py` only to display a non-blocking migration notice after the live Tauri release exists.
- Modify: `launcher/VoxteraLauncher.spec` only if needed to retain the legacy fallback build.

**Interfaces:**
- Consumes: the verified live Preview from Task 9.
- Produces: operator/user documentation and a deliberate legacy-to-Tauri transition without deleting the fallback.

- [ ] **Step 1: Document user and operator flows**

Document installation, update, repair, rollback, offline behavior, log location, preserved data, uninstall boundaries, unsigned-installer warning when applicable, development commands, release commands, and recovery from a broken release contract.

- [ ] **Step 2: Add a migration regression test before changing legacy code**

Add a Python source-level test that the legacy launcher continues to launch and that any Tauri migration notice never blocks Play or Update. Run it and observe RED before adding the notice.

- [ ] **Step 3: Implement only the migration notice**

The legacy launcher may point users to the new installer asset but remains functional. Do not delete Python, PyInstaller spec, or the existing release asset in this task.

- [ ] **Step 4: Run final launcher verification**

Repeat Task 9's full gate from a clean checkout, run `git diff --check`, confirm no secrets or `.vercel` data are tracked, and perform a read-only whole-launcher review against the approved spec.

- [ ] **Step 5: Commit documentation and cutover guard**

```powershell
git add launcher/README.md launcher/voxtera_launcher.py launcher/VoxteraLauncher.spec launcher/tests
git commit -m "docs: complete Voxtera launcher cutover"
```
