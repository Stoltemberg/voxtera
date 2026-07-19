$windowsRoot = Split-Path $PSScriptRoot -Parent
Import-Module (Join-Path $windowsRoot 'Bootstrap.Common.psm1') -Force -DisableNameChecking

function New-FakeCommandResult {
    param(
        [int]$ExitCode = 0,
        [string[]]$Output = @('result'),
        [string]$Command = 'fake command'
    )
    [pscustomobject][ordered]@{
        ExitCode = $ExitCode
        Output = $Output
        Command = $Command
    }
}

Test-Case 'package detection executes the resolved version command' {
    $resolvedCommands = New-Object System.Collections.Generic.List[string]
    $runnerCalls = New-Object System.Collections.Generic.List[object]
    $resolver = {
        param($Command)
        $resolvedCommands.Add($Command) | Out-Null
        [pscustomobject]@{ Source = 'C:\fake\git.exe' }
    }.GetNewClosure()
    $runner = {
        param($FilePath, $Arguments)
        $runnerCalls.Add([pscustomobject]@{
            FilePath = $FilePath
            Arguments = @($Arguments)
        }) | Out-Null
        New-FakeCommandResult -Output @('git version 2.50.0.windows.1')
    }.GetNewClosure()
    $present = Test-PackagePresent -Package @{
        Id = 'Git.Git'
        Command = 'git.exe'
        VersionArguments = @('--version')
        VersionPattern = '^git version '
    } -CommandResolver $resolver -Runner $runner
    Assert-Equal $true $present
    Assert-Equal 1 $resolvedCommands.Count
    Assert-Equal 'git.exe' $resolvedCommands[0]
    Assert-Equal 1 $runnerCalls.Count
    Assert-Equal 'C:\fake\git.exe' $runnerCalls[0].FilePath
    Assert-Equal '--version' ($runnerCalls[0].Arguments -join ' ')
}

Test-Case 'package detection rejects a broken resolved command' {
    $present = Test-PackagePresent -Package @{
        Id = 'Git.Git'
        Command = 'git.exe'
        VersionArguments = @('--version')
        VersionPattern = '^git version '
    } -CommandResolver {
        [pscustomobject]@{ Source = 'C:\fake\git.exe' }
    } -Runner {
        New-FakeCommandResult -ExitCode 9009 -Output @('not executable')
    }
    Assert-Equal $false $present
}

Test-Case 'package detection treats a version invocation error as unusable' {
    $present = Test-PackagePresent -Package @{
        Id = 'Git.Git'
        Command = 'git.exe'
        VersionArguments = @('--version')
        VersionPattern = '^git version '
    } -CommandResolver {
        [pscustomobject]@{ Source = 'C:\fake\git.exe' }
    } -Runner {
        throw 'cannot start process'
    }
    Assert-Equal $false $present
}

Test-Case 'Python package detection requires Python 3.13' {
    foreach ($version in @('Python 2.7.18', 'Python 3.12.10', 'Python 3.14.0')) {
        $present = Test-PackagePresent -Package @{
            Id = 'Python.Python.3.13'
            Command = 'python.exe'
            VersionArguments = @('--version')
            VersionPattern = '^Python 3\.13(?:\.|$)'
        } -CommandResolver {
            [pscustomobject]@{ Source = 'C:\fake\python.exe' }
        } -Runner {
            New-FakeCommandResult -Output @($version)
        }.GetNewClosure()
        Assert-Equal $false $present
    }

    $present = Test-PackagePresent -Package @{
        Id = 'Python.Python.3.13'
        Command = 'python.exe'
        VersionArguments = @('--version')
        VersionPattern = '^Python 3\.13(?:\.|$)'
    } -CommandResolver {
        [pscustomobject]@{ Source = 'C:\fake\python.exe' }
    } -Runner {
        New-FakeCommandResult -Output @('Python 3.13.5')
    }
    Assert-Equal $true $present
}

