# Windows용 cloudflared 바이너리 다운로드
$ErrorActionPreference = "Stop"

$Target = "x86_64-pc-windows-msvc"
$Dest   = "src-tauri\binaries\cloudflared-$Target.exe"
$Url    = "https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-windows-amd64.exe"

New-Item -ItemType Directory -Force -Path "src-tauri\binaries" | Out-Null

Write-Host "Downloading cloudflared for $Target ..."
Invoke-WebRequest -Uri $Url -OutFile $Dest

Write-Host "OK  $Dest"
