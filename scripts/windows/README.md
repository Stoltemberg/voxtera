# Windows development environment

The bootstrap prepares the complete Veloren client and server compilation
environment. It does not compile or run the project.

## Install missing prerequisites

From the repository root:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\windows\bootstrap.ps1
```

Approve the single UAC request. The script installs missing packages only and
does not upgrade tools that are already usable.

## Preview without changing the machine

```powershell
.\scripts\windows\bootstrap.ps1 -WhatIf
```

## Diagnose the environment

```powershell
.\scripts\windows\doctor.ps1
.\scripts\windows\doctor.ps1 -Json
```

Exit codes are `0` for healthy, `1` for incomplete, and `2` for unsupported
platform, invalid repository, or internal script failure.

## Managed prerequisites

- Git and Git LFS
- Visual Studio Build Tools 2022 with the Visual C++ workload and Windows SDK
- CMake
- Ninja
- Python 3.13
- Rustup
- the nightly from `rust-toolchain`
- `rustfmt` and `clippy`

Logs are stored under `%LOCALAPPDATA%\VelorenDev\logs`. Restart Windows when
the final report shows a pending restart, then reopen the terminal.

If Winget is missing, install Microsoft App Installer from Microsoft and rerun
the bootstrap.
