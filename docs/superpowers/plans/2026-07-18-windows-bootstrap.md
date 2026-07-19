# Windows Development Environment Bootstrap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an idempotent Windows bootstrap and read-only doctor that prepare every prerequisite needed to compile the complete Veloren workspace without compiling or running it.

**Architecture:** PowerShell remains the only runtime prerequisite. A shared module owns process execution, detection, logging, installation, and orchestration; thin entry scripts expose bootstrap and doctor commands. All external effects pass through injectable runners so tests can exercise empty, partial, failed, repeated, and dry-run scenarios without changing the machine.

**Tech Stack:** Windows PowerShell 5.1, PowerShell 7, Winget, Rustup, Git LFS, Visual Studio Build Tools 2022, self-contained PowerShell test harness.

## Global Constraints

- Target only 64-bit Windows 10 and Windows 11.
- Support Windows PowerShell 5.1 and PowerShell 7.
- Use exact Winget package IDs and do not introduce Scoop or Chocolatey.
- Install missing dependencies only; never force upgrades.
- Install the Rust nightly named by the first non-empty line of `rust-toolchain`.
- Install `rustfmt` and `clippy` without changing the global default toolchain.
- Do not compile, test, or launch Veloren from the bootstrap or doctor.
- Do not automatically reboot, uninstall, or roll back packages.
- Store logs only under `%LOCALAPPDATA%\VelorenDev\logs\`.
- Keep `doctor.ps1` read-only and make `bootstrap.ps1 -WhatIf` mutation-free.
- Exit `0` for a healthy environment, `1` for missing or unusable requirements, and `2` for unsupported platform, invalid repository, or internal error.

---

## File Map

- Create `scripts/windows/Bootstrap.Common.psm1`: shared result types, command runner, PATH refresh, logging, probes, installers, and workflow functions.
- Create `scripts/windows/packages.psd1`: exact package declarations and Visual Studio installer arguments.
- Create `scripts/windows/doctor.ps1`: thin human/JSON diagnostic entrypoint.
- Create `scripts/windows/bootstrap.ps1`: thin elevated installation entrypoint.
- Create `scripts/windows/tests/TestHarness.psm1`: dependency-free test primitives.
- Create `scripts/windows/tests/Common.Tests.ps1`: pure helper and repository tests.
- Create `scripts/windows/tests/Doctor.Tests.ps1`: probe aggregation, JSON, and exit-code tests.
- Create `scripts/windows/tests/Installer.Tests.ps1`: retry, dependency, idempotence, and dry-run tests.
- Create `scripts/windows/tests/Bootstrap.Tests.ps1`: elevation and end-to-end workflow tests with fakes.
- Create `scripts/windows/tests/run-tests.ps1`: deterministic test runner.
- Create `scripts/windows/README.md`: user-facing commands, package list, and troubleshooting.

---

### Task 1: Dependency-Free Test Harness and Common Primitives

**Files:**
- Create: `scripts/windows/tests/TestHarness.psm1`
- Create: `scripts/windows/tests/run-tests.ps1`
- Create: `scripts/windows/tests/Common.Tests.ps1`
- Create: `scripts/windows/Bootstrap.Common.psm1`

**Interfaces:**
- Produces: `Test-Case`, `Assert-True`, `Assert-Equal`, `Assert-Match`, `Complete-TestRun`.
- Produces: `New-CheckResult`, `Get-RepositoryRoot`, `Invoke-ExternalCommand`, `Refresh-ProcessPath`, `New-BootstrapLogPath`, `Write-BootstrapLog`, `Test-IsAdministrator`, `Test-PendingRestart`.
- `Invoke-ExternalCommand` returns an object with `ExitCode`, `Output`, and `Command`.

- [ ] **Step 1: Create the test harness**

```powershell
# scripts/windows/tests/TestHarness.psm1
Set-StrictMode -Version Latest
$script:Failures = 0
$script:Executed = 0

function Test-Case {
    param([Parameter(Mandatory)][string]$Name, [Parameter(Mandatory)][scriptblock]$Body)
    $script:Executed++
    try {
        & $Body
        Write-Host "PASS  $Name"
    } catch {
        $script:Failures++
        Write-Host "FAIL  $Name`n      $($_.Exception.Message)" -ForegroundColor Red
    }
}

function Assert-True {
    param([Parameter(Mandatory)]$Value, [string]$Message = 'Expected value to be true.')
    if (-not [bool]$Value) { throw $Message }
}

function Assert-Equal {
    param([Parameter(Mandatory)]$Expected, [Parameter(Mandatory)]$Actual)
    if ($Expected -ne $Actual) {
        throw "Expected '$Expected' but received '$Actual'."
    }
}

function Assert-Match {
    param([Parameter(Mandatory)][string]$Pattern, [Parameter(Mandatory)][string]$Actual)
    if ($Actual -notmatch $Pattern) {
        throw "Expected '$Actual' to match '$Pattern'."
    }
}

function Complete-TestRun {
    Write-Host "`nExecuted: $script:Executed  Failed: $script:Failures"
    if ($script:Failures -gt 0) { exit 1 }
    exit 0
}

Export-ModuleMember -Function Test-Case, Assert-True, Assert-Equal, Assert-Match, Complete-TestRun
```

```powershell
# scripts/windows/tests/run-tests.ps1
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'TestHarness.psm1') -Force

Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*.Tests.ps1' |
    Sort-Object Name |
    ForEach-Object { . $_.FullName }

Complete-TestRun
```

- [ ] **Step 2: Write failing common primitive tests**

```powershell
# scripts/windows/tests/Common.Tests.ps1
$modulePath = Join-Path (Split-Path $PSScriptRoot -Parent) 'Bootstrap.Common.psm1'
Import-Module $modulePath -Force

Test-Case 'repository root contains Cargo.toml and rust-toolchain' {
    $root = Get-RepositoryRoot
    Assert-True (Test-Path -LiteralPath (Join-Path $root 'Cargo.toml'))
    Assert-True (Test-Path -LiteralPath (Join-Path $root 'rust-toolchain'))
}

