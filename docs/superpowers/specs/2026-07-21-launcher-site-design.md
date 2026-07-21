# Voxtera Launcher and Website Design

**Date:** 2026-07-21
**Status:** Approved design
**Audience:** Voxtera maintainers and release operators

## 1. Context

Voxtera currently ships a Windows launcher implemented as a single Python/Tkinter
file and packaged with PyInstaller. It checks GitHub pre-releases, downloads the
first ZIP asset, extracts directly into the live installation, stores configuration
beside the executable, and launches `Voxtera.exe`.

The replacement must provide a substantially more polished game-facing experience,
reduce launcher overhead, make interrupted downloads recoverable, and ensure that a
failed update never destroys the last usable installation. A public website must use
the same visual identity and provide a direct download of the newest Preview launcher.

This work is split into two ordered deliverables:

1. A production-ready Windows launcher.
2. A responsive marketing and download site deployed to Vercel.

The site depends on the launcher release contract, so the launcher is completed and
published before the site's download flow is promoted to production.

## 2. Goals

- Replace Python/Tkinter with Tauri 2, React, TypeScript, and a Rust update engine.
- Support install, update, repair, installation-folder selection, and game launch.
- Use the newest non-draft GitHub Preview release as the single release channel.
- Resume interrupted downloads when the server supports HTTP range requests.
- Validate release metadata, size, and SHA-256 before installation.
- Install through a staging directory and atomically promote a validated build.
- Preserve player-owned data and retain a rollback copy until a successful launch.
- Provide clear offline, failure, progress, and recovery states.
- Keep idle CPU and memory use appropriate for a 4 GB GPU / 16 GB RAM computer.
- Deliver a cohesive fantasy-voxel identity across launcher and website.
- Publish a fast, accessible, responsive site to Vercel with a working launcher CTA.

## 3. Non-goals for the First Release

- User accounts, authentication, store, payments, friends, or chat in the launcher.
- Embedded news feeds beyond the current release title and short release notes.
- Automatic self-update of the launcher.
- Binary or block-level differential patches.
- Multiple channels such as Stable, Nightly, or custom mirrors.
- macOS or Linux launcher packages.
- Hosting large game or launcher binaries on Vercel.

The architecture keeps the update engine and UI boundaries suitable for later delta
updates and a Stable channel without implementing them now.

## 4. Technical Architecture

### 4.1 Application shell

The launcher uses Tauri 2 with a React/TypeScript frontend and Rust commands for all
filesystem, network, archive, hashing, configuration, and process operations. The
frontend cannot write arbitrary paths or invoke arbitrary programs.

The launcher is organized into bounded Rust services:

- `release_client`: fetches and validates Preview release metadata.
- `download_manager`: performs streaming, resumable downloads with cancellation.
- `integrity`: validates manifest schema, expected size, and SHA-256.
- `install_manager`: extracts safely to staging, validates, promotes, and rolls back.
- `repair`: compares managed files against the manifest and plans restoration.
- `game_process`: launches the expected executable and observes the first start.
- `config`: loads, validates, migrates, and atomically saves local preferences.
- `logging`: writes bounded diagnostic logs without secrets or personal data.

Tauri commands expose typed request and response objects. Long-running commands emit
throttled progress events rather than blocking the UI thread.

### 4.2 Frontend state model

The frontend is driven by an explicit state machine:

`checking -> install_available | update_available | ready | offline | failed`

An install or update continues through:

`downloading -> verifying -> staging -> promoting -> ready | failed`

Repair continues through:

`scanning -> repair_required | ready -> downloading -> verifying -> staging -> promoting`

Only actions valid for the current state are enabled. Closing the window during a
download requests cancellation and leaves the partial file available for resumption.

### 4.3 Local data

Launcher state is stored outside the installation directory:

