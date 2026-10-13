param([string]$Repository = (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent))

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path -LiteralPath $Repository).Path
$scratch = Join-Path ([System.IO.Path]::GetTempPath()) ("ccp-nsis-test-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $scratch | Out-Null
$environmentFile = Join-Path $scratch "github-env.txt"
$invalidCompiler = Join-Path $scratch "invalid-compiler.ps1"
Set-Content -LiteralPath $invalidCompiler -Value '$global:LASTEXITCODE = 1' -Encoding utf8
$nsisTestState = @{}
$originalGithubEnv = $env:GITHUB_ENV
$env:GITHUB_ENV = $environmentFile

function choco {
    $nsisTestState.attempts += 1
    $nsisTestState.installed = $nsisTestState.attempts -ge $nsisTestState.readyOn
    $global:LASTEXITCODE = if ($nsisTestState.attempts -eq 1) { $nsisTestState.firstExit } else { 0 }
}
function Get-Command {
    [CmdletBinding()]
    param([string]$Name, [string]$CommandType)
    if ($Name -ne "makensis") { throw "Unexpected command lookup: $Name" }
    if ($nsisTestState.installed) { [pscustomobject]@{ Source = $nsisTestState.probePath } }
}
function Test-Path {
    [CmdletBinding()]
    param([string]$LiteralPath, [string]$PathType)
    if ($LiteralPath -eq $nsisTestState.probePath) {
        Microsoft.PowerShell.Management\Test-Path -LiteralPath $LiteralPath -PathType Leaf
    } else {
        $false
    }
}
function Start-Sleep {
    param([int]$Seconds)
    if ($Seconds -ne 10) { throw "Unexpected retry delay: $Seconds" }
    $nsisTestState.waits += 1
}

try {
    Push-Location $repo
    $cases = @(
        @{ Name = "zero exit without compiler"; ReadyOn = 99; FirstExit = 0; Attempts = 3; Success = $false },
        @{ Name = "retry after nonzero exit"; ReadyOn = 2; FirstExit = 1; Attempts = 2; Success = $true },
        @{ Name = "retry after false success"; ReadyOn = 2; FirstExit = 0; Attempts = 2; Success = $true },
        @{ Name = "first attempt succeeds"; ReadyOn = 1; FirstExit = 0; Attempts = 1; Success = $true },
        @{ Name = "compiler version check fails"; ReadyOn = 1; FirstExit = 0; Attempts = 3; Success = $false; Invalid = $true }
    )
    foreach ($workflow in @("auto-release-installers.yml", "release-assets.yml")) {
        $text = Get-Content (Join-Path $repo ".github/workflows/$workflow") -Raw
        $step = [regex]::Match($text, '(?m)^      - name: Install NSIS\r?\n(?:        shell: pwsh\r?\n)?        run: (?<command>[^\r\n]+)\r?$')
        if (-not $step.Success) { throw "Install NSIS entry missing: $workflow" }
        $command = [scriptblock]::Create($step.Groups["command"].Value)
        foreach ($case in $cases) {
            $nsisTestState.attempts = 0
            $nsisTestState.waits = 0
            $nsisTestState.installed = $false
            $nsisTestState.readyOn = $case.ReadyOn
            $nsisTestState.firstExit = $case.FirstExit
            $nsisTestState.probePath = if ($case.Invalid) { $invalidCompiler } else { Join-Path $PSHOME "pwsh.exe" }
            [System.IO.File]::WriteAllText($environmentFile, "")
            $success = $true
            try { & $command } catch { $success = $false }
            $exported = [System.IO.File]::ReadAllText($environmentFile)
            if ($nsisTestState.attempts -ne $case.Attempts -or $success -ne $case.Success -or $nsisTestState.waits -ne ($case.Attempts - 1)) {
                throw "$workflow / $($case.Name): attempts=$($nsisTestState.attempts) success=$success waits=$($nsisTestState.waits); expected attempts=$($case.Attempts) success=$($case.Success)"
            }
            if ($success -and -not $exported.Contains("MAKENSIS_EXE=$($nsisTestState.probePath)")) { throw "Verified compiler path was not exported" }
            if (-not $success -and $exported.Contains("MAKENSIS_EXE=")) { throw "Unverified compiler path was exported" }
            Write-Output "PASS $workflow / $($case.Name)"
        }
    }
    Write-Output "10 NSIS installation scenarios passed."
} finally {
    Pop-Location
    $env:GITHUB_ENV = $originalGithubEnv
    Remove-Item -LiteralPath $environmentFile, $invalidCompiler -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath $scratch
}