[CmdletBinding()]
param([switch]$Json)

$ErrorActionPreference = 'Stop'
try {
    Import-Module (Join-Path $PSScriptRoot 'Bootstrap.Common.psm1') -Force -DisableNameChecking
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
        Write-Error $_.Exception.Message -ErrorAction Continue
    }
    exit 2
}
