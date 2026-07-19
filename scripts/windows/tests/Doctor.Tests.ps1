$windowsRoot = Split-Path $PSScriptRoot -Parent
Import-Module (Join-Path $windowsRoot 'Bootstrap.Common.psm1') -Force -DisableNameChecking

function Test-ForbiddenCargoArguments {
    param([string[]]$Arguments = @())
    $commandIndex = 0
    while ($commandIndex -lt $Arguments.Count -and
        $Arguments[$commandIndex].StartsWith('+')) {
        $commandIndex++
    }
    $commandIndex -lt $Arguments.Count -and
        $Arguments[$commandIndex].ToLowerInvariant() -in @(
            'build',
            'check',
            'test',
            'run'
        )
}

function Test-ForbiddenRustupArguments {
    param([string[]]$Arguments = @())
    if ($Arguments.Count -eq 0 -or
        $Arguments[0].ToLowerInvariant() -ne 'run') {
        return $false
    }
    for ($index = 1; $index -lt $Arguments.Count; $index++) {
        if ($Arguments[$index].ToLowerInvariant() -in @('cargo', 'cargo.exe')) {
            $cargoArguments = @($Arguments | Select-Object -Skip ($index + 1))
            return Test-ForbiddenCargoArguments -Arguments $cargoArguments
        }
    }
    $false
}