Test-Case 'check result has stable fields' {
    $result = New-CheckResult -Name 'Git' -Status 'PASS' -Detail '2.55'
    Assert-Equal 'Git' $result.Name
    Assert-Equal 'PASS' $result.Status
    Assert-Equal '2.55' $result.Detail
}

Test-Case 'external command preserves arguments and exit code' {
    $hostExecutable = if ($PSVersionTable.PSEdition -eq 'Core') {
        (Get-Command pwsh.exe).Source
    } else {
        (Get-Command powershell.exe).Source
    }
    $result = Invoke-ExternalCommand -FilePath $hostExecutable -Arguments @(
        '-NoProfile', '-Command', 'Write-Output alpha; exit 7'
    )
    Assert-Equal 7 $result.ExitCode
    Assert-Match 'alpha' ($result.Output -join "`n")
}

Test-Case 'log path stays under LOCALAPPDATA' {
    $path = New-BootstrapLogPath -Timestamp ([datetime]'2026-07-18T12:34:56')
    Assert-True $path.StartsWith($env:LOCALAPPDATA, [System.StringComparison]::OrdinalIgnoreCase)
    Assert-Match 'VelorenDev[\\/]logs' $path
}
```

- [ ] **Step 3: Run the tests and confirm the module is missing**

Run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/tests/run-tests.ps1
```

Expected: exit `1` with failures stating that `Bootstrap.Common.psm1` or its functions do not exist.

- [ ] **Step 4: Implement the common primitives**

```powershell
# scripts/windows/Bootstrap.Common.psm1
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function New-CheckResult {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][ValidateSet('PASS', 'WARN', 'FAIL')][string]$Status,
        [Parameter(Mandatory)][string]$Detail
    )
    [pscustomobject][ordered]@{ Name = $Name; Status = $Status; Detail = $Detail }
}

function Get-RepositoryRoot {
    [CmdletBinding()]
    param([string]$StartPath = (Join-Path $PSScriptRoot '..\..'))
    $candidate = (Resolve-Path -LiteralPath $StartPath -ErrorAction Stop).Path
    if (-not (Test-Path -LiteralPath (Join-Path $candidate 'Cargo.toml')) -or
        -not (Test-Path -LiteralPath (Join-Path $candidate 'rust-toolchain'))) {
        throw "Invalid Veloren repository root: $candidate"
    }
    $candidate
}

function Invoke-ExternalCommand {
    [CmdletBinding()]
    param(
        [Parameter(Mandatory)][string]$FilePath,
        [string[]]$Arguments = @()
    )
    $output = @(& $FilePath @Arguments 2>&1 | ForEach-Object { $_.ToString() })
    $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { $LASTEXITCODE }
    [pscustomobject][ordered]@{
        ExitCode = [int]$exitCode
        Output   = $output
        Command  = "$FilePath $($Arguments -join ' ')".Trim()
    }
}

function Refresh-ProcessPath {
    $machine = [Environment]::GetEnvironmentVariable('Path', 'Machine')
    $user = [Environment]::GetEnvironmentVariable('Path', 'User')
    $segments = @($machine, $user, (Join-Path $env:USERPROFILE '.cargo\bin')) |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) }
    $env:Path = $segments -join ';'
    $env:Path
}

function New-BootstrapLogPath {
    param([datetime]$Timestamp = (Get-Date))
    $directory = Join-Path $env:LOCALAPPDATA 'VelorenDev\logs'
    Join-Path $directory ("bootstrap-{0}.log" -f $Timestamp.ToString('yyyyMMdd-HHmmss'))
}

function Write-BootstrapLog {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][string]$Message)
    $directory = Split-Path -Parent $Path
    if (-not (Test-Path -LiteralPath $directory)) {
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
    }
    Add-Content -LiteralPath $Path -Value ("[{0:o}] {1}" -f (Get-Date), $Message)
}

function Test-IsAdministrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = New-Object Security.Principal.WindowsPrincipal($identity)
    $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Test-PendingRestart {
    $paths = @(
        'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Component Based Servicing\RebootPending',
        'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\WindowsUpdate\Auto Update\RebootRequired'
    )
    [bool]($paths | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1)
}

Export-ModuleMember -Function *
```

- [ ] **Step 5: Run the tests in Windows PowerShell and PowerShell 7**

Run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/tests/run-tests.ps1
pwsh.exe -NoProfile -File scripts/windows/tests/run-tests.ps1
```

Expected: both commands report `Failed: 0`. If `pwsh.exe` is not installed on the development machine, record that environment limitation and run the PowerShell 7 command in CI before completion.

- [ ] **Step 6: Commit the common foundation**

```powershell
git add scripts/windows/Bootstrap.Common.psm1 scripts/windows/tests
git commit -m "test: add Windows bootstrap harness and primitives"
```

---

### Task 2: Package Manifest and Read-Only Doctor

**Files:**
- Create: `scripts/windows/packages.psd1`
- Create: `scripts/windows/doctor.ps1`
- Create: `scripts/windows/tests/Doctor.Tests.ps1`
- Modify: `scripts/windows/Bootstrap.Common.psm1`

**Interfaces:**
- Consumes: `New-CheckResult`, `Get-RepositoryRoot`, `Invoke-ExternalCommand`, `Test-PendingRestart`.
- Produces: `Get-PinnedToolchain`, `Get-DefaultDoctorProbes`, `Get-DoctorReport`, `Get-DoctorExitCode`, `Write-DoctorTable`.
- A probe is a zero-argument scriptblock returning one `New-CheckResult` object.

- [ ] **Step 1: Write failing manifest and doctor aggregation tests**

```powershell
# scripts/windows/tests/Doctor.Tests.ps1
$windowsRoot = Split-Path $PSScriptRoot -Parent
Import-Module (Join-Path $windowsRoot 'Bootstrap.Common.psm1') -Force

