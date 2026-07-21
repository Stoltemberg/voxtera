Set-StrictMode -Version Latest
$script:Failures = 0
$script:Executed = 0

function Test-Case {
    param([Parameter(Mandatory)][string]$Name, [Parameter(Mandatory)][scriptblock]$Body)
    $script:Executed++
    try {
        & $Body
        Write-Host "PASS  $Name"
    } catch {
        $script:Failures++
        Write-Host "FAIL  $Name`n      $($_.Exception.Message)" -ForegroundColor Red
    }
}

function Assert-True {
    param([Parameter(Mandatory)]$Value, [string]$Message = 'Expected value to be true.')
    if (-not [bool]$Value) { throw $Message }
}

function Assert-Equal {
    param([Parameter(Mandatory)]$Expected, [Parameter(Mandatory)]$Actual)
    if ($Expected -ne $Actual) {
        throw "Expected '$Expected' but received '$Actual'."
    }
}

function Assert-Match {
    param([Parameter(Mandatory)][string]$Pattern, [Parameter(Mandatory)][string]$Actual)
    if ($Actual -notmatch $Pattern) {
        throw "Expected '$Actual' to match '$Pattern'."
    }
}

function Complete-TestRun {
    Write-Host "`nExecuted: $script:Executed  Failed: $script:Failures"
    if ($script:Failures -gt 0) { exit 1 }
    exit 0
}

Export-ModuleMember -Function Test-Case, Assert-True, Assert-Equal, Assert-Match, Complete-TestRun
