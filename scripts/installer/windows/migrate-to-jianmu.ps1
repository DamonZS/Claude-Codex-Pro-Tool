param([string]$Dest)
$ErrorActionPreference = 'Stop'
try {
  [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
  $feed = Invoke-RestMethod -Uri 'https://toporeduce.cn/api/latest' -TimeoutSec 30
  $a = $feed.assets.'jianmu-windows-x64.exe'
  if (-not $a -or -not $a.sha256 -or -not $a.url) { throw '更新源缺少 jianmu-windows-x64.exe 或其 sha256' }
  if (-not ([string]$a.url).StartsWith('https://')) { throw 'Jianmu 下载地址不是 https' }
  $tmp = Join-Path $env:TEMP 'jianmu-migrate.exe'
  Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue
  Invoke-WebRequest -Uri $a.url -OutFile $tmp -UseBasicParsing -TimeoutSec 1800
  if ($a.size -and (Get-Item -LiteralPath $tmp).Length -ne [int64]$a.size) { throw 'Jianmu 下载不完整，大小不一致' }
  $hash = (Get-FileHash -LiteralPath $tmp -Algorithm SHA256).Hash
  if ($hash -ine ([string]$a.sha256)) { Remove-Item -LiteralPath $tmp -Force; throw 'Jianmu 校验失败，sha256 不一致' }
  Get-Process -Name jianmu -ErrorAction SilentlyContinue | Stop-Process -Force
  New-Item -ItemType Directory -Force -Path (Split-Path -Parent $Dest) | Out-Null
  Move-Item -LiteralPath $tmp -Destination $Dest -Force
  Write-Output 'ok'
  exit 0
} catch {
  Write-Output $_.Exception.Message
  exit 1
}