function Test-ForbiddenVelorenSource {
    param([Parameter(Mandatory)][string]$Source)
    $tokens = $null
    $errors = $null
    $ast = [System.Management.Automation.Language.Parser]::ParseInput(
        $Source,
        [ref]$tokens,
        [ref]$errors
    )
    if ($errors.Count -gt 0) {
        throw "Cannot scan invalid PowerShell source: $($errors[0].Message)"
    }

    $forbiddenExecutables = @(
        'veloren-voxygen',
        'veloren-voxygen.exe',
        'veloren-server-cli',
        'veloren-server-cli.exe'
    )
    $commands = $ast.FindAll(
        {
            param($candidate)
            $candidate -is [System.Management.Automation.Language.CommandAst]
        },
        $true
    )
    foreach ($command in $commands) {
        $commandName = $command.GetCommandName()
        if ([string]::IsNullOrWhiteSpace($commandName)) {
            continue
        }
        $executable = [System.IO.Path]::GetFileName(
            $commandName.Replace('/', '\')
        ).ToLowerInvariant()

        $literalArguments = New-Object System.Collections.Generic.List[string]
        for ($index = 1; $index -lt $command.CommandElements.Count; $index++) {
            $strings = $command.CommandElements[$index].FindAll(
                {
                    param($candidate)
                    $candidate -is [System.Management.Automation.Language.StringConstantExpressionAst]
                },
                $true
            )
            foreach ($string in $strings) {
                $literalArguments.Add($string.Value) | Out-Null
            }
        }

        if ($forbiddenExecutables -contains $executable) {
            return $true
        }
        if ($executable -in @('cargo', 'cargo.exe') -and
            (Test-ForbiddenCargoArguments -Arguments $literalArguments)) {
            return $true
        }
        if ($executable -in @('rustup', 'rustup.exe') -and
            (Test-ForbiddenRustupArguments -Arguments $literalArguments)) {
            return $true
        }
        if ($executable -eq 'invoke-externalcommand') {
            $wrappedExecutable = $null
            $wrappedArguments = @($literalArguments)
            if ($command.CommandElements.Count -gt 1) {
                $targetStrings = $command.CommandElements[1].FindAll(
                    {
                        param($candidate)
                        $candidate -is [System.Management.Automation.Language.StringConstantExpressionAst]
                    },
                    $true
                )
                if ($targetStrings.Count -gt 0) {
                    $wrappedExecutable = [System.IO.Path]::GetFileName(
                        $targetStrings[0].Value.Replace('/', '\')
                    ).ToLowerInvariant()
                    $wrappedArguments = @(
                        $literalArguments | Select-Object -Skip 1
                    )
                }
            }

            if ($forbiddenExecutables -contains $wrappedExecutable) {
                return $true
            }
            if ($wrappedExecutable -in @('cargo', 'cargo.exe') -and
                (Test-ForbiddenCargoArguments -Arguments $wrappedArguments)) {
                return $true
            }
            if (($wrappedExecutable -in @('rustup', 'rustup.exe') -or
                $null -eq $wrappedExecutable) -and
                (Test-ForbiddenRustupArguments -Arguments $wrappedArguments)) {
                return $true
            }
        }
    }
    $false
}

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

Test-Case 'default Python probe requires managed Python 3.13 and rejects Store alias' {
    $runnerCalls = 0
    $storeProbes = Get-DefaultDoctorProbes `
        -RepositoryRoot (Get-RepositoryRoot) `
        -CommandResolver {
            param($Command)
            if ($Command -eq 'python.exe') {
                return [pscustomobject]@{
                    Source = 'C:\Users\test\AppData\Local\Microsoft\WindowsApps\python.exe'
                }
            }
            $null
        } `
        -CommandRunner {
            $runnerCalls++
            throw 'Store alias must not execute.'
        }.GetNewClosure()
    $storeResult = & $storeProbes.Python
    Assert-Equal 'FAIL' $storeResult.Status
    Assert-Equal 0 $runnerCalls

    foreach ($version in @('Python 2.7.18', 'Python 3.12.10', 'Python 3.14.0')) {
        $wrongVersionProbes = Get-DefaultDoctorProbes `
            -RepositoryRoot (Get-RepositoryRoot) `
            -CommandResolver {
                [pscustomobject]@{ Source = 'C:\Python\python.exe' }
            } `
            -CommandRunner {
                [pscustomobject]@{
                    ExitCode = 0
                    Output = @($version)
                    Command = 'python.exe --version'
                }
            }.GetNewClosure()
        Assert-Equal 'FAIL' (& $wrongVersionProbes.Python).Status
    }

    $readyProbes = Get-DefaultDoctorProbes `
        -RepositoryRoot (Get-RepositoryRoot) `
        -CommandResolver {
            [pscustomobject]@{ Source = 'C:\Python313\python.exe' }
        } `
        -CommandRunner {
            [pscustomobject]@{
                ExitCode = 0
                Output = @('Python 3.13.5')
                Command = 'python.exe --version'
            }
        }
    Assert-Equal 'PASS' (& $readyProbes.Python).Status
}

Test-Case 'doctor entrypoint rejects a Python Store alias' {
    $doctor = (Join-Path $windowsRoot 'doctor.ps1').Replace("'", "''")
    $command = @"
& {
    function global:Get-Command {
        param(`$Name, `$ErrorAction)
        if (`$Name -eq 'python.exe') {
            return [pscustomobject]@{
                Source = 'C:\Users\test\AppData\Local\Microsoft\WindowsApps\python.exe'
            }
        }
        Microsoft.PowerShell.Core\Get-Command -Name `$Name -ErrorAction `$ErrorAction
    }
    & '$doctor' -Json
}
"@
    $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -Command $command
    $exitCode = $LASTEXITCODE
    $document = ($output -join "`n") | ConvertFrom-Json
    $python = $document.Checks | Where-Object Name -eq Python
    Assert-Equal $false $document.Healthy
    Assert-Equal 'FAIL' $python.Status
    Assert-Match 'WindowsApps' $python.Detail
}