Test-Case 'manifest contains exact package IDs' {
    $manifest = Import-PowerShellDataFile (Join-Path $windowsRoot 'packages.psd1')
    Assert-Equal 'Git.Git' $manifest.Packages.Git.Id
    Assert-Equal 'GitHub.GitLFS' $manifest.Packages.GitLfs.Id
    Assert-Equal 'Rustlang.Rustup' $manifest.Packages.Rustup.Id
    Assert-Equal 'Microsoft.VisualStudio.2022.BuildTools' $manifest.Packages.VisualStudio.Id
    Assert-Equal 'Kitware.CMake' $manifest.Packages.CMake.Id
    Assert-Equal 'Ninja-build.Ninja' $manifest.Packages.Ninja.Id
    Assert-Equal 'Python.Python.3.13' $manifest.Packages.Python.Id
    Assert-Equal 'Git' $manifest.Order[0]
    Assert-Equal 'Rustup' $manifest.Order[-1]
}

Test-Case 'doctor aggregates injected probes in order' {
    $probes = [ordered]@{
        Git = { New-CheckResult Git PASS 'present' }
        Cargo = { New-CheckResult Cargo FAIL 'missing' }
    }
    $report = @(Get-DoctorReport -Probes $probes)
    Assert-Equal 2 $report.Count
    Assert-Equal 'Git' $report[0].Name
    Assert-Equal 'Cargo' $report[1].Name
    Assert-Equal 1 (Get-DoctorExitCode -Report $report)
}

Test-Case 'warnings do not fail the doctor' {
    $report = @(
        New-CheckResult Restart WARN 'pending'
        New-CheckResult Git PASS 'present'
    )
    Assert-Equal 0 (Get-DoctorExitCode -Report $report)
}
```

- [ ] **Step 2: Run tests to verify missing manifest and doctor functions**

Run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/tests/run-tests.ps1
```

Expected: exit `1` mentioning missing `packages.psd1` and undefined `Get-DoctorReport`.

- [ ] **Step 3: Create the package manifest**

```powershell
# scripts/windows/packages.psd1
@{
    Order = @('Git', 'GitLfs', 'VisualStudio', 'CMake', 'Ninja', 'Python', 'Rustup')
    Packages = @{
        Git = @{
            Id = 'Git.Git'
            Command = 'git.exe'
            DependsOn = @()
            WingetArguments = @()
        }
        GitLfs = @{
            Id = 'GitHub.GitLFS'
            Command = 'git-lfs.exe'
            DependsOn = @('Git')
            WingetArguments = @()
        }
        VisualStudio = @{
            Id = 'Microsoft.VisualStudio.2022.BuildTools'
            Command = $null
            DependsOn = @()
            WingetArguments = @(
                '--override',
                '--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'
            )
        }
        CMake = @{
            Id = 'Kitware.CMake'
            Command = 'cmake.exe'
            DependsOn = @()
            WingetArguments = @()
        }
        Ninja = @{
            Id = 'Ninja-build.Ninja'
            Command = 'ninja.exe'
            DependsOn = @()
            WingetArguments = @()
        }
        Python = @{
            Id = 'Python.Python.3.13'
            Command = 'python.exe'
            DependsOn = @()
            WingetArguments = @()
        }
        Rustup = @{
            Id = 'Rustlang.Rustup'
            Command = 'rustup.exe'
            DependsOn = @('VisualStudio')
            WingetArguments = @()
        }
    }
}
```

- [ ] **Step 4: Implement doctor aggregation and production probes**

Append these functions to `scripts/windows/Bootstrap.Common.psm1` before `Export-ModuleMember`:

```powershell
function Get-PinnedToolchain {
    param([string]$RepositoryRoot = (Get-RepositoryRoot))
    $line = Get-Content -LiteralPath (Join-Path $RepositoryRoot 'rust-toolchain') |
        Where-Object { -not [string]::IsNullOrWhiteSpace($_) } |
        Select-Object -First 1
    if ([string]::IsNullOrWhiteSpace($line)) { throw 'rust-toolchain is empty.' }
    $line.Trim()
}

function Get-CommandCheck {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Command,
        [string[]]$VersionArguments = @('--version')
    )
    $resolved = Get-Command $Command -ErrorAction SilentlyContinue
    if ($null -eq $resolved) { return New-CheckResult $Name FAIL "$Command is missing." }
    $result = Invoke-ExternalCommand $resolved.Source $VersionArguments
    if ($result.ExitCode -ne 0) {
        return New-CheckResult $Name FAIL (($result.Output -join ' ').Trim())
    }
    New-CheckResult $Name PASS (($result.Output -join ' ').Trim())
}

function Get-VisualStudioCheck {
    $vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere)) {
        return New-CheckResult 'Visual Studio Build Tools' FAIL 'vswhere.exe is missing.'
    }
    $result = Invoke-ExternalCommand $vswhere @(
        '-latest', '-products', '*',
        '-requires', 'Microsoft.VisualStudio.Workload.VCTools',
        '-property', 'installationPath'
    )
    $path = ($result.Output | Select-Object -First 1)
    if ($result.ExitCode -ne 0 -or [string]::IsNullOrWhiteSpace($path)) {
        return New-CheckResult 'Visual Studio Build Tools' FAIL 'C++ workload is missing.'
    }
    New-CheckResult 'Visual Studio Build Tools' PASS $path
}

function Get-WindowsSdkCheck {
    $root = Get-ItemPropertyValue `
        -LiteralPath 'HKLM:\SOFTWARE\Microsoft\Windows Kits\Installed Roots' `
        -Name KitsRoot10 -ErrorAction SilentlyContinue
    if ([string]::IsNullOrWhiteSpace($root) -or -not (Test-Path -LiteralPath (Join-Path $root 'Include'))) {
        return New-CheckResult 'Windows SDK' FAIL 'Windows 10/11 SDK is missing.'
    }
    New-CheckResult 'Windows SDK' PASS $root
}

function Get-AssetCheck {
    param([string]$RepositoryRoot = (Get-RepositoryRoot))
    $canary = Join-Path $RepositoryRoot 'assets\common\canary.canary'
    if (-not (Test-Path -LiteralPath $canary)) {
        return New-CheckResult Assets FAIL 'Asset canary is missing.'
    }
    $first = Get-Content -LiteralPath $canary -TotalCount 1
    if ($first -ne 'VELOREN_CANARY_MAGIC') {
        return New-CheckResult Assets FAIL 'Asset canary is invalid; check Git LFS.'
    }
    New-CheckResult Assets PASS 'Asset canary is valid.'
}

function Get-RustToolchainChecks {
    param([string]$RepositoryRoot = (Get-RepositoryRoot))
    $pinned = Get-PinnedToolchain $RepositoryRoot
    $rustup = Get-Command rustup.exe -ErrorAction SilentlyContinue
    if ($null -eq $rustup) {
        return @(
            (New-CheckResult Rustup FAIL 'rustup.exe is missing.'),
            (New-CheckResult 'Pinned Rust toolchain' FAIL "$pinned is not available."),
            (New-CheckResult 'Rust components' FAIL 'rustfmt and clippy are not available.')
        )
    }
    $toolchains = Invoke-ExternalCommand $rustup.Source @('toolchain', 'list')
    $hasPinned = ($toolchains.Output -join "`n") -match [regex]::Escape($pinned)
    $components = if ($hasPinned) {
        Invoke-ExternalCommand $rustup.Source @('component', 'list', '--toolchain', $pinned, '--installed')
    } else { $null }
    $componentText = if ($null -eq $components) { '' } else { $components.Output -join "`n" }
    @(
        (New-CheckResult Rustup PASS ((Invoke-ExternalCommand $rustup.Source @('--version')).Output -join ' ')),
        (New-CheckResult 'Pinned Rust toolchain' $(if ($hasPinned) { 'PASS' } else { 'FAIL' }) $pinned),
        (New-CheckResult 'Rust components' $(if ($componentText -match 'rustfmt' -and $componentText -match 'clippy') { 'PASS' } else { 'FAIL' }) 'rustfmt, clippy')
    )
}

