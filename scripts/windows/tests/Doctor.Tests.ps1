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

Test-Case 'a failed probe becomes a FAIL check' {
    $probes = [ordered]@{
        MissingPrerequisite = { throw 'prerequisite is unavailable' }
        Git = { New-CheckResult Git PASS 'present' }
    }
    $report = @(Get-DoctorReport -Probes $probes)
    Assert-Equal 2 $report.Count
    Assert-Equal 'MissingPrerequisite' $report[0].Name
    Assert-Equal 'FAIL' $report[0].Status
    Assert-Equal 'prerequisite is unavailable' $report[0].Detail
    Assert-Equal 'Git' $report[1].Name
}

Test-Case 'doctor JSON is one valid document' {
    $doctor = Join-Path $windowsRoot 'doctor.ps1'
    $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $doctor -Json
    $exitCode = $LASTEXITCODE
    $document = ($output -join "`n") | ConvertFrom-Json
    Assert-True ($exitCode -in @(0, 1))
    Assert-True ($document.Checks.Count -gt 0)
    Assert-True ($null -ne $document.Healthy)
}