Test-Case 'package detection rejects the Microsoft Store execution alias' {
    $runnerCalls = 0
    $present = Test-PackagePresent -Package @{
        Id = 'Python.Python.3.13'
        Command = 'python.exe'
        VersionArguments = @('--version')
        VersionPattern = '^Python 3\.13(?:\.|$)'
    } -CommandResolver {
        [pscustomobject]@{
            Source = 'C:\Users\test\AppData\Local\Microsoft\WindowsApps\python.exe'
        }
    } -Runner {
        $runnerCalls++
        throw 'Store alias must not execute.'
    }.GetNewClosure()
    Assert-Equal $false $present
    Assert-Equal 0 $runnerCalls
}

Test-Case 'Visual Studio package requires its C++ workload and Windows SDK' {
    $package = @{
        Id = 'Microsoft.VisualStudio.2022.BuildTools'
        Command = $null
    }
    $present = Test-PackagePresent `
        -Package $package `
        -VisualStudioCheck {
            New-CheckResult 'Visual Studio Build Tools' PASS 'C:\fake\BuildTools'
        } `
        -WindowsSdkCheck {
            New-CheckResult 'Windows SDK' PASS 'C:\fake\Windows Kits\10'
        }
    Assert-Equal $true $present
}

Test-Case 'Visual Studio package is repairable when Windows SDK is missing' {
    $package = @{
        Id = 'Microsoft.VisualStudio.2022.BuildTools'
        Command = $null
    }
    $present = Test-PackagePresent `
        -Package $package `
        -VisualStudioCheck {
            New-CheckResult 'Visual Studio Build Tools' PASS 'C:\fake\BuildTools'
        } `
        -WindowsSdkCheck {
            New-CheckResult 'Windows SDK' FAIL 'missing'
        }
    Assert-Equal $false $present
}

Test-Case 'Visual Studio check requires exact Build Tools 2022 arguments' {
    $calls = New-Object System.Collections.Generic.List[object]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add([pscustomobject]@{
            FilePath = $FilePath
            Arguments = @($Arguments)
        }) | Out-Null
        New-FakeCommandResult -Output @('C:\VS\2022\BuildTools')
    }.GetNewClosure()

    $result = Get-VisualStudioCheck `
        -VswherePath 'C:\fake\vswhere.exe' `
        -Runner $runner

    Assert-Equal 'PASS' $result.Status
    Assert-Equal 1 $calls.Count
    Assert-Equal (
        '-latest -products Microsoft.VisualStudio.Product.BuildTools ' +
        '-version [17.0,18.0) -requires Microsoft.VisualStudio.Workload.VCTools ' +
        '-property installationPath'
    ) ($calls[0].Arguments -join ' ')
}

Test-Case 'Visual Studio identity query failure does not become an absent installation' {
    $caught = $null
    try {
        Get-VisualStudioInstallationPath `
            -VswherePath 'C:\fake\vswhere.exe' `
            -Runner {
                New-FakeCommandResult -ExitCode 5 -Output @('query failed')
            } | Out-Null
    } catch {
        $caught = $_.Exception
    }
    Assert-True ($null -ne $caught)
    Assert-Match 'query failed' $caught.Message
}

Test-Case 'manifest package dispatcher uses the Visual Studio modifier path' {
    $calls = New-Object System.Collections.Generic.List[string]
    $result = Install-ManifestPackage `
        -Name VisualStudio `
        -Package @{
            Id = 'Microsoft.VisualStudio.2022.BuildTools'
        } `
        -Detector { $false } `
        -Runner { throw 'runner should be owned by injected installer' } `
        -VisualStudioInstaller {
            param($Package, $Runner, $LogPath)
            $calls.Add('visual-studio') | Out-Null
            New-InstallResult VisualStudio INSTALLED modified
        }.GetNewClosure() `
        -WingetInstaller {
            $calls.Add('winget') | Out-Null
            throw 'must not use generic Winget dispatch'
        }.GetNewClosure()
    Assert-Equal 'INSTALLED' $result.Status
    Assert-Equal 'visual-studio' ($calls -join ',')
}

