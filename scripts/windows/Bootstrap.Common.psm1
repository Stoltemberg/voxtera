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

function Resolve-BootstrapLogPath {
    param([Parameter(Mandatory)][string]$Path)
    $logRoot = [System.IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA 'VelorenDev\logs'))
    $normalizedPath = [System.IO.Path]::GetFullPath($Path)
    $logRootPrefix = $logRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar, [System.IO.Path]::AltDirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
    if (-not $normalizedPath.StartsWith($logRootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Bootstrap log path must stay under: $logRoot"
    }
    $normalizedPath
}

function Write-BootstrapLog {
    param([Parameter(Mandatory)][string]$Path, [Parameter(Mandatory)][string]$Message)
    $normalizedPath = Resolve-BootstrapLogPath -Path $Path
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

function Get-PlatformClassification {
    param(
        [Parameter(Mandatory)][PlatformID]$Platform,
        [Parameter(Mandatory)][bool]$Is64BitOperatingSystem,
        [Parameter(Mandatory)][int]$MajorVersion,
        [Parameter(Mandatory)][int]$ProductType
    )
    if ($Platform -ne [PlatformID]::Win32NT) {
        return [pscustomobject][ordered]@{
            Supported = $false
            Detail = 'Only Windows is supported.'
        }
    }
    if (-not $Is64BitOperatingSystem) {
        return [pscustomobject][ordered]@{
            Supported = $false
            Detail = 'Only 64-bit Windows is supported.'
        }
    }
    if ($MajorVersion -ne 10) {
        return [pscustomobject][ordered]@{
            Supported = $false
            Detail = 'Only Windows 10/11 (NT major version 10) is supported.'
        }
    }
    if ($ProductType -ne 1) {
        return [pscustomobject][ordered]@{
            Supported = $false
            Detail = 'Only Windows 10/11 client editions are supported; Windows Server is unsupported.'
        }
    }
    [pscustomobject][ordered]@{
        Supported = $true
        Detail = '64-bit Windows 10/11 client edition.'
    }
}

function Get-PlatformCheck {
    [CmdletBinding()]
    param(
        [PlatformID]$Platform = ([Environment]::OSVersion.Platform),
        [bool]$Is64BitOperatingSystem = ([Environment]::Is64BitOperatingSystem),
        [int]$MajorVersion = ([Environment]::OSVersion.Version.Major),
        [string]$VersionString = ([Environment]::OSVersion.VersionString),
        [scriptblock]$ProductTypeProvider = {
            $operatingSystem = Get-CimInstance `
                -ClassName Win32_OperatingSystem -Property ProductType -ErrorAction Stop |
                Select-Object -First 1
            if ($null -ne $operatingSystem) { $operatingSystem.ProductType }
        }
    )
    $baseClassification = Get-PlatformClassification `
        -Platform $Platform `
        -Is64BitOperatingSystem $Is64BitOperatingSystem `
        -MajorVersion $MajorVersion `
        -ProductType 1
    if (-not $baseClassification.Supported) {
        throw [PlatformNotSupportedException]::new($baseClassification.Detail)
    }
    $productTypes = @(& $ProductTypeProvider)
    if ($productTypes.Count -ne 1 -or $null -eq $productTypes[0]) {
        throw 'Unable to determine the Windows operating system product type.'
    }
    $classification = Get-PlatformClassification `
        -Platform $Platform `
        -Is64BitOperatingSystem $Is64BitOperatingSystem `
        -MajorVersion $MajorVersion `
        -ProductType ([int]$productTypes[0])
    if (-not $classification.Supported) {
        throw [PlatformNotSupportedException]::new($classification.Detail)
    }
    New-CheckResult Platform PASS $VersionString
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

function Get-CommandPresenceCheck {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][string]$Command,
        [scriptblock]$Resolver = {
            param($CommandName)
            Get-Command $CommandName -ErrorAction SilentlyContinue
        }
    )
    $resolved = & $Resolver $Command
    if ($null -eq $resolved) {
        return New-CheckResult $Name FAIL "$Command is missing."
    }
    $path = [string]$resolved.Source
    if ([string]::IsNullOrWhiteSpace($path)) {
        return New-CheckResult $Name FAIL "$Command did not resolve to an executable path."
    }
    New-CheckResult $Name PASS $path
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

function Resolve-RustupPath {
    $rustup = Get-Command rustup.exe -ErrorAction SilentlyContinue
    if ($null -ne $rustup) { $rustup.Source }
}

function Get-RustupCheck {
    Get-CommandCheck Rustup rustup.exe @('--version')
}

function Get-PinnedToolchainCheck {
    param(
        [string]$RepositoryRoot = (Get-RepositoryRoot),
        [string]$RustupPath,
        [scriptblock]$Runner = {
            param($Invocation)
            Invoke-ExternalCommand $Invocation.FilePath $Invocation.Arguments
        }
    )
    $pinned = Get-PinnedToolchain $RepositoryRoot
    if ([string]::IsNullOrWhiteSpace($RustupPath)) {
        $RustupPath = Resolve-RustupPath
    }
    if ([string]::IsNullOrWhiteSpace($RustupPath)) {
        return New-CheckResult 'Pinned Rust toolchain' FAIL "$pinned is not available because rustup.exe is missing."
    }
    $cargo = & $Runner ([pscustomobject][ordered]@{
        FilePath = $RustupPath
        Arguments = @('run', $pinned, 'cargo', '--version')
    })
    $rustc = & $Runner ([pscustomobject][ordered]@{
        FilePath = $RustupPath
        Arguments = @('run', $pinned, 'rustc', '--version')
    })
    $failures = @()
    if ($cargo.ExitCode -ne 0) {
        $cargoDetail = (($cargo.Output -join ' ').Trim())
        if ([string]::IsNullOrWhiteSpace($cargoDetail)) { $cargoDetail = "exit $($cargo.ExitCode)" }
        $failures += "cargo: $cargoDetail"
    }
    if ($rustc.ExitCode -ne 0) {
        $rustcDetail = (($rustc.Output -join ' ').Trim())
        if ([string]::IsNullOrWhiteSpace($rustcDetail)) { $rustcDetail = "exit $($rustc.ExitCode)" }
        $failures += "rustc: $rustcDetail"
    }
    if ($failures.Count -gt 0) {
        return New-CheckResult 'Pinned Rust toolchain' FAIL ($failures -join '; ')
    }
    $versions = (@($cargo.Output) + @($rustc.Output)) -join ' '
    New-CheckResult 'Pinned Rust toolchain' PASS "$pinned; $($versions.Trim())"
}

function Get-RustComponentsCheck {
    param(
        [string]$RepositoryRoot = (Get-RepositoryRoot),
        [string]$RustupPath,
        [scriptblock]$Runner = {
            param($Invocation)
            Invoke-ExternalCommand $Invocation.FilePath $Invocation.Arguments
        }
    )
    $pinned = Get-PinnedToolchain $RepositoryRoot
    if ([string]::IsNullOrWhiteSpace($RustupPath)) {
        $RustupPath = Resolve-RustupPath
    }
    if ([string]::IsNullOrWhiteSpace($RustupPath)) {
        return New-CheckResult 'Rust components' FAIL 'rustfmt and clippy are not available because rustup.exe is missing.'
    }
    $components = & $Runner ([pscustomobject][ordered]@{
        FilePath = $RustupPath
        Arguments = @('component', 'list', '--toolchain', $pinned, '--installed')
    })
    $componentText = ($components.Output -join "`n")
    if ($components.ExitCode -ne 0) {
        $detail = $componentText.Trim()
        if ([string]::IsNullOrWhiteSpace($detail)) { $detail = "rustup exited with $($components.ExitCode)." }
        return New-CheckResult 'Rust components' FAIL $detail
    }
    if ($componentText -notmatch 'rustfmt' -or $componentText -notmatch 'clippy') {
        return New-CheckResult 'Rust components' FAIL 'rustfmt and clippy are not both installed.'
    }
    New-CheckResult 'Rust components' PASS 'rustfmt, clippy'
}

function Get-DefaultDoctorProbes {
    param(
        [string]$RepositoryRoot = (Get-RepositoryRoot),
        [scriptblock]$CommandResolver = {
            param($CommandName)
            Get-Command $CommandName -ErrorAction SilentlyContinue
        }
    )
    $root = $RepositoryRoot
    $resolver = $CommandResolver
    [ordered]@{
        Platform = { Get-PlatformCheck }
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
        Cargo = ({ Get-CommandPresenceCheck Cargo cargo.exe $resolver }).GetNewClosure()
        Rustup = { Get-RustupCheck }
        PinnedToolchain = ({ Get-PinnedToolchainCheck $root }).GetNewClosure()
        RustComponents = ({ Get-RustComponentsCheck $root }).GetNewClosure()
        Restart = {
            New-CheckResult Restart $(if (Test-PendingRestart) { 'WARN' } else { 'PASS' }) `
                $(if (Test-PendingRestart) { 'Windows restart is pending.' } else { 'No restart is pending.' })
        }
    }
}

function Get-DoctorReport {
    param([Parameter(Mandatory)][System.Collections.IDictionary]$Probes)
    foreach ($entry in $Probes.GetEnumerator()) {
        $result = @(& $entry.Value)
        if ($result.Count -ne 1) {
            throw "Doctor probe '$($entry.Key)' returned $($result.Count) results; expected exactly one."
        }
        $result[0]
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

function New-InstallResult {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)]
        [ValidateSet('INSTALLED', 'ALREADY PRESENT', 'FAILED', 'SKIPPED')]
        [string]$Status,
        [Parameter(Mandatory)][string]$Detail
    )
    [pscustomobject][ordered]@{
        Name = $Name
        Status = $Status
        Detail = $Detail
    }
}

function Test-PackagePresent {
    param(
        [Parameter(Mandatory)][hashtable]$Package,
        [scriptblock]$CommandResolver = {
            param($Command)
            Get-Command $Command -ErrorAction SilentlyContinue
        }
    )
    if (-not [string]::IsNullOrWhiteSpace($Package.Command)) {
        return $null -ne (& $CommandResolver $Package.Command)
    }
    if ($Package.Id -eq 'Microsoft.VisualStudio.2022.BuildTools') {
        return (Get-VisualStudioCheck).Status -eq 'PASS'
    }
    $false
}

function Test-PinnedRustToolchainListed {
    param(
        [Parameter(Mandatory)][string]$Pinned,
        [string[]]$Entries = @()
    )
    $pattern = '^{0}(?:-(?:x86_64|aarch64)-pc-windows-(?:msvc|gnu))?$' -f
        [regex]::Escape($Pinned)
    foreach ($entry in @($Entries)) {
        foreach ($line in @([string]$entry -split "\r?\n")) {
            $name = [regex]::Replace($line.Trim(), '\s+\([^)]*\)\s*$', '')
            if ([regex]::IsMatch($name, $pattern)) {
                return $true
            }
        }
    }
    $false
}

function Install-WingetPackage {
    [CmdletBinding(SupportsShouldProcess)]
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][hashtable]$Package,
        [scriptblock]$Detector = {
            param($Value)
            Test-PackagePresent $Value
        },
        [scriptblock]$Runner = {
            param($FilePath, $Arguments)
            Invoke-ExternalCommand $FilePath $Arguments
        },
        [string]$LogPath
    )
    $resolvedLogPath = $null
    if (-not [string]::IsNullOrWhiteSpace($LogPath)) {
        $resolvedLogPath = Resolve-BootstrapLogPath -Path $LogPath
    }
    if (& $Detector $Package) {
        return New-InstallResult -Name $Name -Status 'ALREADY PRESENT' -Detail $Package.Id
    }
    if (-not $PSCmdlet.ShouldProcess($Package.Id, 'Install Winget package')) {
        return New-InstallResult -Name $Name -Status 'SKIPPED' -Detail 'WhatIf'
    }

    $arguments = @(
        'install',
        '--id', $Package.Id,
        '--exact',
        '--accept-package-agreements',
        '--accept-source-agreements',
        '--disable-interactivity'
    ) + @($Package.WingetArguments)

    for ($attempt = 1; $attempt -le 2; $attempt++) {
        $result = & $Runner 'winget.exe' $arguments
        if ($null -ne $resolvedLogPath) {
            Write-BootstrapLog -Path $resolvedLogPath `
                -Message "$($result.Command) => $($result.ExitCode)"
        }
        if ($result.ExitCode -in @(0, 1641, 3010)) {
            $detail = if ($result.ExitCode -eq 0) {
                $Package.Id
            } else {
                "$($Package.Id); restart required"
            }
            return New-InstallResult -Name $Name -Status 'INSTALLED' -Detail $detail
        }
    }

    $detail = (($result.Output -join ' ').Trim())
    if ([string]::IsNullOrWhiteSpace($detail)) {
        $detail = "winget.exe exited with code $($result.ExitCode)."
    }
    New-InstallResult -Name $Name -Status 'FAILED' -Detail $detail
}

function Initialize-GitLfs {
    [CmdletBinding(SupportsShouldProcess)]
    param(
        [bool]$GitLfsAvailable = ($null -ne (Get-Command git-lfs.exe -ErrorAction SilentlyContinue)),
        [scriptblock]$Runner = {
            param($FilePath, $Arguments)
            Invoke-ExternalCommand $FilePath $Arguments
        },
        [string]$LogPath
    )
    $resolvedLogPath = $null
    if (-not [string]::IsNullOrWhiteSpace($LogPath)) {
        $resolvedLogPath = Resolve-BootstrapLogPath -Path $LogPath
    }
    if (-not $GitLfsAvailable) {
        return New-InstallResult `
            -Name 'Git LFS initialization' `
            -Status 'SKIPPED' `
            -Detail 'Git LFS is unavailable.'
    }
    if (-not $PSCmdlet.ShouldProcess('Git LFS', 'Initialize user configuration')) {
        return New-InstallResult `
            -Name 'Git LFS initialization' `
            -Status 'SKIPPED' `
            -Detail 'WhatIf'
    }

    $result = & $Runner 'git.exe' @('lfs', 'install')
    if ($null -ne $resolvedLogPath) {
        Write-BootstrapLog -Path $resolvedLogPath `
            -Message "$($result.Command) => $($result.ExitCode)"
    }
    if ($result.ExitCode -eq 0) {
        return New-InstallResult `
            -Name 'Git LFS initialization' `
            -Status 'INSTALLED' `
            -Detail 'git lfs install'
    }

    $detail = (($result.Output -join ' ').Trim())
    if ([string]::IsNullOrWhiteSpace($detail)) {
        $detail = "git.exe lfs install exited with code $($result.ExitCode)."
    }
    New-InstallResult -Name 'Git LFS initialization' -Status 'FAILED' -Detail $detail
}

function Install-PinnedRustToolchain {
    [CmdletBinding(SupportsShouldProcess)]
    param(
        [string]$RepositoryRoot = (Get-RepositoryRoot),
        [bool]$RustupAvailable = ($null -ne (Get-Command rustup.exe -ErrorAction SilentlyContinue)),
        [scriptblock]$Runner = {
            param($FilePath, $Arguments)
            Invoke-ExternalCommand $FilePath $Arguments
        },
        [string]$LogPath
    )
    $resolvedLogPath = $null
    if (-not [string]::IsNullOrWhiteSpace($LogPath)) {
        $resolvedLogPath = Resolve-BootstrapLogPath -Path $LogPath
    }
    if (-not $RustupAvailable) {
        return New-InstallResult `
            -Name 'Pinned Rust toolchain' `
            -Status 'SKIPPED' `
            -Detail 'Rustup is unavailable.'
    }

    $pinned = Get-PinnedToolchain $RepositoryRoot
    if (-not $PSCmdlet.ShouldProcess($pinned, 'Install Rust toolchain and components')) {
        return New-InstallResult `
            -Name 'Pinned Rust toolchain' `
            -Status 'SKIPPED' `
            -Detail 'WhatIf'
    }

    $listed = & $Runner 'rustup.exe' @('toolchain', 'list')
    if ($listed.ExitCode -ne 0) {
        $detail = (($listed.Output -join ' ').Trim())
        if ([string]::IsNullOrWhiteSpace($detail)) {
            $detail = "rustup.exe toolchain list exited with code $($listed.ExitCode)."
        }
        return New-InstallResult -Name 'Pinned Rust toolchain' -Status 'FAILED' -Detail $detail
    }

    $toolchainPresent = Test-PinnedRustToolchainListed -Pinned $pinned -Entries @($listed.Output)
    $missingComponents = @('rustfmt', 'clippy')
    if ($toolchainPresent) {
        $components = & $Runner 'rustup.exe' @(
            'component', 'list',
            '--toolchain', $pinned,
            '--installed'
        )
        if ($components.ExitCode -ne 0) {
            $detail = (($components.Output -join ' ').Trim())
            if ([string]::IsNullOrWhiteSpace($detail)) {
                $detail = "rustup.exe component list exited with code $($components.ExitCode)."
            }
            return New-InstallResult -Name 'Pinned Rust toolchain' -Status 'FAILED' -Detail $detail
        }

        $componentText = ($components.Output -join "`n")
        $missingComponents = @(
            @('rustfmt', 'clippy') | Where-Object {
                $componentText -notmatch "(?m)^$([regex]::Escape($_))(?:-|$)"
            }
        )
        if ($missingComponents.Count -eq 0) {
            return New-InstallResult `
                -Name 'Pinned Rust toolchain' `
                -Status 'ALREADY PRESENT' `
                -Detail $pinned
        }
    }

    if ($toolchainPresent) {
        $arguments = @('component', 'add', '--toolchain', $pinned) + $missingComponents
        $successDetail = "$pinned; added $($missingComponents -join ', ')"
        $failureDescription = 'component add'
    } else {
        $arguments = @(
            'toolchain', 'install', $pinned,
            '--profile', 'minimal',
            '--component', 'rustfmt',
            '--component', 'clippy'
        )
        $successDetail = $pinned
        $failureDescription = 'toolchain install'
    }

    $result = & $Runner 'rustup.exe' $arguments
    if ($null -ne $resolvedLogPath) {
        Write-BootstrapLog -Path $resolvedLogPath `
            -Message "$($result.Command) => $($result.ExitCode)"
    }
    if ($result.ExitCode -eq 0) {
        return New-InstallResult `
            -Name 'Pinned Rust toolchain' `
            -Status 'INSTALLED' `
            -Detail $successDetail
    }

    $detail = (($result.Output -join ' ').Trim())
    if ([string]::IsNullOrWhiteSpace($detail)) {
        $detail = "rustup.exe $failureDescription exited with code $($result.ExitCode)."
    }
    New-InstallResult -Name 'Pinned Rust toolchain' -Status 'FAILED' -Detail $detail
}

Export-ModuleMember -Function *
