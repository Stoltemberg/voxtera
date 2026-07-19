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
    $logRoot = [System.IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'VelorenDev\logs'))
    $normalizedPath = [System.IO.Path]::GetFullPath($Path)
    $logRootPrefix = $logRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
    if (-not $normalizedPath.StartsWith($logRootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Bootstrap log path must stay under: $logRoot"
    }
    $directory = Split-Path -Parent $normalizedPath
    if (-not (Test-Path -LiteralPath $directory)) {
        New-Item -ItemType Directory -Path $directory -Force | Out-Null
    }
    Add-Content -LiteralPath $normalizedPath -Value ("[{0:o}] {1}" -f (Get-Date), $Message)
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
        try {
            @(& $entry.Value) | ForEach-Object { $_ }
        } catch {
            New-CheckResult ([string]$entry.Key) FAIL $_.Exception.Message
        }
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

Export-ModuleMember -Function *