function Get-DefaultDoctorProbes {
    param([string]$RepositoryRoot = (Get-RepositoryRoot))
    $root = $RepositoryRoot
    [ordered]@{
        Platform = ({
            $version = [Environment]::OSVersion.Version
            $ok = [Environment]::OSVersion.Platform -eq [PlatformID]::Win32NT -and
                [Environment]::Is64BitOperatingSystem -and $version.Major -ge 10
            New-CheckResult Platform $(if ($ok) { 'PASS' } else { 'FAIL' }) ([Environment]::OSVersion.VersionString)
        }).GetNewClosure()
        Winget = { Get-CommandCheck Winget winget.exe @('--version') }
        Git = { Get-CommandCheck Git git.exe @('--version') }
        GitLfs = { Get-CommandCheck 'Git LFS' git-lfs.exe @('--version') }
        GitLfsConfig = {
            if ($null -eq (Get-Command git.exe -ErrorAction SilentlyContinue)) {
                return New-CheckResult 'Git LFS configuration' FAIL 'Git is unavailable.'
            }
            $result = Invoke-ExternalCommand 'git.exe' @('lfs', 'env')
            New-CheckResult 'Git LFS configuration' $(if ($result.ExitCode -eq 0) { 'PASS' } else { 'FAIL' }) `
                (($result.Output -join ' ').Trim())
        }
        Assets = ({ Get-AssetCheck $root }).GetNewClosure()
        VisualStudio = { Get-VisualStudioCheck }
        WindowsSdk = { Get-WindowsSdkCheck }
        CMake = { Get-CommandCheck CMake cmake.exe @('--version') }
        Ninja = { Get-CommandCheck Ninja ninja.exe @('--version') }
        Python = { Get-CommandCheck Python python.exe @('--version') }
        Cargo = { Get-CommandCheck Cargo cargo.exe @('--version') }
        Rust = ({ Get-RustToolchainChecks $root }).GetNewClosure()
        Restart = {
            New-CheckResult Restart $(if (Test-PendingRestart) { 'WARN' } else { 'PASS' }) `
                $(if (Test-PendingRestart) { 'Windows restart is pending.' } else { 'No restart is pending.' })
        }
    }
}

function Get-DoctorReport {
    param([Parameter(Mandatory)][System.Collections.IDictionary]$Probes)
    foreach ($entry in $Probes.GetEnumerator()) {
        @(& $entry.Value) | ForEach-Object { $_ }
    }
}

function Get-DoctorExitCode {
    param([Parameter(Mandatory)][object[]]$Report)
    if (@($Report | Where-Object Status -eq 'FAIL').Count -gt 0) { 1 } else { 0 }
}

function Write-DoctorTable {
    param([Parameter(Mandatory)][object[]]$Report)
    foreach ($item in $Report) {
        "{0,-5} {1,-28} {2}" -f $item.Status, $item.Name, $item.Detail
    }
}
```

- [ ] **Step 5: Create the doctor entrypoint**

```powershell
# scripts/windows/doctor.ps1
[CmdletBinding()]
param([switch]$Json)

$ErrorActionPreference = 'Stop'
try {
    Import-Module (Join-Path $PSScriptRoot 'Bootstrap.Common.psm1') -Force
    $root = Get-RepositoryRoot
    $report = @(Get-DoctorReport -Probes (Get-DefaultDoctorProbes -RepositoryRoot $root))
    if ($Json) {
        [pscustomobject][ordered]@{
            RepositoryRoot = $root
            Checks = $report
            Healthy = (Get-DoctorExitCode $report) -eq 0
        } | ConvertTo-Json -Depth 5
    } else {
        Write-Host 'Veloren Development Environment'
        Write-Host ''
        Write-DoctorTable $report | Write-Host
    }
    exit (Get-DoctorExitCode $report)
} catch {
    if ($Json) {
        [pscustomobject][ordered]@{
            RepositoryRoot = $null
            Checks = @()
            Healthy = $false
            Error = $_.Exception.Message
        } | ConvertTo-Json -Depth 5
    } else {
        Write-Error $_.Exception.Message
    }
    exit 2
}
```

- [ ] **Step 6: Add JSON validity test and run all doctor tests**

Append to `scripts/windows/tests/Doctor.Tests.ps1`:

```powershell
Test-Case 'doctor JSON is one valid document' {
    $doctor = Join-Path $windowsRoot 'doctor.ps1'
    $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $doctor -Json
    $exitCode = $LASTEXITCODE
    $document = ($output -join "`n") | ConvertFrom-Json
    Assert-True ($exitCode -in @(0, 1))
    Assert-True ($document.Checks.Count -gt 0)
    Assert-True ($null -ne $document.Healthy)
}
```

Run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/tests/run-tests.ps1
```

