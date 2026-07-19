[CmdletBinding(SupportsShouldProcess)]
param(
    [switch]$Elevated,
    [string]$LogPath
)

$ErrorActionPreference = 'Stop'
$dryRun = [bool]$WhatIfPreference
try {
    Import-Module `
        (Join-Path $PSScriptRoot 'Bootstrap.Common.psm1') `
        -Force `
        -DisableNameChecking

    $root = Get-RepositoryRoot
    if ($dryRun) {
        $WhatIfPreference = $false
        try {
            Get-PlatformCheck | Out-Null
        } finally {
            $WhatIfPreference = $true
        }
    } else {
        Get-PlatformCheck | Out-Null
    }

    if (-not $dryRun -and
        $null -eq (Get-Command winget.exe -ErrorAction SilentlyContinue)) {
        Write-Error `
            'Winget is missing. Install Microsoft App Installer and rerun this command.' `
            -ErrorAction Continue
        exit 1
    }

    if ([string]::IsNullOrWhiteSpace($LogPath)) {
        $LogPath = New-BootstrapLogPath
    }

    if (-not $dryRun -and
        -not $Elevated -and
        -not (Test-IsAdministrator)) {
        $childExitCode = Start-ElevatedBootstrap `
            -ScriptPath $PSCommandPath `
            -LogPath $LogPath
        exit $childExitCode
    }

    if (-not $dryRun) {
        Write-BootstrapLog -Path $LogPath -Message "Repository: $root"
    }

    $manifest = Import-PowerShellDataFile (Join-Path $PSScriptRoot 'packages.psd1')
    $packageInstaller = {
        param($Name, $Package, $Detector, $Runner)
        Install-ManifestPackage `
            -Name $Name `
            -Package $Package `
            -Detector $Detector `
            -Runner $Runner `
            -LogPath $LogPath `
            -Confirm:$false
    }.GetNewClosure()
    $gitLfsInstaller = {
        param($Runner)
        Initialize-GitLfs `
            -Runner $Runner `
            -LogPath $LogPath `
            -Confirm:$false
    }.GetNewClosure()
    $rustInstaller = {
        param($Runner)
        Install-PinnedRustToolchain `
            -RepositoryRoot $root `
            -Runner $Runner `
            -LogPath $LogPath `
            -Confirm:$false
    }.GetNewClosure()
    $doctor = {
        if ($dryRun) {
            $WhatIfPreference = $false
        }
        Get-DoctorReport -Probes (Get-DefaultDoctorProbes -RepositoryRoot $root)
    }.GetNewClosure()

    $workflow = Invoke-BootstrapWorkflow `
        -Manifest $manifest `
        -PackageInstaller $packageInstaller `
        -GitLfsInstaller $gitLfsInstaller `
        -RustInstaller $rustInstaller `
        -Doctor $doctor `
        -DryRun:$dryRun
    $installResults = @($workflow.InstallResults)
    $doctorReport = @($workflow.DoctorReport)

    if (-not $dryRun) {
        foreach ($result in $installResults) {
            Write-BootstrapLog `
                -Path $LogPath `
                -Message "Install $($result.Name): $($result.Status) - $($result.Detail)"
        }
        foreach ($check in $doctorReport) {
            Write-BootstrapLog `
                -Path $LogPath `
                -Message "Doctor $($check.Name): $($check.Status) - $($check.Detail)"
        }
    }

    Write-Host 'Veloren Development Environment'
    Write-Host ''
    foreach ($result in $installResults) {
        "{0,-16} {1,-28} {2}" -f `
            $result.Status, $result.Name, $result.Detail |
            Write-Host
    }
    Write-Host ''
    Write-DoctorTable -Report $doctorReport | Write-Host
    Write-Host ''
    if (Test-PendingRestart) {
        Write-Warning 'Restart Windows before using the toolchain.'
    }
    Write-Host 'Reopen the terminal before using Cargo.'

    exit (Get-BootstrapExitCode `
        -InstallResults $installResults `
        -DoctorReport $doctorReport)
} catch {
    if (-not $dryRun -and
        -not [string]::IsNullOrWhiteSpace($LogPath)) {
        try {
            Write-BootstrapLog `
                -Path $LogPath `
                -Message "Internal error: $($_.Exception.Message)"
        } catch {
            # Preserve the original bootstrap failure when its log path is unusable.
        }
    }
    Write-Error $_.Exception.Message -ErrorAction Continue
    exit 2
}
