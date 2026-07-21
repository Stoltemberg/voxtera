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
    $local:ErrorActionPreference = 'Continue'
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

function Resolve-VswherePath {
    Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
}

function Get-VisualStudioInstallationPath {
    param(
        [string]$VswherePath,
        [scriptblock]$Runner = {
            param($FilePath, $Arguments)
            Invoke-ExternalCommand $FilePath $Arguments
        }
    )
    if ([string]::IsNullOrWhiteSpace($VswherePath)) {
        $VswherePath = Resolve-VswherePath
        if (-not (Test-Path -LiteralPath $VswherePath)) {
            return $null
        }
    }
    $result = & $Runner $VswherePath @(
        '-latest',
        '-products', 'Microsoft.VisualStudio.Product.BuildTools',
        '-version', '[17.0,18.0)',
        '-property', 'installationPath'
    )
    if ($result.ExitCode -ne 0) {
        $detail = (($result.Output -join ' ').Trim())
        if ([string]::IsNullOrWhiteSpace($detail)) {
            $detail = "vswhere.exe exited with code $($result.ExitCode)."
        }
        throw "Unable to query Visual Studio Build Tools identity: $detail"
    }
    $path = [string]($result.Output | Where-Object {
        -not [string]::IsNullOrWhiteSpace([string]$_)
    } | Select-Object -First 1)
    if ([string]::IsNullOrWhiteSpace($path) -or
        -not [System.IO.Path]::IsPathRooted($path)) {
        return $null
    }
    $path.Trim()
}

