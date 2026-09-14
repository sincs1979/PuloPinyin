#Requires -RunAsAdministrator
param(
    [Parameter(Mandatory = $false)]
    [string]$DllPath
)

$ErrorActionPreference = "Stop"
$Clsid = "{7B4C0E21-9F3A-4D6B-8E11-2C9F5A7B4D01}"
$Profile = "{7B4C0E21-9F3A-4D6B-8E11-2C9F5A7B4D02}"
$Name = "部落输入法"

if (-not $DllPath) {
    $DllPath = Join-Path $PSScriptRoot "ime_win.dll"
}
$DllPath = [System.IO.Path]::GetFullPath($DllPath)
if (-not (Test-Path $DllPath)) {
    Write-Error "missing DLL: $DllPath"
}

# COM in-proc server (TSF loads this; ThreadingModel=Apartment).
New-Item -Path "HKLM:\SOFTWARE\Classes\CLSID\$Clsid" -Force | Out-Null
Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$Clsid" -Name "(default)" -Value $Name
New-Item -Path "HKLM:\SOFTWARE\Classes\CLSID\$Clsid\InprocServer32" -Force | Out-Null
Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$Clsid\InprocServer32" -Name "(default)" -Value $DllPath
Set-ItemProperty -Path "HKLM:\SOFTWARE\Classes\CLSID\$Clsid\InprocServer32" -Name "ThreadingModel" -Value "Apartment"

$Tip = "HKLM:\SOFTWARE\Microsoft\CTF\TIP\$Clsid"
New-Item -Path $Tip -Force | Out-Null
Set-ItemProperty -Path $Tip -Name "(default)" -Value $Name
$Lang = Join-Path $Tip "LanguageProfile\0x00000804\$Profile"
New-Item -Path $Lang -Force | Out-Null
New-ItemProperty -Path $Lang -Name "Enable" -PropertyType DWord -Value 1 -Force | Out-Null
Set-ItemProperty -Path $Lang -Name "Description" -Value $Name

Write-Host "Registered $Name"
Write-Host "  InprocServer32 = $DllPath"
Write-Host "This DLL is a TSF scaffold: DllGetClassObject is not a working TIP yet."
Write-Host "After a real ITfTextInputProcessor exists, also run: regsvr32 /s `"$DllPath`""
Write-Host "Then: Settings → Time & language → Typing → Input language → add 部落输入法"
