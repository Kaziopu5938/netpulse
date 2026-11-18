#define AppName "NetPulse"
#define AppVersion "1.0.0"

[Setup]
AppId={{7E2C4B1A-9F3D-4C8E-8B5A-2D6F1C9A4E70}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher=NetPulse
DefaultDirName={autopf}\NetPulse
DefaultGroupName=NetPulse
DisableProgramGroupPage=yes
OutputDir=.
OutputBaseFilename=NetPulse-{#AppVersion}-setup
Compression=lzma2
SolidCompression=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
UninstallDisplayIcon={app}\netpulse.exe
WizardStyle=modern

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "startup"; Description: "Start NetPulse with Windows"; GroupDescription: "Additional tasks:"; Flags: checkedonce
Name: "desktopicon"; Description: "Create a desktop shortcut to Settings"; GroupDescription: "Additional icons:"

[Files]
Source: "..\target\release\netpulse.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\NetPulse Settings"; Filename: "{app}\netpulse.exe"; Parameters: "--settings"
Name: "{group}\Uninstall NetPulse"; Filename: "{uninstallexe}"
Name: "{autodesktop}\NetPulse Settings"; Filename: "{app}\netpulse.exe"; Parameters: "--settings"; Tasks: desktopicon

[Run]
Filename: "{app}\netpulse.exe"; Parameters: "--enable-startup"; Flags: runhidden; Tasks: startup
Filename: "{app}\netpulse.exe"; Description: "Launch NetPulse"; Flags: nowait postinstall skipifsilent

[UninstallRun]
Filename: "{cmd}"; Parameters: "/C taskkill /IM netpulse.exe /F"; Flags: runhidden; RunOnceId: "KillNetPulse"
Filename: "{app}\netpulse.exe"; Parameters: "--disable-startup"; Flags: runhidden; RunOnceId: "RemoveStartup"

[UninstallDelete]
Type: filesandordirs; Name: "{userappdata}\NetPulse"
