#Requires -RunAsAdministrator
$ErrorActionPreference = "SilentlyContinue"
$Clsid = "{7B4C0E21-9F3A-4D6B-8E11-2C9F5A7B4D01}"

Remove-Item -Path "HKLM:\SOFTWARE\Microsoft\CTF\TIP\$Clsid" -Recurse -Force
Remove-Item -Path "HKLM:\SOFTWARE\Classes\CLSID\$Clsid" -Recurse -Force

$Dll = Join-Path $PSScriptRoot "ime_win.dll"
if (Test-Path $Dll) {
    # Harmless until DllUnregisterServer does real COM teardown.
    & "$env:SystemRoot\System32\regsvr32.exe" /s /u $Dll
}

Write-Host "Unregistered 部落输入法 TSF keys."
