$ErrorActionPreference = "Stop"
$PSNativeCommandUseErrorActionPreference = $false

for ($attempt = 1; $attempt -le 3; $attempt++) {
    & choco install nsis -y --no-progress
    $candidates = @(
        "${env:ProgramFiles(x86)}\NSIS\makensis.exe"
        "$env:ProgramFiles\NSIS\makensis.exe"
        (Get-Command makensis -CommandType Application -ErrorAction SilentlyContinue).Source
    )
    $makensis = $candidates | Where-Object {
        $_ -and (Test-Path -LiteralPath $_ -PathType Leaf)
    } | Select-Object -First 1
    if ($makensis) {
        & $makensis "/VERSION"
        if ($LASTEXITCODE -eq 0) {
            "MAKENSIS_EXE=$makensis" | Out-File -FilePath $env:GITHUB_ENV -Encoding utf8 -Append
            Write-Output "Verified NSIS compiler: $makensis"
            return
        }
    }
    if ($attempt -lt 3) {
        Write-Warning "NSIS compiler is not ready after attempt $attempt; retrying in 10 seconds."
        Start-Sleep -Seconds 10
    }
}

throw "NSIS installation failed after 3 attempts: makensis is missing or its version check failed."