Test-Case 'Git LFS configuration requires all effective filter values' {
    $expected = @{
        'filter.lfs.clean' = 'git-lfs clean -- %f'
        'filter.lfs.smudge' = 'git-lfs smudge -- %f'
        'filter.lfs.process' = 'git-lfs filter-process'
        'filter.lfs.required' = 'true'
    }
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($FilePath, $Arguments)
        $key = $Arguments[-1]
        $calls.Add("$FilePath $($Arguments -join ' ')") | Out-Null
        [pscustomobject]@{
            ExitCode = 0
            Output = @($expected[$key])
            Command = "$FilePath $($Arguments -join ' ')"
        }
    }.GetNewClosure()

    $result = Get-GitLfsConfigCheck `
        -CommandResolver {
            [pscustomobject]@{ Source = 'git.exe' }
        } `
        -Runner $runner

    Assert-Equal 'PASS' $result.Status
    Assert-Equal 4 $calls.Count
    Assert-True ($calls[0].EndsWith(' config --get filter.lfs.clean'))
    Assert-True ($calls[1].EndsWith(' config --get filter.lfs.smudge'))
    Assert-True ($calls[2].EndsWith(' config --get filter.lfs.process'))
    Assert-True ($calls[3].EndsWith(' config --get filter.lfs.required'))
}

Test-Case 'default Git LFS configuration probe anchors every read to repository root' {
    $root = Get-RepositoryRoot
    $calls = New-Object System.Collections.Generic.List[string]
    $values = @{
        'filter.lfs.clean' = 'git-lfs clean -- %f'
        'filter.lfs.smudge' = 'git-lfs smudge -- %f'
        'filter.lfs.process' = 'git-lfs filter-process'
        'filter.lfs.required' = 'true'
    }
    $probes = Get-DefaultDoctorProbes `
        -RepositoryRoot $root `
        -CommandResolver {
            [pscustomobject]@{ Source = 'git.exe' }
        } `
        -CommandRunner {
            param($FilePath, $Arguments)
            $calls.Add(($Arguments -join ' ')) | Out-Null
            [pscustomobject]@{
                ExitCode = 0
                Output = @($values[$Arguments[-1]])
                Command = "$FilePath $($Arguments -join ' ')"
            }
        }.GetNewClosure()
    $result = & $probes.GitLfsConfig
    Assert-Equal 'PASS' $result.Status
    Assert-Equal 4 $calls.Count
    foreach ($call in $calls) {
        Assert-Match ("^-C $([regex]::Escape($root)) config --get filter\.lfs\.") $call
    }
}

Test-Case 'Git LFS configuration fails for an ineffective filter value' {
    $runner = {
        param($FilePath, $Arguments)
        $values = @{
            'filter.lfs.clean' = 'git-lfs clean -- %f'
            'filter.lfs.smudge' = 'git-lfs smudge -- %f'
            'filter.lfs.process' = 'not-git-lfs'
            'filter.lfs.required' = 'true'
        }
        [pscustomobject]@{
            ExitCode = 0
            Output = @($values[$Arguments[-1]])
            Command = "$FilePath $($Arguments -join ' ')"
        }
    }
    $result = Get-GitLfsConfigCheck `
        -CommandResolver {
            [pscustomobject]@{ Source = 'git.exe' }
        } `
        -Runner $runner
    Assert-Equal 'FAIL' $result.Status
    Assert-Match 'filter.lfs.process' $result.Detail
}

Test-Case 'Git LFS configuration reports missing Git without invoking a runner' {
    $calls = 0
    $result = Get-GitLfsConfigCheck `
        -CommandResolver { $null } `
        -Runner {
            $calls++
            throw 'must not run'
        }.GetNewClosure()
    Assert-Equal 'FAIL' $result.Status
    Assert-Match 'Git is unavailable' $result.Detail
    Assert-Equal 0 $calls
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

Test-Case 'unexpected probe errors propagate' {
    $probes = [ordered]@{
        Internal = { throw 'unexpected internal failure' }
    }
    $caught = $null
    try {
        @(Get-DoctorReport -Probes $probes) | Out-Null
    } catch {
        $caught = $_.Exception
    }
    Assert-True ($null -ne $caught)
    Assert-Match 'unexpected internal failure' $caught.Message
}

Test-Case 'non-Windows platforms are unsupported' {
    $classification = Get-PlatformClassification `
        -Platform ([PlatformID]::Unix) -Is64BitOperatingSystem $true -MajorVersion 10 -ProductType 1
    Assert-Equal $false $classification.Supported
    Assert-Match 'Windows' $classification.Detail
}

