$windowsRoot = Split-Path $PSScriptRoot -Parent
Import-Module (Join-Path $windowsRoot 'Bootstrap.Common.psm1') -Force -DisableNameChecking
$manifest = Import-PowerShellDataFile (Join-Path $windowsRoot 'packages.psd1')

Test-Case 'elevation arguments quote paths and preserve dry run' {
    $arguments = @(Get-ElevationArguments `
        -ScriptPath 'C:\repo with spaces\bootstrap.ps1' `
        -LogPath 'C:\log path\bootstrap.log' `
        -DryRun)

    Assert-Equal '-File' $arguments[3]
    Assert-Equal '"C:\repo with spaces\bootstrap.ps1"' $arguments[4]
    Assert-Equal '-Elevated' $arguments[5]
    Assert-Equal '-LogPath' $arguments[6]
    Assert-Equal '"C:\log path\bootstrap.log"' $arguments[7]
    Assert-Equal '-WhatIf' $arguments[8]
}

Test-Case 'elevated bootstrap uses the injected starter and propagates its exit code' {
    $calls = New-Object System.Collections.Generic.List[object]
    $starter = {
        param($Executable, $Arguments)
        $calls.Add([pscustomobject]@{
            Executable = $Executable
            Arguments = @($Arguments)
        }) | Out-Null
        [pscustomobject]@{ ExitCode = 37 }
    }.GetNewClosure()

    $exitCode = Start-ElevatedBootstrap `
        -ScriptPath 'C:\repo with spaces\bootstrap.ps1' `
        -LogPath 'C:\log path\bootstrap.log' `
        -DryRun `
        -Starter $starter

    Assert-Equal 37 $exitCode
    Assert-Equal 1 $calls.Count
    Assert-Match 'powershell(\.exe)?$' $calls[0].Executable
    Assert-True ($calls[0].Arguments -contains '-WhatIf')
    Assert-True ($calls[0].Arguments -contains '"C:\log path\bootstrap.log"')
}

Test-Case 'workflow preserves manifest order' {
    $calls = New-Object System.Collections.Generic.List[string]
    $installer = {
        param($Name, $Package)
        $calls.Add($Name) | Out-Null
        New-InstallResult $Name INSTALLED $Package.Id
    }.GetNewClosure()
    $doctor = {
        $calls.Add('Doctor') | Out-Null
        New-CheckResult 'Injected doctor' PASS ok
    }.GetNewClosure()

    $workflow = Invoke-BootstrapWorkflow `
        -Manifest $manifest `
        -PackageInstaller $installer `
        -GitLfsInstaller { New-InstallResult 'Git LFS initialization' INSTALLED ok } `
        -RustInstaller { New-InstallResult 'Pinned Rust toolchain' INSTALLED ok } `
        -PathRefresher { 'refreshed' } `
        -Doctor $doctor
    $results = @($workflow.InstallResults)

    Assert-Equal (($manifest.Order + 'Doctor') -join ',') ($calls -join ',')
    Assert-Equal (($manifest.Order + @(
        'Git LFS initialization',
        'Pinned Rust toolchain'
    )) -join ',') ($results.Name -join ',')
    Assert-Equal 1 @($workflow.DoctorReport).Count
    Assert-Equal 'Injected doctor' $workflow.DoctorReport[0].Name
}

Test-Case 'workflow continues after independent package failure' {
    $installer = {
        param($Name, $Package)
        if ($Name -eq 'CMake') { New-InstallResult $Name FAILED 'simulated' }
        else { New-InstallResult $Name INSTALLED $Package.Id }
    }

    $workflow = Invoke-BootstrapWorkflow `
        -Manifest $manifest `
        -PackageInstaller $installer `
        -GitLfsInstaller { New-InstallResult 'Git LFS initialization' INSTALLED ok } `
        -RustInstaller { New-InstallResult 'Pinned Rust toolchain' INSTALLED ok } `
        -PathRefresher { 'refreshed' } `
        -Doctor { New-CheckResult 'Injected doctor' PASS ok }
    $results = @($workflow.InstallResults)

    Assert-True ($results.Name -contains 'Ninja')
    Assert-Equal 'FAILED' (($results | Where-Object Name -eq CMake).Status)
    Assert-Equal 'INSTALLED' (($results | Where-Object Name -eq Ninja).Status)
}

Test-Case 'workflow skips failed dependencies without calling their installer' {
    $orderedManifest = @{
        Order = @('Base', 'Dependent', 'Independent')
        Packages = @{
            Base = @{ Id = 'Example.Base'; DependsOn = @() }
            Dependent = @{ Id = 'Example.Dependent'; DependsOn = @('Base') }
            Independent = @{ Id = 'Example.Independent'; DependsOn = @() }
        }
    }
    $calls = New-Object System.Collections.Generic.List[string]
    $installer = {
        param($Name, $Package)
        $calls.Add($Name) | Out-Null
        if ($Name -eq 'Base') { New-InstallResult $Name FAILED simulated }
        else { New-InstallResult $Name INSTALLED $Package.Id }
    }.GetNewClosure()

    $workflow = Invoke-BootstrapWorkflow `
        -Manifest $orderedManifest `
        -PackageInstaller $installer `
        -GitLfsInstaller { New-InstallResult 'Git LFS initialization' INSTALLED ok } `
        -RustInstaller { New-InstallResult 'Pinned Rust toolchain' INSTALLED ok } `
        -PathRefresher { 'refreshed' } `
        -Doctor { New-CheckResult 'Injected doctor' PASS ok }
    $results = @($workflow.InstallResults)

    Assert-Equal 'Base,Independent' ($calls -join ',')
    Assert-Equal 'SKIPPED' (($results | Where-Object Name -eq Dependent).Status)
    Assert-Match 'Base' (($results | Where-Object Name -eq Dependent).Detail)
}

Test-Case 'dry run calls no installers or path refresher and returns a coherent plan' {
    $calls = New-Object System.Collections.Generic.List[string]
    $packageInstaller = {
        param($Name, $Package)
        $calls.Add("package:$Name") | Out-Null
        throw 'Package installer must not run.'
    }.GetNewClosure()
    $gitLfsInstaller = {
        $calls.Add('git-lfs') | Out-Null
        throw 'Git LFS installer must not run.'
    }.GetNewClosure()
    $rustInstaller = {
        $calls.Add('rust') | Out-Null
        throw 'Rust installer must not run.'
    }.GetNewClosure()
    $pathRefresher = {
        $calls.Add('path') | Out-Null
        throw 'PATH refresher must not run.'
    }.GetNewClosure()
    $doctorCalls = New-Object System.Collections.Generic.List[string]
    $doctor = {
        $doctorCalls.Add('doctor') | Out-Null
        New-CheckResult 'Injected doctor' FAIL incomplete
    }.GetNewClosure()

    $workflow = Invoke-BootstrapWorkflow `
        -Manifest $manifest `
        -PackageInstaller $packageInstaller `
        -GitLfsInstaller $gitLfsInstaller `
        -RustInstaller $rustInstaller `
        -PathRefresher $pathRefresher `
        -Doctor $doctor `
        -DryRun
    $results = @($workflow.InstallResults)

    Assert-Equal 0 $calls.Count
    Assert-Equal 1 $doctorCalls.Count
    Assert-Equal 'FAIL' $workflow.DoctorReport[0].Status
    Assert-Equal (($manifest.Order + @(
        'Git LFS initialization',
        'Pinned Rust toolchain'
    )) -join ',') ($results.Name -join ',')
    Assert-Equal 0 (@($results | Where-Object Status -ne SKIPPED).Count)
    Assert-Equal 0 (@($results | Where-Object Detail -ne WhatIf).Count)
}

Test-Case 'bootstrap exit code prioritizes installer and doctor failures' {
    $failedInstall = @(New-InstallResult Git FAILED simulated)
    $healthyInstall = @(New-InstallResult Git INSTALLED ok)
    $failedDoctor = @(New-CheckResult Git FAIL missing)
    $healthyDoctor = @(New-CheckResult Git PASS present)

    Assert-Equal 1 (Get-BootstrapExitCode $failedInstall $healthyDoctor)
    Assert-Equal 1 (Get-BootstrapExitCode $healthyInstall $failedDoctor)
    Assert-Equal 0 (Get-BootstrapExitCode $healthyInstall $healthyDoctor)
}

Test-Case 'bootstrap dry run creates no log directory and requests no elevation' {
    $bootstrap = (Join-Path $windowsRoot 'bootstrap.ps1').Replace("'", "''")
    $localAppData = Join-Path $env:TEMP ("veloren-whatif-{0}" -f [guid]::NewGuid())
    $log = Join-Path $localAppData 'VelorenDev\logs\bootstrap.log'
    $escapedLocalAppData = $localAppData.Replace("'", "''")
    $escapedLog = $log.Replace("'", "''")
    $command = @"
& {
    `$env:LOCALAPPDATA = '$escapedLocalAppData'
    function global:Start-Process { throw 'Dry run requested elevation.' }
    & '$bootstrap' -WhatIf -LogPath '$escapedLog'
    exit `$LASTEXITCODE
}
"@

    try {
        $previousErrorActionPreference = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -Command $command *>&1
        $exitCode = $LASTEXITCODE
        $ErrorActionPreference = $previousErrorActionPreference

        Assert-True ($exitCode -in @(0, 1)) "Dry run exited $exitCode.`n$($output -join "`n")"
        Assert-True (-not (Test-Path -LiteralPath $localAppData)) 'Dry run created a log directory.'
        Assert-True (-not (($output -join "`n") -match 'Dry run requested elevation\.'))
        Assert-True (-not (($output -join "`n") -match '(?im)^What if:'))
        Assert-Equal 9 (@($output | Where-Object { $_.ToString() -match '^SKIPPED\s+' }).Count)
    } finally {
        $ErrorActionPreference = $previousErrorActionPreference
        Remove-Item -LiteralPath $localAppData -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Test-Case 'bootstrap rejects Windows Server before installation' {
    $bootstrap = (Join-Path $windowsRoot 'bootstrap.ps1').Replace("'", "''")
    $command = @"
& {
    function global:Get-CimInstance {
        [pscustomobject]@{ ProductType = 3 }
    }
    function global:Start-Process { throw 'Unsupported platforms must not request elevation.' }
    & '$bootstrap' -WhatIf
    exit `$LASTEXITCODE
}
"@

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -Command $command 2>&1
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference

    Assert-Equal 2 $exitCode
    Assert-Match 'client editions' ($output -join "`n")
    Assert-True (-not (($output -join "`n") -match 'Unsupported platforms must not request elevation\.'))
}

Test-Case 'bootstrap dry run does not emit nested ShouldProcess noise' {
    $bootstrap = Join-Path $windowsRoot 'bootstrap.ps1'
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $output = & powershell.exe `
        -NoProfile `
        -ExecutionPolicy Bypass `
        -File $bootstrap `
        -WhatIf *>&1
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference

    Assert-True ($exitCode -in @(0, 1)) "Dry run exited $exitCode."
    Assert-True (-not (($output -join "`n") -match '(?im)^What if:'))
}

Test-Case 'bootstrap preserves its primary error when error logging also fails' {
    $bootstrap = (Join-Path $windowsRoot 'bootstrap.ps1').Replace("'", "''")
    $outsideLog = (Join-Path $env:TEMP ("veloren-bootstrap-{0}.log" -f [guid]::NewGuid())).
        Replace("'", "''")
    $command = @"
& {
    function global:Get-CimInstance {
        [pscustomobject]@{ ProductType = 3 }
    }
    function global:Start-Process { throw 'Unsupported platforms must not request elevation.' }
    & '$bootstrap' -LogPath '$outsideLog'
    exit `$LASTEXITCODE
}
"@

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -Command $command 2>&1
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference

    Assert-Equal 2 $exitCode
    Assert-Match 'client editions' ($output -join "`n")
    Assert-True (-not (($output -join "`n") -match 'must stay under'))
}

Test-Case 'missing Winget is an incomplete environment' {
    $bootstrap = (Join-Path $windowsRoot 'bootstrap.ps1').Replace("'", "''")
    $log = (Join-Path $env:TEMP ("veloren-missing-winget-{0}.log" -f [guid]::NewGuid())).
        Replace("'", "''")
    $command = @"
& {
    function global:Get-CimInstance {
        [pscustomobject]@{ ProductType = 1 }
    }
    function global:Get-Command {
        param(`$Name, `$ErrorAction)
        if (`$Name -eq 'winget.exe') { return `$null }
        Microsoft.PowerShell.Core\Get-Command -Name `$Name -ErrorAction `$ErrorAction
    }
    & '$bootstrap' -Elevated -LogPath '$log'
    exit `$LASTEXITCODE
}
"@

    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -Command $command 2>&1
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference

    Assert-Equal 1 $exitCode
    Assert-Match 'Winget is missing' ($output -join "`n")
    Assert-True (-not (Test-Path -LiteralPath $log))
}
