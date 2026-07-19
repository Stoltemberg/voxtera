# Windows Development Environment Bootstrap Design

## Status

Approved for implementation planning on 2026-07-18.

## Purpose

Provide a repeatable Windows bootstrap for this private Veloren project. The
bootstrap prepares the complete client and server compilation environment
without compiling or running the project.

The primary target is 64-bit Windows 10 or Windows 11 using PowerShell 5.1 or
PowerShell 7.

## Goals

- Automatically install missing build prerequisites.
- Use Winget as the package source.
- Install only missing components; do not force upgrades.
- Support safe repeated execution and recovery from partial installation.
- Install the exact Rust nightly declared by the repository.
- Provide a standalone, read-only environment diagnostic.
- Keep logs and transient state outside the repository.
- Avoid compiling or launching Veloren.

## Non-goals

- Supporting Linux or macOS.
- Installing an IDE.
- Installing graphics drivers or the Vulkan SDK.
- Compiling, testing, or launching Veloren.
- Updating tools that are already installed.
- Automatically rebooting Windows.
- Uninstalling packages or rolling back partial installations.
- Configuring a game server, player data, or runtime settings.

## File Layout

```text
scripts/windows/
├── bootstrap.ps1
├── doctor.ps1
├── packages.psd1
└── Bootstrap.Common.psm1
```

Tests will live under:

```text
scripts/windows/tests/
```

The responsibilities are:

- `bootstrap.ps1`: validate the platform, request elevation, install missing
  prerequisites, refresh the current process environment, and invoke the
  doctor.
- `doctor.ps1`: perform read-only checks and produce human-readable or JSON
  output.
- `packages.psd1`: declare exact Winget package IDs, detection commands, and
  dependency information.
- `Bootstrap.Common.psm1`: contain shared process, Winget, PATH, logging,
  repository discovery, and result-formatting functions.

PowerShell is used because the bootstrap must run before Rust and Cargo exist.

## Packages and Components

The bootstrap manages these exact Winget packages:

| Purpose | Winget package ID |
| --- | --- |
| Git | `Git.Git` |
| Git LFS | `GitHub.GitLFS` |
| Rustup | `Rustlang.Rustup` |
| Visual Studio Build Tools | `Microsoft.VisualStudio.2022.BuildTools` |
| CMake | `Kitware.CMake` |
| Ninja | `Ninja-build.Ninja` |
| Python | `Python.Python.3.13` |

Visual Studio Build Tools must include:

- `Microsoft.VisualStudio.Workload.VCTools`
- the workload's recommended components
- a Windows SDK supplied by the workload

The Visual Studio installer runs passively, waits for completion, and does not
restart Windows.

After Rustup is available, the bootstrap reads the first non-empty line of the
repository's `rust-toolchain` file and installs that exact toolchain with:

- the minimal Rustup profile
- Cargo
- `rustfmt`
- `clippy`

The bootstrap does not change the user's global default Rust toolchain. Cargo
selects the pinned toolchain through the repository's `rust-toolchain` file.

After Git LFS is available, the bootstrap runs `git lfs install`. It does not
fetch or checkout LFS data because downloading repository content is outside
the environment-preparation scope.

## Installation Flow

1. Resolve the repository root relative to the script location.
2. Verify 64-bit Windows 10 or Windows 11.
3. Verify that Winget is available.
4. Create a timestamped log under
   `%LOCALAPPDATA%\VelorenDev\logs\`.
5. If the process is not elevated, relaunch the bootstrap once through UAC and
   terminate the original process.
6. Detect the current state of all declared prerequisites.
7. Install each missing independent package through Winget.
8. Install the Visual C++ workload and Windows SDK.
9. Refresh the current process PATH from machine and user environment values.
10. Run `git lfs install`.
11. Install the pinned Rust nightly and required components.
12. Refresh PATH again.
13. Run `doctor.ps1`.
14. Print an installation summary and return an exit code.

Winget invocations use exact IDs, disable interactive prompts, and explicitly
accept package and source agreements.

If Winget itself is missing, the bootstrap exits with instructions to install
Microsoft App Installer. It does not download and execute an MSIX bundle or
third-party package-manager bootstrap.

## Idempotence

Each package has two detection layers:

1. Check its executable, installation registry entry, or Visual Studio
   component state.
2. Query Winget when the local check is inconclusive.

An installed and usable dependency is reported as `ALREADY PRESENT` and is not
passed to `winget upgrade` or reinstalled.

The Rust toolchain is checked through `rustup toolchain list`. Components are
checked separately so a partial Rust installation can be completed.

The Visual C++ workload is checked through `vswhere.exe` and Visual Studio
component metadata. `cl.exe` is not required to be permanently present on the
ordinary user PATH.

## Elevation

The bootstrap requests UAC elevation at most once per invocation. A marker
argument prevents an elevation loop.

The elevated process preserves:

- the repository root
- `-WhatIf`
- the log path

The doctor never requests elevation.

## Diagnostic Checks

`doctor.ps1` checks:

- supported Windows version and 64-bit architecture
- Winget
- Git
- Git LFS
- Git LFS initialization
- repository root
- the asset canary and its `VELOREN_CANARY_MAGIC` prefix
- Rustup
- Cargo
- the exact pinned Rust toolchain
- `rustfmt`
- `clippy`
- Visual Studio Build Tools 2022
- Visual C++ workload
- Windows SDK
- CMake
- Ninja
- Python 3

Checks produce one of:

- `PASS`: the requirement is usable.
- `WARN`: the environment is usable but attention may be required, such as a
  pending restart.
- `FAIL`: the environment is incomplete.

Normal output is a concise table. `doctor.ps1 -Json` writes one valid JSON
document to standard output and sends no decorative text to that stream.

## Command-Line Interface

Normal installation:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\windows\bootstrap.ps1
```

