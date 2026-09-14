$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location $Root
New-Item -ItemType Directory -Force -Path (Join-Path $Root "dist\windows") | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $Root "resources") | Out-Null

Write-Host "==> rustup target"
rustup target add x86_64-pc-windows-msvc

Write-Host "==> cargo test engine + ime-win"
cargo test -p engine --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw "cargo test -p engine failed" }
cargo test -p ime-win --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw "cargo test -p ime-win failed" }

Write-Host "==> cargo build release"
cargo build --release -p ime-win --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw "cargo build ime-win failed" }
cargo build --release -p dict-compiler --target x86_64-pc-windows-msvc
if ($LASTEXITCODE -ne 0) { throw "cargo build dict-compiler failed" }

$Release = Join-Path $Root "target\x86_64-pc-windows-msvc\release"
$Dll = Join-Path $Release "ime_win.dll"
if (-not (Test-Path $Dll)) {
    $Dll = Join-Path $Root "target\release\ime_win.dll"
}
if (-not (Test-Path $Dll)) {
    throw "ime_win.dll not found"
}

$Dict = Join-Path $Root "resources\system.dict"
if (-not (Test-Path $Dict)) {
    $Compiler = Join-Path $Release "dict-compiler.exe"
    if (-not (Test-Path $Compiler)) {
        $Compiler = Join-Path $Root "target\release\dict-compiler.exe"
    }
    $Tsv = Join-Path $Root "data\system.tsv"
    if (-not (Test-Path $Tsv)) {
        $Tsv = Join-Path $Root "data\builtin.tsv"
    }
    Write-Host "==> compiling system.dict from $Tsv"
    & $Compiler --system $Tsv -o $Dict
}

$Stage = Join-Path $Root "dist\windows"
Copy-Item -Force $Dll (Join-Path $Stage "ime_win.dll")
Copy-Item -Force $Dict (Join-Path $Stage "system.dict")
Copy-Item -Force (Join-Path $Root "windows\README.md") (Join-Path $Stage "README.txt")
Copy-Item -Force (Join-Path $Root "windows\installer\register.ps1") (Join-Path $Stage "register.ps1")
Copy-Item -Force (Join-Path $Root "windows\installer\unregister.ps1") (Join-Path $Stage "unregister.ps1")

$Iscc = @(
    "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
    "${env:ProgramFiles}\Inno Setup 6\ISCC.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1

if ($Iscc) {
    Write-Host "==> Inno Setup $Iscc"
    & $Iscc (Join-Path $Root "windows\installer\buluo.iss")
    if ($LASTEXITCODE -ne 0) { throw "ISCC failed with $LASTEXITCODE" }
    $Built = Join-Path $Root "BuluoIME-windows.exe"
    if (-not (Test-Path $Built)) {
        throw "ISCC did not produce BuluoIME-windows.exe"
    }
    $Final = Join-Path $Root "部落输入法-windows.exe"
    Copy-Item -Force $Built $Final
    Write-Host "installer: $Final (and $Built)"
} else {
    Write-Host "==> Inno Setup missing; zip only"
}

Write-Host "==> zip copy at repo root"
$Zip = Join-Path $Root "部落输入法-windows.zip"
if (Test-Path $Zip) { Remove-Item -Force $Zip }
Compress-Archive -Path (Join-Path $Stage "*") -DestinationPath $Zip
Copy-Item -Force $Zip (Join-Path $Root "BuluoIME-windows.zip")
Write-Host "zip: $Zip"