Expected: `Failed: 0`; the doctor process may exit `1` on an incomplete development machine, while still producing valid JSON.

- [ ] **Step 7: Commit the manifest and doctor**

```powershell
git add scripts/windows/packages.psd1 scripts/windows/doctor.ps1 scripts/windows/Bootstrap.Common.psm1 scripts/windows/tests/Doctor.Tests.ps1
git commit -m "feat: add read-only Windows environment doctor"
```

---

### Task 3: Winget, Git LFS, and Rust Installers

**Files:**
- Create: `scripts/windows/tests/Installer.Tests.ps1`
- Modify: `scripts/windows/Bootstrap.Common.psm1`

**Interfaces:**
- Consumes: package records from `packages.psd1`, `Invoke-ExternalCommand`, `Get-PinnedToolchain`, `Write-BootstrapLog`.
- Produces: `Install-WingetPackage`, `Initialize-GitLfs`, `Install-PinnedRustToolchain`, `Test-PackagePresent`.
- Installer results contain `Name`, `Status`, and `Detail`; action status is one of `INSTALLED`, `ALREADY PRESENT`, `FAILED`, `SKIPPED`.

- [ ] **Step 1: Write failing retry, idempotence, and dependency tests**

```powershell
# scripts/windows/tests/Installer.Tests.ps1
$windowsRoot = Split-Path $PSScriptRoot -Parent
Import-Module (Join-Path $windowsRoot 'Bootstrap.Common.psm1') -Force

Test-Case 'Winget installer retries once and succeeds' {
    $script:calls = 0
    $runner = {
        param($FilePath, $Arguments)
        $script:calls++
        [pscustomobject]@{
            ExitCode = $(if ($script:calls -eq 1) { 1 } else { 0 })
            Output = @('result')
            Command = "$FilePath $($Arguments -join ' ')"
        }
    }
    $result = Install-WingetPackage -Name Git -Package @{ Id = 'Git.Git'; WingetArguments = @() } -Runner $runner -Confirm:$false
    Assert-Equal 2 $script:calls
    Assert-Equal 'INSTALLED' $result.Status
}

Test-Case 'present package is not sent to Winget' {
    $script:calls = 0
    $detector = { param($Package) $true }
    $runner = { param($FilePath, $Arguments) $script:calls++; throw 'must not run' }
    $result = Install-WingetPackage -Name Git -Package @{ Id = 'Git.Git'; WingetArguments = @() } `
        -Detector $detector -Runner $runner -Confirm:$false
    Assert-Equal 0 $script:calls
    Assert-Equal 'ALREADY PRESENT' $result.Status
}

Test-Case 'restart-required installer code is successful' {
    $runner = {
        param($FilePath, $Arguments)
        [pscustomobject]@{
            ExitCode = 3010
            Output = @('restart required')
            Command = 'winget install'
        }
    }
    $result = Install-WingetPackage -Name CMake `
        -Package @{ Id = 'Kitware.CMake'; WingetArguments = @() } `
        -Detector { param($Package) $false } -Runner $runner -Confirm:$false
    Assert-Equal 'INSTALLED' $result.Status
    Assert-Match 'restart' $result.Detail
}

Test-Case 'failed prerequisite produces skipped Rust toolchain' {
    $result = Install-PinnedRustToolchain -RepositoryRoot (Get-RepositoryRoot) `
        -RustupAvailable:$false -Confirm:$false
    Assert-Equal 'SKIPPED' $result.Status
}
```

- [ ] **Step 2: Run tests to verify installer functions are undefined**

Run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/tests/run-tests.ps1
```

Expected: exit `1` with undefined `Install-WingetPackage` and `Install-PinnedRustToolchain`.

- [ ] **Step 3: Implement package detection and Winget installation**

Append to `scripts/windows/Bootstrap.Common.psm1`:

```powershell
function New-InstallResult {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][ValidateSet('INSTALLED', 'ALREADY PRESENT', 'FAILED', 'SKIPPED')][string]$Status,
        [Parameter(Mandatory)][string]$Detail
    )
    [pscustomobject][ordered]@{ Name = $Name; Status = $Status; Detail = $Detail }
}

function Test-PackagePresent {
    param([Parameter(Mandatory)][hashtable]$Package)
    if (-not [string]::IsNullOrWhiteSpace($Package.Command)) {
        return $null -ne (Get-Command $Package.Command -ErrorAction SilentlyContinue)
    }
    if ($Package.Id -eq 'Microsoft.VisualStudio.2022.BuildTools') {
        return (Get-VisualStudioCheck).Status -eq 'PASS'
    }
    $false
}

