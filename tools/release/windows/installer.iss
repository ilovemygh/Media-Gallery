; Media Gallery Windows Desktop Installer (Inno Setup 6)
; Invoked via: iscc /DAppVersion=0.2.0 /DStageDir=... /DOutputDir=... /DOutputBaseFilename=MediaGallery-0.2.0-windows-x64-setup installer.iss

#ifndef AppVersion
  #define AppVersion "0.2.4"
#endif
#ifndef AppNumericVersion
  #define AppNumericVersion "0.2.4.0"
#endif
#ifndef StageDir
  #define StageDir "..\..\..\dist\windows\stage"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\..\dist\windows"
#endif
#ifndef OutputBaseFilename
  #define OutputBaseFilename "MediaGallery-" + AppVersion + "-windows-x64-setup"
#endif

[Setup]
AppId={{B4E8A71D-4D95-4C46-9D39-8E3B98F4C712}
AppName=Media Gallery
AppVersion={#AppVersion}
AppVerName=Media Gallery {#AppVersion}
VersionInfoVersion={#AppNumericVersion}
AppPublisher=Media Gallery (io.mediagallery)
AppPublisherURL=https://github.com/ilovemygh/Media-Gallery
AppSupportURL=https://github.com/ilovemygh/Media-Gallery/issues
AppUpdatesURL=https://github.com/ilovemygh/Media-Gallery/releases
DefaultDirName={autopf}\Media Gallery
DefaultGroupName=Media Gallery
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir={#OutputDir}
OutputBaseFilename={#OutputBaseFilename}
SetupIconFile=..\..\..\app\windows\runner\resources\app_icon.ico
UninstallDisplayIcon={app}\media_gallery.exe
LicenseFile={#StageDir}\THIRD_PARTY_LICENSES.txt
Compression=lzma2/normal
SolidCompression=yes
WizardStyle=modern
ChangesEnvironment=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#StageDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Media Gallery"; Filename: "{app}\media_gallery.exe"
Name: "{group}\Third-Party Licenses"; Filename: "{app}\THIRD_PARTY_LICENSES.txt"
Name: "{group}\{cm:UninstallProgram,Media Gallery}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\Media Gallery"; Filename: "{app}\media_gallery.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\media_gallery.exe"; Description: "{cm:LaunchProgram,Media Gallery}"; Flags: nowait postinstall skipifsilent