Test-Case 'incomplete existing Visual Studio is modified at its exact path' {
    $calls = New-Object System.Collections.Generic.List[object]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add([pscustomobject]@{
            FilePath = $FilePath
            Arguments = @($Arguments)
        }) | Out-Null
        New-FakeCommandResult -Command "$FilePath $($Arguments -join ' ')"
    }.GetNewClosure()
    $result = Install-VisualStudioBuildTools `
        -Package @{
            Id = 'Microsoft.VisualStudio.2022.BuildTools'
            WingetArguments = @()
        } `
        -InstallationPathResolver { 'C:\VS Path\BuildTools' } `
        -VisualStudioCheck {
            New-CheckResult 'Visual Studio Build Tools' FAIL 'workload missing'
        } `
        -WindowsSdkCheck {
            New-CheckResult 'Windows SDK' PASS 'present'
        } `
        -InstallerPath 'C:\VS Installer\setup.exe' `
        -Runner $runner `
        -Confirm:$false

    Assert-Equal 'INSTALLED' $result.Status
    Assert-Equal 1 $calls.Count
    Assert-Equal 'C:\VS Installer\setup.exe' $calls[0].FilePath
    Assert-Equal (
        'modify --installPath C:\VS Path\BuildTools ' +
        '--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended ' +
        '--passive --norestart'
    ) ($calls[0].Arguments -join ' ')
}

Test-Case 'existing Visual Studio with missing SDK uses modify instead of Winget' {
    $calls = New-Object System.Collections.Generic.List[object]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add([pscustomobject]@{
            FilePath = $FilePath
            Arguments = @($Arguments)
        }) | Out-Null
        New-FakeCommandResult
    }.GetNewClosure()

    $result = Install-VisualStudioBuildTools `
        -Package @{
            Id = 'Microsoft.VisualStudio.2022.BuildTools'
            WingetArguments = @()
        } `
        -InstallationPathResolver { 'C:\VS\BuildTools' } `
        -VisualStudioCheck {
            New-CheckResult 'Visual Studio Build Tools' PASS 'C:\VS\BuildTools'
        } `
        -WindowsSdkCheck {
            New-CheckResult 'Windows SDK' FAIL 'missing'
        } `
        -InstallerPath 'C:\VSInstaller\setup.exe' `
        -Runner $runner `
        -Confirm:$false

    Assert-Equal 'INSTALLED' $result.Status
    Assert-Equal 1 $calls.Count
    Assert-Equal 'C:\VSInstaller\setup.exe' $calls[0].FilePath
    Assert-Equal 'modify' $calls[0].Arguments[0]
}

Test-Case 'absent Visual Studio uses exact Winget installation' {
    $calls = New-Object System.Collections.Generic.List[object]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add([pscustomobject]@{
            FilePath = $FilePath
            Arguments = @($Arguments)
        }) | Out-Null
        New-FakeCommandResult
    }.GetNewClosure()
    $result = Install-VisualStudioBuildTools `
        -Package @{
            Id = 'Microsoft.VisualStudio.2022.BuildTools'
            WingetArguments = @(
                '--override',
                '--wait --passive --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended'
            )
        } `
        -InstallationPathResolver { $null } `
        -Runner $runner `
        -Confirm:$false

    Assert-Equal 'INSTALLED' $result.Status
    Assert-Equal 1 $calls.Count
    Assert-Equal 'winget.exe' $calls[0].FilePath
    Assert-True ($calls[0].Arguments -contains '--no-upgrade')
    Assert-True ($calls[0].Arguments -contains '--source')
    Assert-True ($calls[0].Arguments -contains 'winget')
}