function Get-VisualStudioCheck {
    param(
        [string]$VswherePath,
        [scriptblock]$Runner = {
            param($FilePath, $Arguments)
            Invoke-ExternalCommand $FilePath $Arguments
        }
    )
    if ([string]::IsNullOrWhiteSpace($VswherePath)) {
        $VswherePath = Resolve-VswherePath
    }
    if (-not (Test-Path -LiteralPath $VswherePath) -and
        $PSBoundParameters.ContainsKey('VswherePath') -eq $false) {
        return New-CheckResult 'Visual Studio Build Tools' FAIL 'vswhere.exe is missing.'
    }
    $result = & $Runner $VswherePath @(
        '-latest',
        '-products', 'Microsoft.VisualStudio.Product.BuildTools',
        '-version', '[17.0,18.0)',
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

function Get-RustupDefaultHostResult {
    param([Parameter(Mandatory)]$CommandResult)
    $output = (($CommandResult.Output -join "`n").Trim())
    if ($CommandResult.ExitCode -ne 0) {
        if ([string]::IsNullOrWhiteSpace($output)) {
            $output = "rustup.exe show exited with code $($CommandResult.ExitCode)."
        }
        return [pscustomobject][ordered]@{
            Success = $false
            Host = $null
            Detail = $output
        }
    }
    $match = [regex]::Match(
        $output,
        '(?im)^Default host:\s*((?:x86_64|aarch64)-pc-windows-(?:msvc|gnu))\s*$'
    )
    if (-not $match.Success) {
        return [pscustomobject][ordered]@{
            Success = $false
            Host = $null
            Detail = 'rustup.exe show did not report a supported Windows default host.'
        }
    }
    [pscustomobject][ordered]@{
        Success = $true
        Host = $match.Groups[1].Value
        Detail = $match.Groups[1].Value
    }
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
    $show = & $Runner ([pscustomobject][ordered]@{
        FilePath = $RustupPath
        Arguments = @('show')
    })
    $defaultHost = Get-RustupDefaultHostResult -CommandResult $show
    if (-not $defaultHost.Success) {
        return New-CheckResult 'Pinned Rust toolchain' FAIL $defaultHost.Detail
    }
    $qualified = "$pinned-$($defaultHost.Host)"
    $cargo = & $Runner ([pscustomobject][ordered]@{
        FilePath = $RustupPath
        Arguments = @('run', $qualified, 'cargo', '--version')
    })
    $rustc = & $Runner ([pscustomobject][ordered]@{
        FilePath = $RustupPath
        Arguments = @('run', $qualified, 'rustc', '--version')
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
    New-CheckResult 'Pinned Rust toolchain' PASS "$qualified; $($versions.Trim())"
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
    $show = & $Runner ([pscustomobject][ordered]@{
        FilePath = $RustupPath
        Arguments = @('show')
    })
    $defaultHost = Get-RustupDefaultHostResult -CommandResult $show
    if (-not $defaultHost.Success) {
        return New-CheckResult 'Rust components' FAIL $defaultHost.Detail
    }
    $qualified = "$pinned-$($defaultHost.Host)"
    $components = & $Runner ([pscustomobject][ordered]@{
        FilePath = $RustupPath
        Arguments = @('component', 'list', '--toolchain', $qualified, '--installed')
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

function Get-ManagedPackageCommandCheck {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][hashtable]$Package,
        [scriptblock]$CommandResolver = {
            param($Command)
            Get-Command $Command -ErrorAction SilentlyContinue
        },
        [scriptblock]$Runner = {
            param($FilePath, $Arguments)
            Invoke-ExternalCommand $FilePath $Arguments
        }
    )
    $resolved = & $CommandResolver $Package.Command
    if ($null -eq $resolved) {
        return New-CheckResult $Name FAIL "$($Package.Command) is missing."
    }
    $path = [string]$resolved.Source
    if ([string]::IsNullOrWhiteSpace($path)) {
        return New-CheckResult $Name FAIL "$($Package.Command) did not resolve to an executable path."
    }
    if ($path -match '(?i)[\\/]Microsoft[\\/]WindowsApps[\\/]') {
        return New-CheckResult $Name FAIL "Microsoft Store execution alias is unusable: $path"
    }
    $versionArguments = if ($Package.ContainsKey('VersionArguments')) {
        @($Package.VersionArguments)
    } else {
        @('--version')
    }
    try {
        $result = & $Runner $path $versionArguments
    } catch {
        return New-CheckResult $Name FAIL $_.Exception.Message
    }
    $version = (($result.Output -join "`n").Trim())
    if ($result.ExitCode -ne 0) {
        if ([string]::IsNullOrWhiteSpace($version)) {
            $version = "$($Package.Command) exited with code $($result.ExitCode)."
        }
        return New-CheckResult $Name FAIL $version
    }
    if ([string]::IsNullOrWhiteSpace($version)) {
        return New-CheckResult $Name FAIL "$($Package.Command) returned no version."
    }
    if ($Package.ContainsKey('VersionPattern') -and
        $version -notmatch $Package.VersionPattern) {
        return New-CheckResult `
            -Name $Name `
            -Status FAIL `
            -Detail "Unexpected version: $version"
    }
    New-CheckResult $Name PASS $version
}

function Get-DefaultDoctorProbes {
    param(
        [string]$RepositoryRoot = (Get-RepositoryRoot),
        [scriptblock]$CommandResolver = {
            param($CommandName)
            Get-Command $CommandName -ErrorAction SilentlyContinue
        },
        [scriptblock]$CommandRunner = {
            param($FilePath, $Arguments)
            Invoke-ExternalCommand $FilePath $Arguments
        }
    )
    $root = $RepositoryRoot
    $resolver = $CommandResolver
    $runner = $CommandRunner
    $manifest = Import-PowerShellDataFile (Join-Path $PSScriptRoot 'packages.psd1')
    $pythonPackage = $manifest.Packages.Python
    [ordered]@{
        Platform = { Get-PlatformCheck }
        Winget = { Get-CommandCheck Winget winget.exe @('--version') }
        Git = { Get-CommandCheck Git git.exe @('--version') }
        GitLfs = { Get-CommandCheck 'Git LFS' git-lfs.exe @('--version') }
        GitLfsConfig = ({
            Get-GitLfsConfigCheck `
                -RepositoryRoot $root `
                -CommandResolver $resolver `
                -Runner $runner
        }).GetNewClosure()
        Assets = ({ Get-AssetCheck $root }).GetNewClosure()
        VisualStudio = { Get-VisualStudioCheck }
        WindowsSdk = { Get-WindowsSdkCheck }
        CMake = { Get-CommandCheck CMake cmake.exe @('--version') }
        Ninja = { Get-CommandCheck Ninja ninja.exe @('--version') }
        Python = ({
            Get-ManagedPackageCommandCheck `
                -Name Python `
                -Package $pythonPackage `
                -CommandResolver $resolver `
                -Runner $runner
        }).GetNewClosure()
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

function Get-GitLfsConfigCheck {
    param(
        [string]$RepositoryRoot = (Get-RepositoryRoot),
        [scriptblock]$CommandResolver = {
            param($CommandName)
            Get-Command $CommandName -ErrorAction SilentlyContinue
        },
        [scriptblock]$Runner = {
            param($FilePath, $Arguments)
            Invoke-ExternalCommand $FilePath $Arguments
        }
    )
    $root = Get-RepositoryRoot -StartPath $RepositoryRoot
    $git = & $CommandResolver 'git.exe'
    if ($null -eq $git -or
        [string]::IsNullOrWhiteSpace([string]$git.Source)) {
        return New-CheckResult `
            -Name 'Git LFS configuration' `
            -Status FAIL `
            -Detail 'Git is unavailable.'
    }
    $gitPath = [string]$git.Source
    $expected = [ordered]@{
        'filter.lfs.clean' = 'git-lfs clean -- %f'
        'filter.lfs.smudge' = 'git-lfs smudge -- %f'
        'filter.lfs.process' = 'git-lfs filter-process'
        'filter.lfs.required' = 'true'
    }
    $failures = New-Object System.Collections.Generic.List[string]
    foreach ($entry in $expected.GetEnumerator()) {
        $result = & $Runner $gitPath @(
            '-C', $root,
            'config', '--get', $entry.Key
        )
        $actual = (($result.Output -join "`n").Trim())
        if ($result.ExitCode -ne 0 -or
            -not [string]::Equals(
                $actual,
                $entry.Value,
                [System.StringComparison]::OrdinalIgnoreCase
            )) {
            $failures.Add($entry.Key) | Out-Null
        }
    }
    if ($failures.Count -gt 0) {
        return New-CheckResult `
            -Name 'Git LFS configuration' `
            -Status FAIL `
            -Detail "Missing or ineffective values: $($failures -join ', ')."
    }
    New-CheckResult `
        -Name 'Git LFS configuration' `
        -Status PASS `
        -Detail 'Effective Git LFS filters are configured.'
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
        },
        [scriptblock]$Runner = {
            param($FilePath, $Arguments)
            Invoke-ExternalCommand $FilePath $Arguments
        },
        [scriptblock]$VisualStudioCheck = { Get-VisualStudioCheck },
        [scriptblock]$WindowsSdkCheck = { Get-WindowsSdkCheck }
    )
    if (-not [string]::IsNullOrWhiteSpace($Package.Command)) {
        $check = Get-ManagedPackageCommandCheck `
            -Name $Package.Id `
            -Package $Package `
            -CommandResolver $CommandResolver `
            -Runner $Runner
        return $check.Status -eq 'PASS'
    }
    if ($Package.Id -eq 'Microsoft.VisualStudio.2022.BuildTools') {
        return (& $VisualStudioCheck).Status -eq 'PASS' -and
            (& $WindowsSdkCheck).Status -eq 'PASS'
    }
    $false
}

function Test-PinnedRustToolchainListed {
    param(
        [Parameter(Mandatory)][string]$Pinned,
        [Parameter(Mandatory)][string]$DefaultHost,
        [string[]]$Entries = @()
    )
    $qualified = "$Pinned-$DefaultHost"
    foreach ($entry in @($Entries)) {
        foreach ($line in @([string]$entry -split "\r?\n")) {
            $name = [regex]::Replace($line.Trim(), '\s+\([^)]*\)\s*$', '')
            if ([string]::Equals(
                $name,
                $qualified,
                [System.StringComparison]::Ordinal
            )) {
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
        '--source', 'winget',
        '--no-upgrade',
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
        if ($result.ExitCode -eq 1641) {
            return New-InstallResult `
                -Name $Name `
                -Status 'FAILED' `
                -Detail "$($Package.Id); restart initiated unexpectedly; automatic reboot is forbidden."
        }
        if ($result.ExitCode -in @(0, 3010)) {
            $detail = if ($result.ExitCode -eq 0) {
                $Package.Id
            } else {
                "$($Package.Id); restart required"
            }
            return New-InstallResult -Name $Name -Status 'INSTALLED' -Detail $detail
        }
        if ($result.ExitCode -ne 1618) {
            break
        }
    }

    $detail = (($result.Output -join ' ').Trim())
    if ([string]::IsNullOrWhiteSpace($detail)) {
        $detail = "winget.exe exited with code $($result.ExitCode)."
    }
    New-InstallResult -Name $Name -Status 'FAILED' -Detail $detail
}

function Install-VisualStudioBuildTools {
    [CmdletBinding(SupportsShouldProcess)]
    param(
        [Parameter(Mandatory)][hashtable]$Package,
        [scriptblock]$InstallationPathResolver = {
            Get-VisualStudioInstallationPath
        },
        [scriptblock]$VisualStudioCheck = { Get-VisualStudioCheck },
        [scriptblock]$WindowsSdkCheck = { Get-WindowsSdkCheck },
        [string]$InstallerPath,
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

    $installationPath = [string](& $InstallationPathResolver)
    if (-not [string]::IsNullOrWhiteSpace($installationPath)) {
        if (-not [System.IO.Path]::IsPathRooted($installationPath)) {
            return New-InstallResult `
                -Name 'VisualStudio' `
                -Status FAILED `
                -Detail 'Visual Studio installation path is not absolute.'
        }
        $workloadReady = (& $VisualStudioCheck).Status -eq 'PASS'
        $sdkReady = (& $WindowsSdkCheck).Status -eq 'PASS'
        if ($workloadReady -and $sdkReady) {
            return New-InstallResult `
                -Name 'VisualStudio' `
                -Status 'ALREADY PRESENT' `
                -Detail $installationPath
        }
    }

    $action = if ([string]::IsNullOrWhiteSpace($installationPath)) {
        'Install Visual Studio Build Tools'
    } else {
        'Add missing Visual Studio workload components'
    }
    $target = if ([string]::IsNullOrWhiteSpace($installationPath)) {
        $Package.Id
    } else {
        $installationPath
    }
    if (-not $PSCmdlet.ShouldProcess($target, $action)) {
        return New-InstallResult `
            -Name 'VisualStudio' `
            -Status 'SKIPPED' `
            -Detail 'WhatIf'
    }

    if ([string]::IsNullOrWhiteSpace($installationPath)) {
        return Install-WingetPackage `
            -Name 'VisualStudio' `
            -Package $Package `
            -Detector { param($Value) $false } `
            -Runner $Runner `
            -LogPath $resolvedLogPath `
            -Confirm:$false
    }

    if ([string]::IsNullOrWhiteSpace($InstallerPath)) {
        $InstallerPath = Join-Path ${env:ProgramFiles(x86)} `
            'Microsoft Visual Studio\Installer\setup.exe'
        if (-not (Test-Path -LiteralPath $InstallerPath)) {
            return New-InstallResult `
                -Name 'VisualStudio' `
                -Status FAILED `
                -Detail 'Visual Studio Installer setup.exe is missing.'
        }
    }
    $arguments = @(
        'modify',
        '--installPath', $installationPath,
        '--add', 'Microsoft.VisualStudio.Workload.VCTools',
        '--includeRecommended',
        '--passive',
        '--norestart'
    )
    for ($attempt = 1; $attempt -le 2; $attempt++) {
        $result = & $Runner $InstallerPath $arguments
        if ($null -ne $resolvedLogPath) {
            Write-BootstrapLog -Path $resolvedLogPath `
                -Message "$($result.Command) => $($result.ExitCode)"
        }
        if ($result.ExitCode -eq 1641) {
            return New-InstallResult `
                -Name 'VisualStudio' `
                -Status FAILED `
                -Detail 'Visual Studio Installer initiated a restart unexpectedly; automatic reboot is forbidden.'
        }
        if ($result.ExitCode -in @(0, 3010)) {
            $detail = if ($result.ExitCode -eq 3010) {
                "$installationPath; restart required"
            } else {
                $installationPath
            }
            return New-InstallResult `
                -Name 'VisualStudio' `
                -Status INSTALLED `
                -Detail $detail
        }
        if ($result.ExitCode -ne 1618) {
            break
        }
    }

    $detail = (($result.Output -join ' ').Trim())
    if ([string]::IsNullOrWhiteSpace($detail)) {
        $detail = "Visual Studio Installer exited with code $($result.ExitCode)."
    }
    New-InstallResult -Name 'VisualStudio' -Status FAILED -Detail $detail
}

function Install-ManifestPackage {
    param(
        [Parameter(Mandatory)][string]$Name,
        [Parameter(Mandatory)][hashtable]$Package,
        [Parameter(Mandatory)][scriptblock]$Detector,
        [Parameter(Mandatory)][scriptblock]$Runner,
        [string]$LogPath,
        [scriptblock]$VisualStudioInstaller = {
            param($Value, $CommandRunner, $BootstrapLogPath)
            Install-VisualStudioBuildTools `
                -Package $Value `
                -Runner $CommandRunner `
                -LogPath $BootstrapLogPath `
                -Confirm:$false
        },
        [scriptblock]$WingetInstaller = {
            param($PackageName, $Value, $PackageDetector, $CommandRunner, $BootstrapLogPath)
            Install-WingetPackage `
                -Name $PackageName `
                -Package $Value `
                -Detector $PackageDetector `
                -Runner $CommandRunner `
                -LogPath $BootstrapLogPath `
                -Confirm:$false
        }
    )
    if ($Package.Id -eq 'Microsoft.VisualStudio.2022.BuildTools') {
        return & $VisualStudioInstaller $Package $Runner $LogPath
    }
    & $WingetInstaller $Name $Package $Detector $Runner $LogPath
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

    $show = & $Runner 'rustup.exe' @('show')
    $defaultHost = Get-RustupDefaultHostResult -CommandResult $show
    if (-not $defaultHost.Success) {
        return New-InstallResult `
            -Name 'Pinned Rust toolchain' `
            -Status FAILED `
            -Detail $defaultHost.Detail
    }
    $qualified = "$pinned-$($defaultHost.Host)"

    $listed = & $Runner 'rustup.exe' @('toolchain', 'list')
    if ($listed.ExitCode -ne 0) {
        $detail = (($listed.Output -join ' ').Trim())
        if ([string]::IsNullOrWhiteSpace($detail)) {
            $detail = "rustup.exe toolchain list exited with code $($listed.ExitCode)."
        }
        return New-InstallResult -Name 'Pinned Rust toolchain' -Status 'FAILED' -Detail $detail
    }

    $toolchainPresent = Test-PinnedRustToolchainListed `
        -Pinned $pinned `
        -DefaultHost $defaultHost.Host `
        -Entries @($listed.Output)
    $missingComponents = @('rustfmt', 'clippy')
    if ($toolchainPresent) {
        $components = & $Runner 'rustup.exe' @(
            'component', 'list',
            '--toolchain', $qualified,
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
                -Detail $qualified
        }
    }

    if ($toolchainPresent) {
        $arguments = @('component', 'add', '--toolchain', $qualified) + $missingComponents
        $successDetail = "$qualified; added $($missingComponents -join ', ')"
        $failureDescription = 'component add'
    } else {
        $arguments = @(
            'toolchain', 'install', $qualified,
            '--profile', 'minimal',
            '--component', 'rustfmt',
            '--component', 'clippy'
        )
        $successDetail = $qualified
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

function Get-ElevationArguments {
    param(
        [Parameter(Mandatory)][ValidateNotNullOrEmpty()][string]$ScriptPath,
        [Parameter(Mandatory)][ValidateNotNullOrEmpty()][string]$LogPath,
        [switch]$DryRun
    )
    if ($ScriptPath.Contains('"') -or $LogPath.Contains('"')) {
        throw 'Bootstrap script and log paths cannot contain double quote characters.'
    }

    $arguments = @(
        '-NoProfile',
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        "`"$ScriptPath`"",
        '-Elevated',
        '-LogPath',
        "`"$LogPath`""
    )
    if ($DryRun) { $arguments += '-WhatIf' }
    $arguments
}

function Start-ElevatedBootstrap {
    param(
        [Parameter(Mandatory)][ValidateNotNullOrEmpty()][string]$ScriptPath,
        [Parameter(Mandatory)][ValidateNotNullOrEmpty()][string]$LogPath,
        [switch]$DryRun,
        [scriptblock]$Starter = {
            param($Executable, $Arguments)
            Start-Process `
                -FilePath $Executable `
                -ArgumentList $Arguments `
                -Verb RunAs `
                -Wait `
                -PassThru
        }
    )
    $hostExecutable = if ($PSVersionTable.PSEdition -eq 'Core') {
        (Get-Command pwsh.exe -ErrorAction Stop).Source
    } else {
        (Get-Command powershell.exe -ErrorAction Stop).Source
    }
    $arguments = @(Get-ElevationArguments `
        -ScriptPath $ScriptPath `
        -LogPath $LogPath `
        -DryRun:$DryRun)
    $process = & $Starter $hostExecutable $arguments
    [int]$process.ExitCode
}

function Invoke-BootstrapWorkflow {
    param(
        [Parameter(Mandatory)][System.Collections.IDictionary]$Manifest,
        [scriptblock]$PackageDetector = {
            param($Package)
            Test-PackagePresent -Package $Package
        },
        [scriptblock]$PackageRunner = {
            param($FilePath, $Arguments)
            Invoke-ExternalCommand -FilePath $FilePath -Arguments $Arguments
        },
        [scriptblock]$PackageInstaller = {
            param($Name, $Package, $Detector, $Runner)
            Install-ManifestPackage `
                -Name $Name `
                -Package $Package `
                -Detector $Detector `
                -Runner $Runner
        },
        [scriptblock]$GitLfsRunner = {
            param($FilePath, $Arguments)
            Invoke-ExternalCommand -FilePath $FilePath -Arguments $Arguments
        },
        [scriptblock]$GitLfsInstaller = {
            param($Runner)
            Initialize-GitLfs -Runner $Runner -Confirm:$false
        },
        [scriptblock]$RustRunner = {
            param($FilePath, $Arguments)
            Invoke-ExternalCommand -FilePath $FilePath -Arguments $Arguments
        },
        [scriptblock]$RustInstaller = {
            param($Runner)
            Install-PinnedRustToolchain -Runner $Runner -Confirm:$false
        },
        [scriptblock]$PathRefresher = {
            Refresh-ProcessPath
        },
        [Parameter(Mandatory)][scriptblock]$Doctor,
        [switch]$DryRun
    )
    $installResults = New-Object System.Collections.Generic.List[object]

    if ($DryRun) {
        foreach ($name in $Manifest.Order) {
            $installResults.Add(
                (New-InstallResult -Name $name -Status 'SKIPPED' -Detail 'WhatIf')
            ) | Out-Null
        }
        $installResults.Add(
            (New-InstallResult `
                -Name 'Git LFS initialization' `
                -Status 'SKIPPED' `
                -Detail 'WhatIf')
        ) | Out-Null
        $installResults.Add(
            (New-InstallResult `
                -Name 'Pinned Rust toolchain' `
                -Status 'SKIPPED' `
                -Detail 'WhatIf')
        ) | Out-Null
    } else {
        $resultsByName = @{}
        & $PathRefresher | Out-Null
        foreach ($name in $Manifest.Order) {
            $package = $Manifest.Packages[$name]
            $blockedDependency = @(
                @($package.DependsOn) | Where-Object {
                    $resultsByName.ContainsKey($_) -and
                    $resultsByName[$_].Status -in @('FAILED', 'SKIPPED')
                }
            ) | Select-Object -First 1

            if ($null -ne $blockedDependency) {
                $result = New-InstallResult `
                    -Name $name `
                    -Status 'SKIPPED' `
                    -Detail "Dependency failed: $blockedDependency"
            } else {
                $installOutput = @(
                    & $PackageInstaller $name $package $PackageDetector $PackageRunner
                )
                if ($installOutput.Count -ne 1) {
                    throw "Package installer '$name' returned $($installOutput.Count) results; expected exactly one."
                }
                $result = $installOutput[0]
            }

            $resultsByName[$name] = $result
            $installResults.Add($result) | Out-Null
            & $PathRefresher | Out-Null
        }

        $gitLfsDependency = @('Git', 'GitLfs') | Where-Object {
            $resultsByName.ContainsKey($_) -and
            $resultsByName[$_].Status -notin @('INSTALLED', 'ALREADY PRESENT')
        } | Select-Object -First 1
        if ($null -ne $gitLfsDependency) {
            $gitLfsResult = New-InstallResult `
                -Name 'Git LFS initialization' `
                -Status SKIPPED `
                -Detail "Dependency failed: $gitLfsDependency"
        } else {
            $gitLfsOutput = @(& $GitLfsInstaller $GitLfsRunner)
            if ($gitLfsOutput.Count -ne 1) {
                throw "Git LFS installer returned $($gitLfsOutput.Count) results; expected exactly one."
            }
            $gitLfsResult = $gitLfsOutput[0]
        }
        $installResults.Add($gitLfsResult) | Out-Null
        & $PathRefresher | Out-Null

        $rustDependency = @('VisualStudio', 'Rustup') | Where-Object {
            $resultsByName.ContainsKey($_) -and
            $resultsByName[$_].Status -notin @('INSTALLED', 'ALREADY PRESENT')
        } | Select-Object -First 1
        if ($null -ne $rustDependency) {
            $rustResult = New-InstallResult `
                -Name 'Pinned Rust toolchain' `
                -Status SKIPPED `
                -Detail "Dependency failed: $rustDependency"
        } else {
            $rustOutput = @(& $RustInstaller $RustRunner)
            if ($rustOutput.Count -ne 1) {
                throw "Rust installer returned $($rustOutput.Count) results; expected exactly one."
            }
            $rustResult = $rustOutput[0]
        }
        $installResults.Add($rustResult) | Out-Null
        & $PathRefresher | Out-Null
    }

    $doctorReport = @(& $Doctor)
    [pscustomobject][ordered]@{
        InstallResults = [object[]]($installResults.ToArray())
        DoctorReport = [object[]]$doctorReport
    }
}

function Get-BootstrapExitCode {
    param(
        [Parameter(Mandatory)][object[]]$InstallResults,
        [Parameter(Mandatory)][object[]]$DoctorReport
    )
    if (@($InstallResults | Where-Object Status -eq 'FAILED').Count -gt 0) {
        return 1
    }
    Get-DoctorExitCode -Report $DoctorReport
}

Export-ModuleMember -Function *
