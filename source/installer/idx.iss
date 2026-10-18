; SPDX-License-Identifier: MIT
#define AppVersion "0.7.0"
#ifndef AppIdentity
  #define AppIdentity "{{1B881430-48C1-46D6-B4B7-CF428B946A76}"
#endif
#ifndef BuildOutputDir
  #define BuildOutputDir "..\target\release"
#endif

[Setup]
AppId={#AppIdentity}
AppName=IDX - Internet Downloader X
AppVersion={#AppVersion}
AppPublisher=IDX Contributors
DefaultDirName={localappdata}\Programs\IDX
DefaultGroupName=IDX - Internet Downloader X
DisableProgramGroupPage=yes
AllowNoIcons=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\idx.exe
SetupIconFile=..\assets\idx.ico
OutputDir=..\dist
OutputBaseFilename=IDX-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
LicenseFile=..\LICENSE
CloseApplications=yes
RestartApplications=no
SetupLogging=yes
VersionInfoVersion=0.7.0.0
VersionInfoDescription=IDX - Internet Downloader X Setup
VersionInfoCopyright=Copyright (c) 2026 IDX Contributors

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked

[Files]
Source: "{#BuildOutputDir}\idx.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#BuildOutputDir}\idx-native-host.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\media-tools\*"; DestDir: "{app}\media-tools"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\docs\*.md"; DestDir: "{app}\docs"; Flags: ignoreversion
Source: "..\docs\*.json"; DestDir: "{app}\docs"; Flags: ignoreversion
Source: "..\THIRD-PARTY-NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\assets\RESOURCE-NOTICE.txt"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\third-party-licenses\*"; DestDir: "{app}\third-party-licenses"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist
Source: "..\browser-extension\chromium\*"; DestDir: "{app}\browser-extension\chromium"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\IDX - Internet Downloader X"; Filename: "{app}\idx.exe"
Name: "{group}\Uninstall IDX"; Filename: "{uninstallexe}"
Name: "{userdesktop}\IDX - Internet Downloader X"; Filename: "{app}\idx.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\idx.exe"; Description: "Open IDX - Internet Downloader X"; Flags: nowait postinstall skipifsilent

[Code]
procedure RemoveOwnedNativeHost(const Vendor, BrowserName: String);
var
  KeyName, ManifestPath, RegisteredPath: String;
begin
  KeyName := 'Software\' + Vendor + '\NativeMessagingHosts\org.idx.downloader';
  ManifestPath := ExpandConstant('{app}\browser-extension\native-hosts\') + BrowserName + '.json';
  if RegQueryStringValue(HKCU32, KeyName, '', RegisteredPath) then
  begin
    if CompareText(RegisteredPath, ManifestPath) = 0 then
    begin
      RegDeleteValue(HKCU32, KeyName, '');
      RegDeleteKeyIfEmpty(HKCU32, KeyName);
    end;
  end;
  if RegQueryStringValue(HKCU64, KeyName, '', RegisteredPath) then
  begin
    if CompareText(RegisteredPath, ManifestPath) = 0 then
    begin
      RegDeleteValue(HKCU64, KeyName, '');
      RegDeleteKeyIfEmpty(HKCU64, KeyName);
    end;
  end;
  DeleteFile(ManifestPath);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
  begin
    RemoveOwnedNativeHost('Google\Chrome', 'chrome');
    RemoveOwnedNativeHost('Microsoft\Edge', 'edge');
    RemoveOwnedNativeHost('Mozilla', 'firefox');
    RemoveDir(ExpandConstant('{app}\browser-extension\native-hosts'));
  end;
end;