Test-Case 'bootstrap log path resolution is pure and canonical' {
    $relativeDirectory = "VelorenDev\logs\pure-$([guid]::NewGuid())"
    $directory = Join-Path $env:LOCALAPPDATA $relativeDirectory
    $path = Join-Path $directory '..\canonical.log'
    try {
        $resolved = Resolve-BootstrapLogPath -Path $path
        $expected = [System.IO.Path]::GetFullPath(
            (Join-Path $env:LOCALAPPDATA 'VelorenDev\logs\canonical.log')
        )
        Assert-Equal $expected $resolved
        Assert-Equal $false (Test-Path -LiteralPath $directory)
    } finally {
        Remove-Item -LiteralPath $directory -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Test-Case 'Winget installer retries once and succeeds' {
    $calls = New-Object System.Collections.Generic.List[object]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add([pscustomobject]@{ FilePath = $FilePath; Arguments = @($Arguments) }) | Out-Null
        New-FakeCommandResult -ExitCode $(if ($calls.Count -eq 1) { 1 } else { 0 }) `
            -Command "$FilePath $($Arguments -join ' ')"
    }.GetNewClosure()
    $result = Install-WingetPackage -Name Git -Package @{
        Id = 'Git.Git'
        WingetArguments = @()
    } -Detector { param($Package) $false } -Runner $runner -Confirm:$false
    Assert-Equal 2 $calls.Count
    Assert-Equal 'INSTALLED' $result.Status
    Assert-Equal 'winget.exe' $calls[0].FilePath
    Assert-Equal 'install --id Git.Git --exact --source winget --no-upgrade --accept-package-agreements --accept-source-agreements --disable-interactivity' `
        ($calls[0].Arguments -join ' ')
}

Test-Case 'present package is not sent to Winget' {
    $calls = 0
    $runner = {
        param($FilePath, $Arguments)
        $calls++
        throw 'must not run'
    }.GetNewClosure()
    $result = Install-WingetPackage -Name Git -Package @{
        Id = 'Git.Git'
        WingetArguments = @()
    } -Detector { param($Package) $true } -Runner $runner -Confirm:$false
    Assert-Equal 0 $calls
    Assert-Equal 'ALREADY PRESENT' $result.Status
}

Test-Case 'successful Winget install is not retried' {
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add($FilePath) | Out-Null
        New-FakeCommandResult -ExitCode 0
    }.GetNewClosure()
    $result = Install-WingetPackage -Name Git -Package @{
        Id = 'Git.Git'
        WingetArguments = @()
    } -Detector { param($Package) $false } -Runner $runner -Confirm:$false
    Assert-Equal 1 $calls.Count
    Assert-Equal 'INSTALLED' $result.Status
}

Test-Case 'failed Winget install is attempted at most twice' {
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add($FilePath) | Out-Null
        New-FakeCommandResult -ExitCode 9 -Output @('simulated Winget failure')
    }.GetNewClosure()
    $result = Install-WingetPackage -Name Git -Package @{
        Id = 'Git.Git'
        WingetArguments = @()
    } -Detector { param($Package) $false } -Runner $runner -Confirm:$false
    Assert-Equal 2 $calls.Count
    Assert-Equal 'FAILED' $result.Status
    Assert-Match 'simulated Winget failure' $result.Detail
}

Test-Case 'Winget code 3010 is successful and requires restart' {
    $runner = {
        param($FilePath, $Arguments)
        New-FakeCommandResult -ExitCode 3010 -Output @('restart required') -Command 'winget install'
    }
    $result = Install-WingetPackage -Name CMake -Package @{
        Id = 'Kitware.CMake'
        WingetArguments = @()
    } -Detector { param($Package) $false } -Runner $runner -Confirm:$false
    Assert-Equal 'INSTALLED' $result.Status
    Assert-Match 'restart' $result.Detail
}

Test-Case 'Winget code 1641 reports forbidden restart initiation as failure' {
    $runner = {
        param($FilePath, $Arguments)
        New-FakeCommandResult -ExitCode 1641 -Output @('restart initiated') -Command 'winget install'
    }
    $result = Install-WingetPackage -Name CMake -Package @{
        Id = 'Kitware.CMake'
        WingetArguments = @()
    } -Detector { param($Package) $false } -Runner $runner -Confirm:$false
    Assert-Equal 'FAILED' $result.Status
    Assert-Match 'restart initiated' $result.Detail
    Assert-Match 'forbidden' $result.Detail
}

Test-Case 'Winget runner errors propagate' {
    $caught = $null
    try {
        Install-WingetPackage -Name Git -Package @{
            Id = 'Git.Git'
            WingetArguments = @()
        } -Detector { param($Package) $false } -Runner {
            param($FilePath, $Arguments)
            throw 'unexpected Winget runner error'
        } -Confirm:$false | Out-Null
    } catch {
        $caught = $_.Exception
    }
    Assert-True ($null -ne $caught)
    Assert-Match 'unexpected Winget runner error' $caught.Message
}

