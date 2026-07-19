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