Test-Case '32-bit Windows is unsupported' {
    $classification = Get-PlatformClassification `
        -Platform ([PlatformID]::Win32NT) -Is64BitOperatingSystem $false -MajorVersion 10 -ProductType 1
    Assert-Equal $false $classification.Supported
    Assert-Match '64-bit' $classification.Detail
}

Test-Case 'Windows versions before 10 are unsupported' {
    $classification = Get-PlatformClassification `
        -Platform ([PlatformID]::Win32NT) -Is64BitOperatingSystem $true -MajorVersion 6 -ProductType 1
    Assert-Equal $false $classification.Supported
    Assert-Match 'Windows 10/11' $classification.Detail
}

Test-Case 'NT major versions after 10 are unsupported' {
    $classification = Get-PlatformClassification `
        -Platform ([PlatformID]::Win32NT) -Is64BitOperatingSystem $true -MajorVersion 11 -ProductType 1
    Assert-Equal $false $classification.Supported
    Assert-Match 'NT major version 10' $classification.Detail
}

Test-Case 'NT major version 11 is fatal at the platform check' {
    $caught = $null
    try {
        Get-PlatformCheck `
            -Platform ([PlatformID]::Win32NT) `
            -Is64BitOperatingSystem $true `
            -MajorVersion 11 `
            -VersionString 'Microsoft Windows NT 11.0' `
            -ProductTypeProvider { 1 } | Out-Null
    } catch {
        $caught = $_.Exception
    }
    Assert-True ($null -ne $caught)
    Assert-Match 'NT major version 10' $caught.Message
}

Test-Case 'Windows Server is unsupported' {
    $classification = Get-PlatformClassification `
        -Platform ([PlatformID]::Win32NT) -Is64BitOperatingSystem $true -MajorVersion 10 -ProductType 3
    Assert-Equal $false $classification.Supported
    Assert-Match 'client' $classification.Detail
}

Test-Case '64-bit Windows 10 workstation is supported' {
    $classification = Get-PlatformClassification `
        -Platform ([PlatformID]::Win32NT) -Is64BitOperatingSystem $true -MajorVersion 10 -ProductType 1
    Assert-Equal $true $classification.Supported
}

Test-Case 'non-Windows platform check does not query a Windows SKU' {
    $caught = $null
    try {
        Get-PlatformCheck `
            -Platform ([PlatformID]::Unix) `
            -Is64BitOperatingSystem $true `
            -MajorVersion 10 `
            -VersionString 'Unix' `
            -ProductTypeProvider { throw 'Windows SKU provider must not run.' } | Out-Null
    } catch {
        $caught = $_.Exception
    }
    Assert-True ($null -ne $caught)
    Assert-Match 'Only Windows is supported' $caught.Message
}

Test-Case 'pinned toolchain validates cargo and rustc through rustup run' {
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($Invocation)
        $call = $Invocation.Arguments -join ' '
        $calls.Add($call) | Out-Null
        if ($call -eq 'show') {
            return [pscustomobject]@{
                ExitCode = 0
                Output = @('Default host: x86_64-pc-windows-msvc')
            }
        }
        [pscustomobject]@{
            ExitCode = 0
            Output = @("$($Invocation.Arguments[2]) 1.0.0")
        }
    }.GetNewClosure()
    $root = Get-RepositoryRoot
    $pinned = Get-PinnedToolchain $root
    $result = Get-PinnedToolchainCheck `
        -RepositoryRoot $root -RustupPath 'rustup-test.exe' -Runner $runner
    Assert-Equal 'PASS' $result.Status
    Assert-Equal 3 $calls.Count
    Assert-Equal 'show' $calls[0]
    Assert-Equal "run $pinned-x86_64-pc-windows-msvc cargo --version" $calls[1]
    Assert-Equal "run $pinned-x86_64-pc-windows-msvc rustc --version" $calls[2]
}

Test-Case 'pinned toolchain list accepts only the exact rustup default host' {
    $pin = 'nightly-2026-06-13'
    $defaultHost = 'x86_64-pc-windows-msvc'
    Assert-Equal $true (
        Test-PinnedRustToolchainListed `
            -Pinned $pin `
            -DefaultHost $defaultHost `
            -Entries @("$pin-$defaultHost (active)")
    )
    foreach ($entry in @(
        $pin,
        "$pin-x86_64-pc-windows-gnu",
        "$pin-aarch64-pc-windows-msvc"
    )) {
        Assert-Equal $false (
            Test-PinnedRustToolchainListed `
                -Pinned $pin `
                -DefaultHost $defaultHost `
                -Entries @($entry)
        )
    }
}