Test-Case 'Winget installer log remains contained under bootstrap log root' {
    $outsidePath = Join-Path $env:TEMP ("veloren-installer-{0}.log" -f [guid]::NewGuid())
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add($FilePath) | Out-Null
        New-FakeCommandResult -ExitCode 0 -Command 'winget install'
    }.GetNewClosure()
    $caught = $null
    try {
        Install-WingetPackage -Name Git -Package @{
            Id = 'Git.Git'
            WingetArguments = @()
        } -Detector { param($Package) $false } -Runner $runner `
            -LogPath $outsidePath -WhatIf | Out-Null
    } catch {
        $caught = $_.Exception
    } finally {
        Remove-Item -LiteralPath $outsidePath -Force -ErrorAction SilentlyContinue
    }
    Assert-True ($null -ne $caught)
    Assert-Match 'must stay under' $caught.Message
    Assert-Equal 0 $calls.Count
}

Test-Case 'Winget WhatIf skips without a runner call' {
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add($FilePath) | Out-Null
        throw 'must not run'
    }.GetNewClosure()
    $result = Install-WingetPackage -Name Git -Package @{
        Id = 'Git.Git'
        WingetArguments = @()
    } -Detector { param($Package) $false } -Runner $runner -WhatIf
    Assert-Equal 'SKIPPED' $result.Status
    Assert-Equal 0 $calls.Count
}

Test-Case 'unavailable Git LFS initialization is skipped without a runner call' {
    $calls = 0
    $runner = {
        param($FilePath, $Arguments)
        $calls++
        throw 'must not run'
    }.GetNewClosure()
    $result = Initialize-GitLfs -GitLfsAvailable:$false -Runner $runner -Confirm:$false
    Assert-Equal 0 $calls
    Assert-Equal 'SKIPPED' $result.Status
}

Test-Case 'Git LFS initialization checks its external exit code' {
    $runner = {
        param($FilePath, $Arguments)
        New-FakeCommandResult -ExitCode 6 -Output @('git lfs failed') `
            -Command "$FilePath $($Arguments -join ' ')"
    }
    $result = Initialize-GitLfs -GitLfsAvailable:$true -Runner $runner -Confirm:$false
    Assert-Equal 'FAILED' $result.Status
    Assert-Match 'git lfs failed' $result.Detail
}

Test-Case 'Git LFS initialization uses git lfs install' {
    $calls = New-Object System.Collections.Generic.List[object]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add([pscustomobject]@{ FilePath = $FilePath; Arguments = @($Arguments) }) | Out-Null
        New-FakeCommandResult -ExitCode 0 -Command "$FilePath $($Arguments -join ' ')"
    }.GetNewClosure()
    $result = Initialize-GitLfs -GitLfsAvailable:$true -Runner $runner -Confirm:$false
    Assert-Equal 'INSTALLED' $result.Status
    Assert-Equal 1 $calls.Count
    Assert-Equal 'git.exe' $calls[0].FilePath
    Assert-Equal 'lfs install' ($calls[0].Arguments -join ' ')
}

Test-Case 'Git LFS runner errors propagate' {
    $caught = $null
    try {
        Initialize-GitLfs -GitLfsAvailable:$true -Runner {
            param($FilePath, $Arguments)
            throw 'unexpected Git LFS runner error'
        } -Confirm:$false | Out-Null
    } catch {
        $caught = $_.Exception
    }
    Assert-True ($null -ne $caught)
    Assert-Match 'unexpected Git LFS runner error' $caught.Message
}

Test-Case 'Git LFS rejects an invalid log path before its runner call' {
    $outsidePath = Join-Path $env:TEMP ("veloren-git-lfs-{0}.log" -f [guid]::NewGuid())
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add($FilePath) | Out-Null
        New-FakeCommandResult -ExitCode 0
    }.GetNewClosure()
    $caught = $null
    try {
        Initialize-GitLfs -GitLfsAvailable:$true -Runner $runner `
            -LogPath $outsidePath -WhatIf | Out-Null
    } catch {
        $caught = $_.Exception
    }
    Assert-True ($null -ne $caught)
    Assert-Match 'must stay under' $caught.Message
    Assert-Equal 0 $calls.Count
}

Test-Case 'Git LFS WhatIf skips without a runner call' {
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add($FilePath) | Out-Null
        throw 'must not run'
    }.GetNewClosure()
    $result = Initialize-GitLfs -GitLfsAvailable:$true -Runner $runner -WhatIf
    Assert-Equal 'SKIPPED' $result.Status
    Assert-Equal 0 $calls.Count
}

Test-Case 'failed prerequisite produces skipped Rust toolchain' {
    $calls = 0
    $runner = {
        param($FilePath, $Arguments)
        $calls++
        throw 'must not run'
    }.GetNewClosure()
    $result = Install-PinnedRustToolchain -RepositoryRoot (Get-RepositoryRoot) `
        -RustupAvailable:$false -Runner $runner -Confirm:$false
    Assert-Equal 0 $calls
    Assert-Equal 'SKIPPED' $result.Status
}

