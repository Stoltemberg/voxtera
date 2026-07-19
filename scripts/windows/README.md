# Windows development environment

These scripts target 64-bit Windows 10 and Windows 11 client editions only.
Windows Server and 32-bit Windows are unsupported.

The bootstrap checks and attempts to install the prerequisites used to compile
the Veloren client and server. It does not compile or run the project. Use the
final doctor report to identify anything that still needs attention.

## Install missing prerequisites

From the repository root:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\scripts\windows\bootstrap.ps1
```

If Windows prompts, approve the elevation request. The script leaves complete,
usable prerequisites alone and invokes Winget only for missing or incomplete
entries; it does not intentionally upgrade complete tools.

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
