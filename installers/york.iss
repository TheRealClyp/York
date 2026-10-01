[Setup]
AppName=York
AppVersion=1.1.3
AppPublisher=York Contributors
AppPublisherURL=https://github.com/TheRealClyp/York
AppSupportURL=https://github.com/TheRealClyp/York/issues
AppUpdatesURL=https://github.com/TheRealClyp/York/releases
AppCopyright=2026 York Contributors
DefaultDirName={userappdata}\Programs\york
DisableProgramGroupPage=yes
LicenseFile=..\LICENSE
OutputDir=.
OutputBaseFilename=york-setup-x64
SetupIconFile=..\assets\logo.ico
UninstallDisplayIcon={app}\bin\york.exe
SolidCompression=yes
Compression=lzma2/max
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
UninstallDisplayName=York 1.1.3
UninstallFilesDir={app}\uninstall
VersionInfoVersion=1.1.1.0
VersionInfoCompany=York Contributors
VersionInfoDescription=York Programming Language Setup
VersionInfoProductName=York
VersionInfoProductVersion=1.1.1

[Dirs]
Name: {app}\bin

[Files]
Source: ..\target\release\york.exe; DestDir: {app}\bin; DestName: york.exe; Flags: ignoreversion
Source: ..\target\release\ypkg.exe; DestDir: {app}\bin; DestName: ypkg.exe; Flags: ignoreversion
Source: ..\target\release\york-lsp.exe; DestDir: {app}\bin; DestName: york-lsp.exe; Flags: ignoreversion
Source: ..\target\release\yc.exe; DestDir: {app}\bin; DestName: yc.exe; Flags: ignoreversion
Source: ..\LICENSE; DestDir: {app}; Flags: ignoreversion isreadme
Source: uninstall.ps1; DestDir: {app}; Flags: ignoreversion

[Run]
Filename: "https://york-lang.org"; Flags: shellexec nowait postinstall skipifsilent unchecked; Description: "Open the York website"

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
var
  Bin: string;
  OrigPath: string;
  NewPath: string;
begin
  if CurStep <> ssPostInstall then Exit;
  Bin := ExpandConstant('{app}\bin');
  if RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath) then
  begin
    if Pos(LowerCase(Bin), LowerCase(';' + OrigPath + ';')) <> 0 then Exit;
    NewPath := OrigPath;
    if (Length(OrigPath) > 0) and (OrigPath[Length(OrigPath)] <> ';') then
      NewPath := NewPath + ';';
    NewPath := NewPath + Bin;
  end
  else
    NewPath := Bin;
  RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', NewPath);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Bin: string;
  OrigPath: string;
  P: Integer;
begin
  if CurUninstallStep <> usPostUninstall then Exit;
  Bin := LowerCase(ExpandConstant('{app}\bin'));
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath) then Exit;
  P := Pos(Bin, LowerCase(OrigPath));
  if P = 0 then Exit;
  Delete(OrigPath, P - 1 + Length(Bin) + 1, Length(Bin) + 1);
  if (Length(OrigPath) > 0) and (OrigPath[Length(OrigPath)] = ';') then
    SetLength(OrigPath, Length(OrigPath) - 1);
  RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath);
end;