Test-Case 'ready pinned Rust toolchain is not reinstalled' {
    $root = Get-RepositoryRoot
    $pinned = Get-PinnedToolchain $root
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $call = $Arguments -join ' '
        $calls.Add($call) | Out-Null
        if ($call -eq 'toolchain list') {
            return New-FakeCommandResult -ExitCode 0 -Output @("$pinned-x86_64-pc-windows-msvc (active)")
        }
        if ($call -eq "component list --toolchain $pinned --installed") {
            return New-FakeCommandResult -ExitCode 0 -Output @(
                "rustfmt-x86_64-pc-windows-msvc (installed)",
                "clippy-x86_64-pc-windows-msvc (installed)"
            )
        }
        throw "Unexpected Rustup command: $call"
    }.GetNewClosure()
    $result = Install-PinnedRustToolchain -RepositoryRoot $root `
        -RustupAvailable:$true -Runner $runner -Confirm:$false
    Assert-Equal 'ALREADY PRESENT' $result.Status
    Assert-Equal 2 $calls.Count
}

Test-Case 'Rust toolchain collision does not satisfy the exact pin' {
    $root = Get-RepositoryRoot
    $pinned = Get-PinnedToolchain $root
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $call = $Arguments -join ' '
        $calls.Add($call) | Out-Null
        if ($call -eq 'toolchain list') {
            return New-FakeCommandResult -ExitCode 0 -Output @("$pinned-custom (active)")
        }
        New-FakeCommandResult -ExitCode 0 -Command "$FilePath $call"
    }.GetNewClosure()
    $result = Install-PinnedRustToolchain -RepositoryRoot $root `
        -RustupAvailable:$true -Runner $runner -Confirm:$false
    Assert-Equal 'INSTALLED' $result.Status
    Assert-Equal 2 $calls.Count
    Assert-Equal "toolchain install $pinned --profile minimal --component rustfmt --component clippy" $calls[1]
}

Test-Case 'failed Rust toolchain query is reported without installation' {
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add(($Arguments -join ' ')) | Out-Null
        New-FakeCommandResult -ExitCode 7 -Output @('toolchain query failed')
    }.GetNewClosure()
    $result = Install-PinnedRustToolchain -RepositoryRoot (Get-RepositoryRoot) `
        -RustupAvailable:$true -Runner $runner -Confirm:$false
    Assert-Equal 'FAILED' $result.Status
    Assert-Equal 1 $calls.Count
    Assert-Match 'toolchain query failed' $result.Detail
}

Test-Case 'failed Rust component query is reported without installation' {
    $root = Get-RepositoryRoot
    $pinned = Get-PinnedToolchain $root
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $call = $Arguments -join ' '
        $calls.Add($call) | Out-Null
        if ($call -eq 'toolchain list') {
            return New-FakeCommandResult -ExitCode 0 -Output @("$pinned-x86_64-pc-windows-msvc")
        }
        New-FakeCommandResult -ExitCode 8 -Output @('component query failed')
    }.GetNewClosure()
    $result = Install-PinnedRustToolchain -RepositoryRoot $root `
        -RustupAvailable:$true -Runner $runner -Confirm:$false
    Assert-Equal 'FAILED' $result.Status
    Assert-Equal 2 $calls.Count
    Assert-Match 'component query failed' $result.Detail
}

