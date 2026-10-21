#define MyAppName "StimTake"
#define MyAppVersion "1.0.0"
#define MyAppPublisher "Talented Creative Design"
#define MyAppExeName "StimTake.exe"

[Setup]
AppId={{B12E738D-1B2D-48E8-9B1A-7A0D78B36A41}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={localappdata}\StimTake
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=.
OutputBaseFilename=StimTake-Setup-{#MyAppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
SetupIconFile=StimTake-Rust-UI-Concept-Build\assets\StimTake.ico
PrivilegesRequired=lowest
UninstallDisplayIcon={app}\{#MyAppExeName}
CloseApplications=yes
RestartApplications=no
ArchitecturesInstallIn64BitMode=x64

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; GroupDescription: "Additional shortcuts:"; Flags: unchecked

[Files]
Source: "StimTake.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "StimTake-Rust-UI-Concept-Build\assets\StimTake.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "StimTake-Rust-UI-Concept-Build\Core_Packs\*"; DestDir: "{app}\Core_Packs"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "StimTake-Rust-UI-Concept-Build\obs_assets\*"; DestDir: "{app}\obs_assets"; Flags: ignoreversion recursesubdirs createallsubdirs skipifsourcedoesntexist

[Icons]
Name: "{autoprograms}\StimTake"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\StimTake.ico"
Name: "{autodesktop}\StimTake"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\StimTake.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch StimTake"; Flags: nowait postinstall skipifsilent