Test-Case 'Rust component doctor queries the exact default-host toolchain' {
    $root = Get-RepositoryRoot
    $pin = Get-PinnedToolchain $root
    $defaultHost = 'x86_64-pc-windows-msvc'
    $calls = New-Object System.Collections.Generic.List[string]
    $runner = {
        param($Invocation)
        $call = $Invocation.Arguments -join ' '
        $calls.Add($call) | Out-Null
        if ($call -eq 'show') {
            return [pscustomobject]@{
                ExitCode = 0
                Output = @("Default host: $defaultHost")
            }
        }
        [pscustomobject]@{
            ExitCode = 0
            Output = @('rustfmt (installed)', 'clippy (installed)')
        }
    }.GetNewClosure()
    $result = Get-RustComponentsCheck `
        -RepositoryRoot $root `
        -RustupPath 'rustup-test.exe' `
        -Runner $runner
    Assert-Equal 'PASS' $result.Status
    Assert-Equal "component list --toolchain $pin-$defaultHost --installed" $calls[1]
}

Test-Case 'pinned toolchain fails when pinned cargo is unusable' {
    $runner = {
        param($Invocation)
        if (($Invocation.Arguments -join ' ') -eq 'show') {
            return [pscustomobject]@{
                ExitCode = 0
                Output = @('Default host: x86_64-pc-windows-msvc')
            }
        }
        $exitCode = if ($Invocation.Arguments[2] -eq 'cargo') { 1 } else { 0 }
        [pscustomobject]@{
            ExitCode = $exitCode
            Output = @("simulated $($Invocation.Arguments[2]) result")
        }
    }
    $result = Get-PinnedToolchainCheck `
        -RepositoryRoot (Get-RepositoryRoot) -RustupPath 'rustup-test.exe' -Runner $runner
    Assert-Equal 'FAIL' $result.Status
    Assert-Match 'cargo' $result.Detail
}

Test-Case 'default doctor uses one-result Rust probes in order' {
    $probes = Get-DefaultDoctorProbes -RepositoryRoot (Get-RepositoryRoot)
    $keys = @($probes.Keys)
    Assert-True ($keys -contains 'Rustup')
    Assert-True ($keys -contains 'PinnedToolchain')
    Assert-True ($keys -contains 'RustComponents')
    Assert-True (-not ($keys -contains 'Rust'))
    Assert-Equal 'Cargo' $keys[11]
    Assert-Equal 'Rustup' $keys[12]
    Assert-Equal 'PinnedToolchain' $keys[13]
    Assert-Equal 'RustComponents' $keys[14]
    Assert-Equal 'Restart' $keys[15]
}