Test-Case 'Rust installer pins toolchain components without changing global default' {
    $root = Get-RepositoryRoot
    $pinned = Get-PinnedToolchain $root
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $call = $Arguments -join ' '
        $calls.Add($call) | Out-Null
        if ($call -eq 'toolchain list') {
            return New-FakeCommandResult -ExitCode 0 -Output @('stable-x86_64-pc-windows-msvc')
        }
        New-FakeCommandResult -ExitCode 0 -Command "$FilePath $call"
    }.GetNewClosure()
    $result = Install-PinnedRustToolchain -RepositoryRoot $root `
        -RustupAvailable:$true -Runner $runner -Confirm:$false
    Assert-Equal 'INSTALLED' $result.Status
    Assert-Equal 2 $calls.Count
    Assert-Equal "toolchain install $pinned --profile minimal --component rustfmt --component clippy" $calls[1]
    Assert-True (-not (($calls -join "`n") -match '(^|\s)default(\s|$)'))
}

Test-Case 'Rust adds only a missing pinned component without changing global default' {
    $root = Get-RepositoryRoot
    $pinned = Get-PinnedToolchain $root
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $call = $Arguments -join ' '
        $calls.Add($call) | Out-Null
        if ($call -eq 'toolchain list') {
            return New-FakeCommandResult -ExitCode 0 `
                -Output @("$pinned-x86_64-pc-windows-msvc (active)")
        }
        if ($call -eq "component list --toolchain $pinned --installed") {
            return New-FakeCommandResult -ExitCode 0 `
                -Output @('rustfmt-x86_64-pc-windows-msvc (installed)')
        }
        New-FakeCommandResult -ExitCode 0 -Command "$FilePath $call"
    }.GetNewClosure()
    $result = Install-PinnedRustToolchain -RepositoryRoot $root `
        -RustupAvailable:$true -Runner $runner -Confirm:$false
    Assert-Equal 'INSTALLED' $result.Status
    Assert-Equal 3 $calls.Count
    Assert-Equal "component add --toolchain $pinned clippy" $calls[2]
    Assert-True (-not (($calls -join "`n") -match '(^|\s)default(\s|$)'))
}

Test-Case 'Rust rejects an invalid log path before its runner call' {
    $outsidePath = Join-Path $env:TEMP ("veloren-rust-{0}.log" -f [guid]::NewGuid())
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add($FilePath) | Out-Null
        New-FakeCommandResult -ExitCode 0
    }.GetNewClosure()
    $caught = $null
    try {
        Install-PinnedRustToolchain -RepositoryRoot (Get-RepositoryRoot) `
            -RustupAvailable:$true -Runner $runner -LogPath $outsidePath `
            -WhatIf | Out-Null
    } catch {
        $caught = $_.Exception
    }
    Assert-True ($null -ne $caught)
    Assert-Match 'must stay under' $caught.Message
    Assert-Equal 0 $calls.Count
}

Test-Case 'Rust WhatIf skips without a runner call' {
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $calls.Add($FilePath) | Out-Null
        throw 'must not run'
    }.GetNewClosure()
    $result = Install-PinnedRustToolchain -RepositoryRoot (Get-RepositoryRoot) `
        -RustupAvailable:$true -Runner $runner -WhatIf
    Assert-Equal 'SKIPPED' $result.Status
    Assert-Equal 0 $calls.Count
}

Test-Case 'failed Rust installation checks its external exit code' {
    $root = Get-RepositoryRoot
    $pinned = Get-PinnedToolchain $root
    $runner = {
        param($FilePath, $Arguments)
        $call = $Arguments -join ' '
        if ($call -eq 'toolchain list') {
            return New-FakeCommandResult -ExitCode 0 -Output @()
        }
        New-FakeCommandResult -ExitCode 10 -Output @('Rust install failed') -Command "$FilePath $call"
    }.GetNewClosure()
    $result = Install-PinnedRustToolchain -RepositoryRoot $root `
        -RustupAvailable:$true -Runner $runner -Confirm:$false
    Assert-Equal 'FAILED' $result.Status
    Assert-Match 'Rust install failed' $result.Detail
}

Test-Case 'Rust runner errors propagate' {
    $caught = $null
    try {
        Install-PinnedRustToolchain -RepositoryRoot (Get-RepositoryRoot) `
            -RustupAvailable:$true -Runner {
                param($FilePath, $Arguments)
                throw 'unexpected Rust runner error'
            } -Confirm:$false | Out-Null
    } catch {
        $caught = $_.Exception
    }
    Assert-True ($null -ne $caught)
    Assert-Match 'unexpected Rust runner error' $caught.Message
}