function Install-WingetPackage {
    [CmdletBinding(SupportsShouldProcess)]
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][hashtable]$Package,
        [scriptblock]$Detector = { param($Value) Test-PackagePresent $Value },
        [scriptblock]$Runner = { param($FilePath, $Arguments) Invoke-ExternalCommand $FilePath $Arguments },
        [string]$LogPath
    )
    if (& $Detector $Package) {
        return New-InstallResult $Name 'ALREADY PRESENT' $Package.Id
    }
    if (-not $PSCmdlet.ShouldProcess($Package.Id, 'Install Winget package')) {
        return New-InstallResult $Name 'SKIPPED' 'WhatIf'
    }
    $arguments = @(
        'install', '--id', $Package.Id, '--exact',
        '--accept-package-agreements', '--accept-source-agreements',
        '--disable-interactivity'
    ) + @($Package.WingetArguments)
    for ($attempt = 1; $attempt -le 2; $attempt++) {
        $result = & $Runner 'winget.exe' $arguments
        if ($LogPath) { Write-BootstrapLog $LogPath "$($result.Command) => $($result.ExitCode)" }
        if ($result.ExitCode -in @(0, 1641, 3010)) {
            $detail = if ($result.ExitCode -eq 0) { $Package.Id } else { "$($Package.Id); restart required" }
            return New-InstallResult $Name 'INSTALLED' $detail
        }
    }
    New-InstallResult $Name 'FAILED' (($result.Output -join ' ').Trim())
}
```

- [ ] **Step 4: Implement Git LFS and pinned Rust setup**

Append to `scripts/windows/Bootstrap.Common.psm1`:

```powershell
function Initialize-GitLfs {
    [CmdletBinding(SupportsShouldProcess)]
    param(
        [bool]$GitLfsAvailable = ($null -ne (Get-Command git-lfs.exe -ErrorAction SilentlyContinue)),
        [scriptblock]$Runner = { param($FilePath, $Arguments) Invoke-ExternalCommand $FilePath $Arguments },
        [string]$LogPath
    )
    if (-not $GitLfsAvailable) { return New-InstallResult 'Git LFS initialization' SKIPPED 'Git LFS is unavailable.' }
    if (-not $PSCmdlet.ShouldProcess('Git LFS', 'Initialize user configuration')) {
        return New-InstallResult 'Git LFS initialization' SKIPPED 'WhatIf'
    }
    $result = & $Runner 'git.exe' @('lfs', 'install')
    if ($LogPath) { Write-BootstrapLog $LogPath "$($result.Command) => $($result.ExitCode)" }
    if ($result.ExitCode -eq 0) {
        New-InstallResult 'Git LFS initialization' INSTALLED 'git lfs install'
    } else {
        New-InstallResult 'Git LFS initialization' FAILED (($result.Output -join ' ').Trim())
    }
}

function Install-PinnedRustToolchain {
    [CmdletBinding(SupportsShouldProcess)]
    param(
        [string]$RepositoryRoot = (Get-RepositoryRoot),
        [bool]$RustupAvailable = ($null -ne (Get-Command rustup.exe -ErrorAction SilentlyContinue)),
        [scriptblock]$Runner = { param($FilePath, $Arguments) Invoke-ExternalCommand $FilePath $Arguments },
        [string]$LogPath
    )
    $pinned = Get-PinnedToolchain $RepositoryRoot
    if (-not $RustupAvailable) {
        return New-InstallResult 'Pinned Rust toolchain' SKIPPED 'Rustup is unavailable.'
    }
    $listed = & $Runner 'rustup.exe' @('toolchain', 'list')
    $components = & $Runner 'rustup.exe' @('component', 'list', '--toolchain', $pinned, '--installed')
    $ready = (($listed.Output -join "`n") -match [regex]::Escape($pinned)) -and
        (($components.Output -join "`n") -match 'rustfmt') -and
        (($components.Output -join "`n") -match 'clippy')
    if ($ready) { return New-InstallResult 'Pinned Rust toolchain' 'ALREADY PRESENT' $pinned }
    if (-not $PSCmdlet.ShouldProcess($pinned, 'Install Rust toolchain and components')) {
        return New-InstallResult 'Pinned Rust toolchain' SKIPPED 'WhatIf'
    }
    $result = & $Runner 'rustup.exe' @(
        'toolchain', 'install', $pinned,
        '--profile', 'minimal',
        '--component', 'rustfmt',
        '--component', 'clippy'
    )
    if ($LogPath) { Write-BootstrapLog $LogPath "$($result.Command) => $($result.ExitCode)" }
    if ($result.ExitCode -eq 0) {
        New-InstallResult 'Pinned Rust toolchain' INSTALLED $pinned
    } else {
        New-InstallResult 'Pinned Rust toolchain' FAILED (($result.Output -join ' ').Trim())
    }
}
```

- [ ] **Step 5: Run installer tests**

Run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/tests/run-tests.ps1
```

Expected: `Failed: 0`; no Winget process runs because tests inject runners or detect existing state.

- [ ] **Step 6: Commit installer behavior**

```powershell
git add scripts/windows/Bootstrap.Common.psm1 scripts/windows/tests/Installer.Tests.ps1
git commit -m "feat: add idempotent Windows prerequisite installers"
```

---

### Task 4: Bootstrap Workflow, Elevation, and Dry Run

**Files:**
- Create: `scripts/windows/bootstrap.ps1`
- Create: `scripts/windows/tests/Bootstrap.Tests.ps1`
- Modify: `scripts/windows/Bootstrap.Common.psm1`

**Interfaces:**
- Consumes: package manifest, installers, `Refresh-ProcessPath`, `Get-DoctorReport`.
- Produces: `Get-ElevationArguments`, `Start-ElevatedBootstrap`, `Invoke-BootstrapWorkflow`, `Get-BootstrapExitCode`.
- `Invoke-BootstrapWorkflow` accepts injected detector, installer, Git LFS, Rust, and doctor scriptblocks.

- [ ] **Step 1: Write failing elevation and workflow tests**

```powershell
# scripts/windows/tests/Bootstrap.Tests.ps1
$windowsRoot = Split-Path $PSScriptRoot -Parent
Import-Module (Join-Path $windowsRoot 'Bootstrap.Common.psm1') -Force
$manifest = Import-PowerShellDataFile (Join-Path $windowsRoot 'packages.psd1')

Test-Case 'elevation arguments preserve dry run and log path' {
    $args = Get-ElevationArguments -ScriptPath 'C:\repo\bootstrap.ps1' -LogPath 'C:\logs\a.log' -DryRun
    $joined = $args -join ' '
    Assert-Match '-Elevated' $joined
    Assert-Match '-WhatIf' $joined
    Assert-Match 'C:\\logs\\a.log' $joined
}

Test-Case 'workflow continues after independent package failure' {
    $installer = {
        param($Name, $Package)
        if ($Name -eq 'CMake') { New-InstallResult $Name FAILED 'simulated' }
        else { New-InstallResult $Name INSTALLED $Package.Id }
    }
    $results = @(Invoke-BootstrapWorkflow -Manifest $manifest -PackageInstaller $installer `
        -GitLfsInstaller { New-InstallResult Lfs INSTALLED ok } `
        -RustInstaller { New-InstallResult Rust INSTALLED ok })
    Assert-True ($results.Name -contains 'Ninja')
    Assert-Equal 'FAILED' (($results | Where-Object Name -eq CMake).Status)
}

Test-Case 'dry run produces no installer calls' {
    $script:calls = 0
    $installer = { param($Name, $Package) $script:calls++; New-InstallResult $Name INSTALLED ok }
    Invoke-BootstrapWorkflow -Manifest $manifest -PackageInstaller $installer -DryRun | Out-Null
    Assert-Equal 0 $script:calls
}

Test-Case 'bootstrap dry run creates no log and requests no elevation' {
    $bootstrap = Join-Path $windowsRoot 'bootstrap.ps1'
    $log = Join-Path $env:TEMP ("veloren-whatif-{0}.log" -f [guid]::NewGuid())
    & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $bootstrap -WhatIf -LogPath $log | Out-Null
    Assert-True (-not (Test-Path -LiteralPath $log)) 'Dry run created a log file.'
}
```

