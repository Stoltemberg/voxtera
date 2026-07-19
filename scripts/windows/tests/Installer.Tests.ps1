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

Test-Case 'package detection resolves the manifest command without executing it' {
    $resolvedCommands = New-Object System.Collections.Generic.List[string]
    $resolver = {
        param($Command)
        $resolvedCommands.Add($Command) | Out-Null
        [pscustomobject]@{ Source = 'C:\fake\git.exe' }
    }.GetNewClosure()
    $present = Test-PackagePresent -Package @{
        Id = 'Git.Git'
        Command = 'git.exe'
    } -CommandResolver $resolver
    Assert-Equal $true $present
    Assert-Equal 1 $resolvedCommands.Count
    Assert-Equal 'git.exe' $resolvedCommands[0]
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
    Assert-Equal 'install --id Git.Git --exact --accept-package-agreements --accept-source-agreements --disable-interactivity' `
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

Test-Case 'Winget code 1641 is successful and requires restart' {
    $runner = {
        param($FilePath, $Arguments)
        New-FakeCommandResult -ExitCode 1641 -Output @('restart initiated') -Command 'winget install'
    }
    $result = Install-WingetPackage -Name CMake -Package @{
        Id = 'Kitware.CMake'
        WingetArguments = @()
    } -Detector { param($Package) $false } -Runner $runner -Confirm:$false
    Assert-Equal 'INSTALLED' $result.Status
    Assert-Match 'restart' $result.Detail
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
    $caught = $null
    try {
        Install-WingetPackage -Name Git -Package @{
            Id = 'Git.Git'
            WingetArguments = @()
        } -Detector { param($Package) $false } -Runner {
            param($FilePath, $Arguments)
            New-FakeCommandResult -ExitCode 0 -Command 'winget install'
        } -LogPath $outsidePath -Confirm:$false | Out-Null
    } catch {
        $caught = $_.Exception
    } finally {
        Remove-Item -LiteralPath $outsidePath -Force -ErrorAction SilentlyContinue
    }
    Assert-True ($null -ne $caught)
    Assert-Match 'must stay under' $caught.Message
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