Test-Case 'default Cargo probe reports its resolved shim without executing it' {
    $fakeCargoPath = 'C:\test\ambient-cargo-must-not-run.exe'
    $resolver = {
        param($Command)
        if ($Command -ne 'cargo.exe') { throw "Unexpected command lookup: $Command" }
        [pscustomobject]@{ Source = $fakeCargoPath }
    }.GetNewClosure()
    $probes = Get-DefaultDoctorProbes `
        -RepositoryRoot (Get-RepositoryRoot) -CommandResolver $resolver
    $result = & $probes.Cargo
    Assert-Equal 'PASS' $result.Status
    Assert-Equal $fakeCargoPath $result.Detail
}

Test-Case 'unsupported Windows Server is fatal at the entrypoint' {
    $doctor = (Join-Path $windowsRoot 'doctor.ps1').Replace("'", "''")
    $command = "& { function global:Get-CimInstance { [pscustomobject]@{ ProductType = 3 } }; & '$doctor'; exit `$LASTEXITCODE }"
    $previousErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -Command $command 2>&1
    $exitCode = $LASTEXITCODE
    $ErrorActionPreference = $previousErrorActionPreference
    Assert-Equal 2 $exitCode
    Assert-Match 'Windows Server is unsupported' ($output -join "`n")
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

Test-Case 'production scripts never compile or run Veloren' {
    $productionFiles = @(
        (Join-Path $windowsRoot 'bootstrap.ps1'),
        (Join-Path $windowsRoot 'doctor.ps1'),
        (Join-Path $windowsRoot 'Bootstrap.Common.psm1')
    )
    foreach ($file in $productionFiles) {
        $content = Get-Content -LiteralPath $file -Raw
        Assert-True (-not (Test-ForbiddenVelorenSource -Source $content)) `
            "$file compiles or runs Veloren."
    }
}

Test-Case 'scope guard rejects call-operator Cargo builds' {
    Assert-True (Test-ForbiddenVelorenSource -Source '& cargo.exe build')
}

Test-Case 'scope guard rejects quoted Cargo path tests' {
    Assert-True (
        Test-ForbiddenVelorenSource `
            -Source "& 'C:\Program Files\Rust\cargo.exe' test --workspace"
    )
}

Test-Case 'scope guard rejects wrapped Cargo builds' {
    Assert-True (
        Test-ForbiddenVelorenSource `
            -Source "Invoke-ExternalCommand 'cargo.exe' @('build')"
    )
}

Test-Case 'scope guard permits Cargo version probes' {
    Assert-Equal $false (
        Test-ForbiddenVelorenSource `
            -Source "Invoke-ExternalCommand 'cargo.exe' @('--version')"
    )
}

Test-Case 'scope guard ignores explanatory text and comments' {
    $source = @"
# cargo test is intentionally outside this bootstrap.
Write-Host 'Do not run cargo build from this script.'
"@
    Assert-Equal $false (Test-ForbiddenVelorenSource -Source $source)
}

Test-Case 'scope guard rejects Cargo builds after a toolchain selector' {
    Assert-True (
        Test-ForbiddenVelorenSource -Source '& cargo.exe +nightly build'
    )
}

Test-Case 'scope guard rejects direct Rustup Cargo builds' {
    Assert-True (
        Test-ForbiddenVelorenSource `
            -Source 'rustup.exe run nightly-2026-06-13 cargo build'
    )
}

Test-Case 'scope guard rejects wrapped Rustup Cargo builds' {
    $source = @'
Invoke-ExternalCommand 'rustup.exe' @('run', $pin, 'cargo', 'build')
'@
    Assert-True (Test-ForbiddenVelorenSource -Source $source)
}

Test-Case 'scope guard rejects direct Voxygen launches' {
    Assert-True (
        Test-ForbiddenVelorenSource `
            -Source "& '.\target\debug\veloren-voxygen.exe'"
    )
}

Test-Case 'scope guard rejects wrapped server launches' {
    Assert-True (
        Test-ForbiddenVelorenSource `
            -Source "Invoke-ExternalCommand 'veloren-server-cli.exe' @()"
    )
}

Test-Case 'scope guard permits Rustup Cargo version probes' {
    $source = @'
Invoke-ExternalCommand $RustupPath @('run', $pin, 'cargo', '--version')
'@
    Assert-Equal $false (Test-ForbiddenVelorenSource -Source $source)
}