Dry run:

```powershell
.\scripts\windows\bootstrap.ps1 -WhatIf
```

Read-only diagnosis:

```powershell
.\scripts\windows\doctor.ps1
.\scripts\windows\doctor.ps1 -Json
```

`bootstrap.ps1` and its mutating helper functions support PowerShell
`ShouldProcess`. Under `-WhatIf`, they do not invoke Winget, elevate, change Git
LFS configuration, install a Rust toolchain, or write machine configuration.
Writing a local diagnostic log is also skipped in `-WhatIf` mode.

## Failure Handling

- A transient Winget failure is retried once.
- Failure of an independent package does not stop other independent packages.
- A dependent step is marked `SKIPPED` when its prerequisite failed.
- No automatic rollback is attempted.
- No automatic reboot is performed.
- Pending-reboot state is detected and reported.
- The final summary uses `INSTALLED`, `ALREADY PRESENT`, `FAILED`, or `SKIPPED`
  for installation actions.
- Logs include invoked command names, package IDs, versions, exit codes, and
  error messages.
- Logs do not dump the complete environment or values unrelated to diagnosis.

Exit codes:

| Exit code | Meaning |
| --- | --- |
| `0` | All required doctor checks passed; warnings may be present. |
| `1` | One or more required dependencies are missing or unusable. |
| `2` | Unsupported platform, invalid repository, or internal script error. |

If the elevated child process returns an installer-specific restart-required
code, the bootstrap normalizes it into a warning and runs the doctor before
returning its final exit code.

## Security

- Install only exact package IDs from the configured Winget source.
- Do not execute downloaded scripts with `Invoke-Expression`.
- Do not bootstrap Scoop, Chocolatey, or another package manager.
- Do not persist credentials or environment dumps.
- Quote all external process arguments without shell reconstruction.
- Validate the repository root before reading `rust-toolchain` or the asset
  canary.
- Treat package output as untrusted text when writing logs.

## Test Strategy

Shared functions are isolated behind command-runner helpers so process
execution, Winget, the registry, and PATH can be simulated.

Automated tests cover:

- a machine with no prerequisites
- a fully prepared machine
- a partial installation
- a repeated bootstrap run
- a failed package followed by successful independent packages
- a missing Rustup followed by a skipped toolchain step
- an outdated current-process PATH after installation
- a missing Winget installation
- a missing or invalid repository root
- an invalid asset canary
- a missing Visual C++ workload
- a pending restart
- `-WhatIf` with zero mutating calls
- JSON output validity and stable field names
- exit codes `0`, `1`, and `2`

The tests must not install packages, request UAC elevation, or change machine
configuration.

## Acceptance Criteria

- Runs on PowerShell 5.1 and PowerShell 7.
- Runs correctly regardless of the caller's current directory.
- Finds the repository root through the script location.
- Installs all prerequisites needed to compile the complete Veloren workspace
  on supported Windows systems.
- Does not compile or run Veloren.
- Does not reboot Windows.
- Does not force upgrades of existing tools.
- A second successful run performs no package installations.
- A partial run resumes only missing work.
- `-WhatIf` performs no mutation.
- `doctor.ps1` remains read-only.
- `doctor.ps1 -Json` emits valid JSON and a coherent exit code.
- Logs are written only under `%LOCALAPPDATA%\VelorenDev\logs\`.
- No files outside the intended scripts, tests, and documentation are changed
  in the repository.
