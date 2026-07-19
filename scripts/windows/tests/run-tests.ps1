[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
Import-Module (Join-Path $PSScriptRoot 'TestHarness.psm1') -Force

Get-ChildItem -LiteralPath $PSScriptRoot -Filter '*.Tests.ps1' |
    Sort-Object Name |
    ForEach-Object { . $_.FullName }

Complete-TestRun
