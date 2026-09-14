#define MyAppName "部落输入法"
#define MyAppVersion "0.1.0"
#define SourceDir "..\..\dist\windows"

[Setup]
AppId={{7B4C0E21-9F3A-4D6B-8E11-2C9F5A7B4D00}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppName}
DefaultDirName={autopf}\BuluoIME
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=..\..
OutputBaseFilename=BuluoIME-windows
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
UninstallDisplayName={#MyAppName}
InfoBeforeFile=infobefore.txt

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "{#SourceDir}\ime_win.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\system.dict"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.txt"; DestDir: "{app}"; Flags: ignoreversion isreadme
Source: "{#SourceDir}\register.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\unregister.ps1"; DestDir: "{app}"; Flags: ignoreversion

[Registry]
Root: HKLM; Subkey: "SOFTWARE\Classes\CLSID\{7B4C0E21-9F3A-4D6B-8E11-2C9F5A7B4D01}"; ValueType: string; ValueName: ""; ValueData: "部落输入法"; Flags: uninsdeletekey
Root: HKLM; Subkey: "SOFTWARE\Classes\CLSID\{7B4C0E21-9F3A-4D6B-8E11-2C9F5A7B4D01}\InprocServer32"; ValueType: string; ValueName: ""; ValueData: "{app}\ime_win.dll"
Root: HKLM; Subkey: "SOFTWARE\Classes\CLSID\{7B4C0E21-9F3A-4D6B-8E11-2C9F5A7B4D01}\InprocServer32"; ValueType: string; ValueName: "ThreadingModel"; ValueData: "Apartment"
Root: HKLM; Subkey: "SOFTWARE\Microsoft\CTF\TIP\{7B4C0E21-9F3A-4D6B-8E11-2C9F5A7B4D01}"; ValueType: string; ValueName: ""; ValueData: "部落输入法"; Flags: uninsdeletekey
Root: HKLM; Subkey: "SOFTWARE\Microsoft\CTF\TIP\{7B4C0E21-9F3A-4D6B-8E11-2C9F5A7B4D01}\LanguageProfile\0x00000804\{7B4C0E21-9F3A-4D6B-8E11-2C9F5A7B4D02}"; ValueType: dword; ValueName: "Enable"; ValueData: 1
Root: HKLM; Subkey: "SOFTWARE\Microsoft\CTF\TIP\{7B4C0E21-9F3A-4D6B-8E11-2C9F5A7B4D01}\LanguageProfile\0x00000804\{7B4C0E21-9F3A-4D6B-8E11-2C9F5A7B4D02}"; ValueType: string; ValueName: "Description"; ValueData: "部落输入法"

[Icons]
Name: "{group}\注册 部落输入法"; Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\register.ps1"" -DllPath ""{app}\ime_win.dll"""
Name: "{group}\卸载注册"; Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\unregister.ps1"""

[Run]
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\register.ps1"" -DllPath ""{app}\ime_win.dll"""; Flags: runhidden waituntilterminated

[UninstallRun]
Filename: "powershell.exe"; Parameters: "-NoProfile -ExecutionPolicy Bypass -File ""{app}\unregister.ps1"""; Flags: runhidden waituntilterminated; RunOnceId: "UnregBuluo"
