$windowsRoot = Split-Path $PSScriptRoot -Parent
Import-Module (Join-Path $windowsRoot 'Bootstrap.Common.psm1') -Force -DisableNameChecking

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

    $forbiddenCargoCommands = @('build', 'check', 'test', 'run')
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
            $literalArguments.Count -gt 0 -and
            $literalArguments[0].ToLowerInvariant() -in $forbiddenCargoCommands) {
            return $true
        }
        if ($executable -eq 'invoke-externalcommand' -and
            $literalArguments.Count -gt 0) {
            $wrappedExecutable = [System.IO.Path]::GetFileName(
                $literalArguments[0].Replace('/', '\')
            ).ToLowerInvariant()
            if ($forbiddenExecutables -contains $wrappedExecutable) {
                return $true
            }
            if ($wrappedExecutable -in @('cargo', 'cargo.exe') -and
                $literalArguments.Count -gt 1 -and
                $literalArguments[1].ToLowerInvariant() -in $forbiddenCargoCommands) {
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
        $calls.Add(($Invocation.Arguments -join ' ')) | Out-Null
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
    Assert-Equal 2 $calls.Count
    Assert-Equal "run $pinned cargo --version" $calls[0]
    Assert-Equal "run $pinned rustc --version" $calls[1]
}

Test-Case 'pinned toolchain fails when pinned cargo is unusable' {
    $runner = {
        param($Invocation)
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
