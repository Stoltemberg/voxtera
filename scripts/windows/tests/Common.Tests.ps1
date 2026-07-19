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