- [ ] **Step 2: Run tests to verify workflow functions are undefined**

Run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/tests/run-tests.ps1
```

Expected: exit `1` with undefined elevation and workflow functions.

- [ ] **Step 3: Implement workflow and elevation helpers**

Append to `scripts/windows/Bootstrap.Common.psm1`:

```powershell
function Get-ElevationArguments {
    param(
        [Parameter(Mandatory)][string]$ScriptPath,
        [Parameter(Mandatory)][string]$LogPath,
        [switch]$DryRun
    )
    $arguments = @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass',
        '-File', "`"$ScriptPath`"",
        '-Elevated',
        '-LogPath', "`"$LogPath`""
    )
    if ($DryRun) { $arguments += '-WhatIf' }
    $arguments
}

function Start-ElevatedBootstrap {
    param(
        [Parameter(Mandatory)][string]$ScriptPath,
        [Parameter(Mandatory)][string]$LogPath,
        [switch]$DryRun,
        [scriptblock]$Starter = {
            param($Executable, $Arguments)
            Start-Process -FilePath $Executable -ArgumentList $Arguments -Verb RunAs -Wait -PassThru
        }
    )
    $hostExe = if ($PSVersionTable.PSEdition -eq 'Core') {
        (Get-Command pwsh.exe).Source
    } else {
        (Get-Command powershell.exe).Source
    }
    $process = & $Starter $hostExe (Get-ElevationArguments $ScriptPath $LogPath -DryRun:$DryRun)
    [int]$process.ExitCode
}

function Invoke-BootstrapWorkflow {
    param(
        [Parameter(Mandatory)][hashtable]$Manifest,
        [scriptblock]$PackageInstaller = {
            param($Name, $Package)
            Install-WingetPackage -Name $Name -Package $Package -Confirm:$false
        },
        [scriptblock]$GitLfsInstaller = { Initialize-GitLfs -Confirm:$false },
        [scriptblock]$RustInstaller = { Install-PinnedRustToolchain -Confirm:$false },
        [switch]$DryRun
    )
    if ($DryRun) {
        foreach ($name in $Manifest.Order) {
            New-InstallResult $name SKIPPED 'WhatIf'
        }
        New-InstallResult 'Git LFS initialization' SKIPPED 'WhatIf'
        New-InstallResult 'Pinned Rust toolchain' SKIPPED 'WhatIf'
        return
    }
    $results = @{}
    foreach ($name in $Manifest.Order) {
        $package = $Manifest.Packages[$name]
        $failedDependency = @($package.DependsOn | Where-Object {
            $results.ContainsKey($_) -and $results[$_].Status -eq 'FAILED'
        }) | Select-Object -First 1
        if ($failedDependency) {
            $result = New-InstallResult $name SKIPPED "Dependency failed: $failedDependency"
        } else {
            $result = & $PackageInstaller $name $package
        }
        $results[$name] = $result
        $result
        Refresh-ProcessPath | Out-Null
    }
    & $GitLfsInstaller
    Refresh-ProcessPath | Out-Null
    & $RustInstaller
}

function Get-BootstrapExitCode {
    param([object[]]$InstallResults, [object[]]$DoctorReport)
    if (@($InstallResults | Where-Object Status -eq 'FAILED').Count -gt 0) { return 1 }
    Get-DoctorExitCode $DoctorReport
}
```

- [ ] **Step 4: Create the bootstrap entrypoint**

```powershell
# scripts/windows/bootstrap.ps1
[CmdletBinding(SupportsShouldProcess)]
param(
    [switch]$Elevated,
    [string]$LogPath
)

