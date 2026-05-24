#define MyAppName "Rachael's Transcriber"
#define MyAppExeName "RachaelsTranscriber.exe"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "Rachael"
#define MyAppURL "https://github.com/anomalyco/rachaels-transcriber"

[Setup]
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma2
SolidCompression=yes
OutputDir=dist
OutputBaseFilename=RachaelsTranscriber-Setup
SetupIconFile=assets\icon.ico
PrivilegesRequired=admin

[Files]
Source: "dist\RachaelsTranscriber\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Dirs]
Name: "{app}\models"; Permissions: users-modify

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"
Name: "{commondesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Additional shortcuts:"

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifsilent

[Code]
function InitializeUninstall(): Boolean;
begin
  if MsgBox('This will remove Rachael''s Transcriber and its models directory.', mbConfirmation, MB_YESNO) = idYes then
    Result := True
  else
    Result := False;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    DelTree(ExpandConstant('{app}\models'), True, True, True);
end;