- Config: `%LOCALAPPDATA%\Voxtera\launcher.json`
- Logs: `%LOCALAPPDATA%\Voxtera\logs\`
- Download cache: `%LOCALAPPDATA%\Voxtera\cache\`
- Default game directory: `%LOCALAPPDATA%\Voxtera\game\`

Writes use a temporary sibling followed by an atomic replace. Logs rotate by size and
count. The new launcher performs a one-time import of the legacy
`voxtera_config.json` when it exists beside the old launcher, but does not delete it.

The release manifest distinguishes managed game files from preserved player data.
At minimum, `userdata/`, `screenshots/`, and user settings are never overwritten or
removed by install, update, rollback, or repair.

## 5. Release Contract

The newest non-draft GitHub release, including a pre-release, is the active `Preview`
release. The launcher does not use GitHub's `releases/latest` endpoint because that
endpoint excludes pre-releases.

Each Preview release publishes exact Windows asset names:

- `VoxteraLauncher-setup.exe`
- `Voxtera-windows-x64.zip`
- `voxtera-manifest.json`

The manifest is versioned and contains:

- schema version;
- game version and channel;
- archive filename, byte size, and SHA-256;
- expected executable path;
- per-file relative path, byte size, and SHA-256 for managed files;
- explicit preserved path rules;
- minimum compatible launcher version.

Release selection rejects drafts, missing assets, duplicate exact asset names,
unsupported schemas, malformed versions, absolute paths, traversal paths, and invalid
hashes. Release responses use an application-specific user agent and bounded timeout.

The existing Python launcher remains available during the transition. It is removed
only after the Tauri installer has passed end-to-end installation and launch checks in
a real Preview release.

## 6. Download, Install, Repair, and Rollback

### 6.1 Download

- Stream the archive to a `.part` file; never buffer it fully in memory.
- Persist URL, ETag, expected size, and downloaded size in cache metadata.
- Resume with `Range` and `If-Range` when metadata still matches.
- Restart cleanly when a server ignores the range or the ETag changes.
- Reuse the HTTP client and apply connect, header, idle, and total-operation timeouts.
- Update UI progress at no more than four events per second.
- Show bytes, percentage, current speed, and estimated time remaining.

### 6.2 Validation and extraction

- Require exact archive byte size and SHA-256 before extraction.
- Reject ZIP entries that are absolute, traverse the staging root, or are links.
- Enforce an extracted-size ceiling derived from the validated manifest.
- Extract to a unique staging directory on the same volume as the live installation.
- Validate the executable and every managed file in staging before promotion.

### 6.3 Atomic promotion and rollback

- Stop before promotion if the game executable is running.
- Move the current managed installation to a rollback sibling.
- Move the validated staging installation into the live path.
- Restore preserved player data when it lives beneath the installation root.
- Launch only the expected executable from the configured installation directory.
- Keep one rollback until the new executable starts successfully.
- Restore rollback automatically if promotion or the first launch fails.

### 6.4 Repair

Repair hashes managed files and reports missing or invalid entries. In the first
release, repair may download the full game archive, but it only replaces managed
files and continues to preserve player data. The UI states this download size before
confirmation. A later file-level or block-level updater can reuse the same manifest.

## 7. Error Handling and Offline Behavior

Network failure during startup does not prevent playing an already valid local build.
The launcher presents a compact offline state and the time of the last successful
release check.

Errors are mapped to actionable categories:

- release unavailable or rate-limited;
- insufficient disk space;
- permission denied or locked files;
- interrupted download;
- size or checksum mismatch;
- unsafe or corrupt archive;
- incompatible manifest or launcher;
- game already running;
- launch failure and rollback result.

Raw stack traces remain in local logs. UI messages include a retry or recovery action
when one is safe. No failed operation silently changes the installed-version record.

## 8. Launcher Experience and Visual Direction

The launcher uses a resizable desktop window with a reference canvas near 1100 by
680 pixels. The visual language is cinematic voxel fantasy:

- real Voxtera landscape imagery as the main visual layer;
- dark stone/metal surfaces with controlled translucency;
- the existing ice-blue Voxtera identity;
- restrained warm-gold accents for highlights and status;
- strong display typography paired with a highly legible UI face;
- sharp, crafted geometry rather than generic rounded SaaS cards.

The home view includes the logo, installed and available versions, short release
notes, a dominant `JOGAR` action, contextual install/update action, repair access, and
a bottom progress surface. Settings include install directory, optional bandwidth
limit, start-minimized preference, and log access.

Motion is short and functional. Reduced-motion preferences are honored. There is no
autoplay video, continuous particle canvas, or heavy background animation.

All Portuguese UI copy is stored as structured frontend strings so an English locale
can be added later without rewriting components.

## 9. Performance Targets

Targets are measured on the user's 4 GB GPU / 16 GB RAM Windows machine:

- First visible window within 2 seconds after the WebView runtime is warm.
- Release checking is asynchronous and never delays the first usable frame.
- Idle launcher CPU below 1% after startup work settles.
- Idle working set target below 150 MB.
- Download buffer below 32 MB independent of archive size.
- UI progress updates no faster than 4 Hz.
- Launcher installer target below 25 MB, excluding an external WebView2 bootstrap.
- No background launcher process remains after the game and launcher are closed.

The site targets a mobile Lighthouse performance score of at least 90 in production,
with LCP below 2.5 seconds under a representative throttled profile.

## 10. Website

The website lives in `website/` as a standalone Next.js application deployable from
the monorepo. Shared brand tokens and approved assets live in a neutral `brand/`
directory so launcher and website use the same palette, typography decisions, and
logo sources without duplicating master assets.

The Portuguese single-page structure is:

1. Cinematic hero with Voxtera logo, concise promise, and `BAIXAR LAUNCHER` CTA.
2. World and gameplay differentiators.
3. Gallery using real Voxtera captures or original project artwork.
4. Launcher benefits: install, update, repair, and safe rollback.
5. Minimum and recommended Windows requirements.
6. Final download CTA.
7. Credits, project license, and repository attribution.

The site uses optimized AVIF/WebP assets, responsive image sizes, semantic HTML,
keyboard-visible focus, sufficient contrast, reduced motion, and no autoplay video.
It does not use stock fantasy imagery or unrelated Veloren promotional artwork.

### 10.1 Download endpoint

`/api/download/windows` queries the public GitHub releases API, selects the newest
non-draft release (including pre-releases), requires exactly one
`VoxteraLauncher-setup.exe` asset, caches the result for five minutes, and redirects
to that asset's immutable GitHub download URL.

The endpoint never proxies the binary through Vercel. It returns an explicit service
error when GitHub is unavailable or the release contract is broken. The CTA displays
the resolved version and Windows x64 platform before navigation.

## 11. Vercel Deployment

- Deploy `website/` as its own Vercel project.
- Run lint, typecheck, unit tests, and production build locally before deployment.
- Create and verify a preview deployment first.
- Test desktop and mobile layouts and the download redirect on the preview.
- Promote the exact verified preview artifact to production.
- Confirm the production URL, response status, main assets, and download route.
- Scan deployment logs for build or runtime errors after promotion.

No secret is required for public release reads. If GitHub rate limits become a real
operational problem, a scoped server-side token can be added through Vercel environment
variables without exposing it to the browser.

## 12. Testing Strategy

### 12.1 Rust unit and integration tests

- semantic version and Preview release selection;
- exact asset contract and manifest schema;
- path normalization and traversal rejection;
- resume decisions for ETag, range support, and changed release;
- streaming checksum and size validation;
- disk-space and permission failures;
- staging validation, atomic promotion, rollback, and cleanup;
- managed versus preserved paths;
- repair planning;
- legacy config migration;
- exact game process invocation.

Network and filesystem effects use injected adapters and temporary directories. A
small local HTTP test server covers range and interrupted-transfer behavior.

### 12.2 Frontend tests

- every launcher state and permitted action;
- progress event throttling and presentation;
- offline play behavior;
- install/update/repair confirmation flows;
- settings validation;
- keyboard access and reduced-motion behavior.

### 12.3 End-to-end launcher acceptance

A small controlled test fixture is published or served locally to prove the real sequence:
fresh install, interrupted resume, update, checksum rejection, repair, rollback, and
successful game-process handoff. The production game archive is not used for routine
tests.

### 12.4 Website verification

- lint, typecheck, unit tests, and production build;
- desktop and mobile browser flows;
- hero and section visual comparison against the approved concept;
- image loading and responsive crops;
- download route success, broken-contract failure, and caching;
- production smoke test after Vercel promotion.

## 13. Acceptance Criteria

The launcher is accepted when:

- it installs and launches a Preview build on Windows x64;
- an interrupted download resumes or safely restarts;
- corrupt archives are rejected before live files change;
- a failed promotion or first launch restores the previous installation;
- repair detects and restores a corrupted managed file;
- preserved player data survives install, update, repair, and rollback;
- offline users can launch an already valid installation;
- the launcher meets the measured performance targets or records an approved variance;
- automated tests and the real small-package acceptance flow pass.

The site is accepted when:

- the approved launcher visual identity is implemented faithfully;
- desktop and mobile layouts have no overflow or clipped primary content;
- production build and browser verification pass;
- `BAIXAR LAUNCHER` resolves to the exact newest Preview installer;
- the deployed Vercel production URL is reachable and shareable;
- production logs show no unresolved build or runtime error.

## 14. Rollout

1. Implement and test the release manifest generator and Tauri update engine.
2. Implement and visually verify the launcher UI.
3. Publish a Preview containing both the legacy and Tauri launchers plus the new
   manifest contract.
4. Validate real install, update, repair, rollback, and launch behavior.
5. Make the Tauri installer the canonical `VoxteraLauncher-setup.exe` asset.
6. Build and verify the website against that live Preview contract.
7. Deploy a Vercel preview, validate it, and promote it to production.
8. Remove the legacy Python launcher only in a later cleanup after the new launcher
has demonstrated successful field use.

Windows code signing is recommended for the public installer but is not a hidden
acceptance dependency. If no signing certificate is available for the first Preview,
the release notes and website must state that the installer is unsigned and may
trigger a Windows reputation warning; the build must not claim a signature it lacks.