$ErrorActionPreference = 'Stop'
try {
    Import-Module (Join-Path $PSScriptRoot 'Bootstrap.Common.psm1') -Force
    $root = Get-RepositoryRoot
    if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT -or
        -not [Environment]::Is64BitOperatingSystem -or
        [Environment]::OSVersion.Version.Major -lt 10) {
        Write-Error 'This bootstrap supports only 64-bit Windows 10 and Windows 11.'
        exit 2
    }
    if ($null -eq (Get-Command winget.exe -ErrorAction SilentlyContinue)) {
        Write-Error 'Winget is missing. Install Microsoft App Installer and rerun this command.'
        exit 2
    }
    if (-not $LogPath) { $LogPath = New-BootstrapLogPath }
    if (-not $WhatIfPreference -and -not $Elevated -and -not (Test-IsAdministrator)) {
        exit (Start-ElevatedBootstrap -ScriptPath $PSCommandPath -LogPath $LogPath)
    }
    if (-not $WhatIfPreference) { Write-BootstrapLog $LogPath "Repository: $root" }
    $manifest = Import-PowerShellDataFile (Join-Path $PSScriptRoot 'packages.psd1')
    $packageInstaller = {
        param($Name, $Package)
        Install-WingetPackage -Name $Name -Package $Package -LogPath $LogPath -Confirm:$false
    }.GetNewClosure()
    $gitLfsInstaller = {
        Initialize-GitLfs -LogPath $LogPath -Confirm:$false
    }.GetNewClosure()
    $rustInstaller = {
        Install-PinnedRustToolchain -RepositoryRoot $root -LogPath $LogPath -Confirm:$false
    }.GetNewClosure()
    $installResults = @(
        Invoke-BootstrapWorkflow -Manifest $manifest `
            -PackageInstaller $packageInstaller `
            -GitLfsInstaller $gitLfsInstaller `
            -RustInstaller $rustInstaller `
            -DryRun:$WhatIfPreference
    )
    Refresh-ProcessPath | Out-Null
    $doctorReport = @(Get-DoctorReport -Probes (Get-DefaultDoctorProbes $root))
    if (-not $WhatIfPreference) {
        foreach ($result in $installResults) {
            Write-BootstrapLog $LogPath "Install $($result.Name): $($result.Status) - $($result.Detail)"
        }
        foreach ($check in $doctorReport) {
            Write-BootstrapLog $LogPath "Doctor $($check.Name): $($check.Status) - $($check.Detail)"
        }
    }

    Write-Host 'Veloren Development Environment'
    Write-Host ''
    foreach ($result in $installResults) {
        "{0,-16} {1,-28} {2}" -f $result.Status, $result.Name, $result.Detail | Write-Host
    }
    Write-Host ''
    Write-DoctorTable $doctorReport | Write-Host
    Write-Host ''
    if (Test-PendingRestart) { Write-Warning 'Restart Windows before using the toolchain.' }
    Write-Host 'Reopen the terminal before using Cargo.'
    exit (Get-BootstrapExitCode $installResults $doctorReport)
} catch {
    if ($LogPath -and -not $WhatIfPreference) {
        Write-BootstrapLog $LogPath "Internal error: $($_.Exception.Message)"
    }
    Write-Error $_.Exception.Message
    exit 2
}
```

- [ ] **Step 5: Run workflow and dry-run tests**

Run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/tests/run-tests.ps1
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/bootstrap.ps1 -WhatIf
```

Expected: tests report `Failed: 0`; bootstrap lists only `SKIPPED ... WhatIf`, does not request UAC, and does not create `%LOCALAPPDATA%\VelorenDev\logs`.

- [ ] **Step 6: Commit bootstrap orchestration**

```powershell
git add scripts/windows/bootstrap.ps1 scripts/windows/Bootstrap.Common.psm1 scripts/windows/tests/Bootstrap.Tests.ps1
git commit -m "feat: orchestrate Windows environment bootstrap"
```

---

### Task 5: Documentation and Full Verification

**Files:**
- Create: `scripts/windows/README.md`
- Modify: `scripts/windows/tests/Doctor.Tests.ps1`

**Interfaces:**
- Documents the public interfaces `bootstrap.ps1`, `bootstrap.ps1 -WhatIf`, `doctor.ps1`, and `doctor.ps1 -Json`.
- Adds a source scan that rejects build or run commands from both production scripts.

- [ ] **Step 1: Add a failing scope-guard test**

Append to `scripts/windows/tests/Doctor.Tests.ps1`:

```powershell
Test-Case 'production scripts never compile or run Veloren' {
    $productionFiles = @(
        (Join-Path $windowsRoot 'bootstrap.ps1'),
        (Join-Path $windowsRoot 'doctor.ps1'),
        (Join-Path $windowsRoot 'Bootstrap.Common.psm1')
    )
    foreach ($file in $productionFiles) {
        $content = Get-Content -LiteralPath $file -Raw
        Assert-True ($content -notmatch '(?im)\bcargo\s+(build|check|test|run)\b') "$file invokes Cargo."
        Assert-True ($content -notmatch '(?im)\bveloren-voxygen(?:\.exe)?\b') "$file launches Voxygen."
        Assert-True ($content -notmatch '(?im)\bveloren-server-cli(?:\.exe)?\b') "$file launches the server."
    }
}
```

- [ ] **Step 2: Run the scope guard**

Run:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/tests/run-tests.ps1
```

Expected: `Failed: 0`. If it fails, remove the prohibited production invocation rather than weakening the expressions.

- [ ] **Step 3: Write user documentation**

```markdown
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
```

Save this content as `scripts/windows/README.md`.

- [ ] **Step 4: Run syntax, tests, dry run, and JSON verification**

Run:

```powershell
$files = @(
    'scripts/windows/bootstrap.ps1',
    'scripts/windows/doctor.ps1',
    'scripts/windows/Bootstrap.Common.psm1'
)
$parseErrors = @()
foreach ($file in $files) {
    $tokens = $null
    $errors = $null
    [System.Management.Automation.Language.Parser]::ParseFile(
        (Resolve-Path $file), [ref]$tokens, [ref]$errors
    ) | Out-Null
    $parseErrors += $errors
}
if ($parseErrors.Count -gt 0) { $parseErrors | Format-List; exit 1 }

powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/tests/run-tests.ps1
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/bootstrap.ps1 -WhatIf

$jsonText = powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/doctor.ps1 -Json
$doctorExit = $LASTEXITCODE
$json = ($jsonText -join "`n") | ConvertFrom-Json
if ($doctorExit -notin @(0, 1) -or $json.Checks.Count -eq 0) { exit 1 }
```

Expected:

- parser reports no errors;
- tests report `Failed: 0`;
- dry run requests no elevation, installs nothing, and creates no log;
- doctor emits valid JSON and exits `0` or `1` according to the current machine.

- [ ] **Step 5: Verify the repository diff and commit**

Run:

```powershell
git diff --check
git status --short
```

Expected: only `scripts/windows/README.md` and the planned doctor test change are uncommitted; `git diff --check` exits `0`.

Commit:

```powershell
git add scripts/windows/README.md scripts/windows/tests/Doctor.Tests.ps1
git commit -m "docs: explain Windows environment bootstrap"
```

- [ ] **Step 6: Final acceptance verification**

Run:

```powershell
git status --short
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/tests/run-tests.ps1
powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows/bootstrap.ps1 -WhatIf
```

Expected: clean working tree, `Failed: 0`, and a mutation-free dry run. Do not claim that Veloren compiles because compilation is explicitly outside this bootstrap's scope.
