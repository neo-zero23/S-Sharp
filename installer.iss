; ============================================================================
; S# Language Interpreter - Inno Setup Installer Script
; ============================================================================
; This script creates a Windows installer (.exe) for the S# programming language
; interpreter. It packages the compiled binary, adds it to PATH (optional),
; and creates Start Menu shortcuts.
;
; To compile this script:
;   1. Install Inno Setup Compiler (https://jrsoftware.org/isinfo.php)
;   2. Right-click this file → "Compile" (or open in Inno Setup Compiler → Build)
;   3. The output installer will be in the "Output" folder next to this script
; ============================================================================

#define AppName "S#"
#define AppVersion "0.1.0"
#define AppPublisher "S# Language Project"
#define AppExeName "ssharp.exe"

[Setup]
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL=https://github.com/ssharp-lang/ssharp
AppSupportURL=https://github.com/ssharp-lang/ssharp
AppUpdatesURL=https://github.com/ssharp-lang/ssharp
DefaultDirName={localappdata}\Programs\SSharp
DefaultGroupName={#AppName}
OutputBaseFilename=SSharp-Setup
OutputDir=Output
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
SetupIconFile=logos\ssharp.ico
UninstallDisplayIcon={app}\{#AppExeName}
LicenseFile=LICENSE
InfoBeforeFile=README.md
ArchitecturesInstallIn64BitMode=x64compatible
ArchitecturesAllowed=x64compatible
PrivilegesRequired=lowest
; ============================================================================
; [Files] - Files to include in the installer
; ============================================================================
[Files]
Source: "target\release\ssharp.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "examples\*"; DestDir: "{app}\examples"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "logos\ssharp.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "logos\ssharp_file.ico"; DestDir: "{app}"; Flags: ignoreversion
; ============================================================================
; [Icons] - Start Menu shortcuts
; ============================================================================
[Icons]
Name: "{group}\S# Interpreter"; Filename: "{app}\{#AppExeName}"; IconFilename: "{app}\ssharp.ico"; WorkingDir: "{app}"
Name: "{group}\Uninstall S#"; Filename: "{uninstallexe}"; IconFilename: "{app}\ssharp.ico"

; ============================================================================
; [Tasks] - Optional tasks shown during install
; ============================================================================
[Tasks]
Name: "envPath"; Description: "Add S# to PATH (recommended)"; GroupDescription: "System integration:"; Flags: unchecked

; ============================================================================
; [UninstallDelete] - Extra cleanup during uninstall
; ============================================================================
[UninstallDelete]
Type: filesandordirs; Name: "{app}\examples"
; ============================================================================
; [Registry] - File association for .ssharp files
; ============================================================================
[Registry]
; Asociar extension .ssharp
Root: HKCU; Subkey: "Software\Classes\.ssharp"; \
  ValueType: string; ValueName: ""; \
  ValueData: "SSharpScript"; \
  Flags: uninsdeletekey

; Nombre legible del tipo de archivo
Root: HKCU; Subkey: "Software\Classes\SSharpScript"; \
  ValueType: string; ValueName: ""; \
  ValueData: "S# Script"; \
  Flags: uninsdeletekey

; Icono del archivo (ssharp_file.ico)
Root: HKCU; Subkey: "Software\Classes\SSharpScript\DefaultIcon"; \
  ValueType: string; ValueName: ""; \
  ValueData: "{app}\ssharp_file.ico"; \
  Flags: uninsdeletekey

; Doble click abre con ssharp.exe
Root: HKCU; Subkey: "Software\Classes\SSharpScript\shell\open\command"; \
  ValueType: string; ValueName: ""; \
  ValueData: """{app}\ssharp.exe"" ""%1"""; \
  Flags: uninsdeletekey

; Descripcion en la barra de estado del Explorer
Root: HKCU; Subkey: "Software\Classes\SSharpScript"; \
  ValueType: string; ValueName: "FriendlyTypeName"; \
  ValueData: "S# Script File"; \
  Flags: uninsdeletekey
; ============================================================================
; [Code] - Pascal Script for PATH management
; ============================================================================
; Inno Setup 7 compatible - only uses built-in Pascal Script functions:
; Pos(), Delete(), Length(), RegQueryStringValue(), RegWriteStringValue(),
; RegDeleteValue(), ExpandConstant(), WizardIsTaskSelected()
; NO StringReplace, NO SetLength, NO SendMessageTimeout
; ============================================================================
[Code]

// -----------------------------------------------------------------------
// Helper: Read current user-level PATH from registry
// -----------------------------------------------------------------------
function GetUserPath(): String;
var
  Path: String;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'PATH', Path) then
    Path := '';
  Result := Path;
end;

// -----------------------------------------------------------------------
// Helper: Write user-level PATH to registry
// -----------------------------------------------------------------------
procedure SetUserPath(Path: String);
begin
  if Path = '' then
    RegDeleteValue(HKEY_CURRENT_USER, 'Environment', 'PATH')
  else
    RegWriteStringValue(HKEY_CURRENT_USER, 'Environment', 'PATH', Path);
end;

// -----------------------------------------------------------------------
// Helper: Remove all occurrences of a substring from a string
// (replaces StringReplace which is not available in Inno Setup 7)
// -----------------------------------------------------------------------
function RemoveSubstring(Source: String; ToRemove: String): String;
var
  Position: Integer;
begin
  Result := Source;
  if ToRemove = '' then Exit;
  Position := Pos(ToRemove, Result);
  while Position > 0 do
  begin
    Delete(Result, Position, Length(ToRemove));
    Position := Pos(ToRemove, Result);
  end;
end;

// -----------------------------------------------------------------------
// Helper: Remove leading and trailing semicolons from PATH string
// -----------------------------------------------------------------------
function TrimSemicolons(S: String): String;
begin
  Result := S;
  // Remove leading semicolons
  while (Length(Result) > 0) and (Result[1] = ';') do
    Delete(Result, 1, 1);
  // Remove trailing semicolons
  while (Length(Result) > 0) and (Result[Length(Result)] = ';') do
    Delete(Result, Length(Result), 1);
end;

// -----------------------------------------------------------------------
// Helper: Check if AppPath is already in the PATH string
// Wraps both strings with semicolons to avoid partial matches
// e.g. "C:\SSharp" should not match "C:\SSharp2"
// -----------------------------------------------------------------------
function PathContains(CurrentPath: String; AppPath: String): Boolean;
begin
  Result := Pos(';' + AppPath + ';', ';' + CurrentPath + ';') > 0;
end;

// -----------------------------------------------------------------------
// CurStepChanged: runs automatically at each install step
// We hook into ssPostInstall to add {app} to PATH
// -----------------------------------------------------------------------
procedure CurStepChanged(CurStep: TSetupStep);
var
  AppPath: String;
  CurrentPath: String;
  NewPath: String;
begin
  if CurStep = ssPostInstall then
  begin
    // Only modify PATH if user checked the "Add to PATH" checkbox
    if WizardIsTaskSelected('envPath') then
    begin
      AppPath := ExpandConstant('{app}');
      CurrentPath := GetUserPath();

      // Only add if not already present (prevents duplicates on reinstall)
      if not PathContains(CurrentPath, AppPath) then
      begin
        if CurrentPath = '' then
          NewPath := AppPath
        else
          NewPath := CurrentPath + ';' + AppPath;

        SetUserPath(NewPath);
      end;
      // If already present, do nothing - idempotent behavior
    end;
  end;
end;

// -----------------------------------------------------------------------
// CurUninstallStepChanged: runs automatically at each uninstall step
// We hook into usPostUninstall to remove {app} from PATH
// -----------------------------------------------------------------------
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  AppPath: String;
  CurrentPath: String;
  NewPath: String;
  SearchStr: String;
  Position: Integer;
begin
  if CurUninstallStep = usPostUninstall then
  begin
    AppPath := ExpandConstant('{app}');
    CurrentPath := GetUserPath();

    if PathContains(CurrentPath, AppPath) then
    begin
      // Wrap with semicolons to simplify all position cases
      NewPath := ';' + CurrentPath + ';';

      // Remove ";AppPath;" regardless of position
      SearchStr := ';' + AppPath + ';';
      Position := Pos(SearchStr, NewPath);
      if Position > 0 then
      begin
        Delete(NewPath, Position, Length(SearchStr));
        // Re-add the semicolon that was shared with the next entry
        // (Delete removed both surrounding semicolons, put one back)
        if (Position <= Length(NewPath)) then
          Insert(';', NewPath, Position);
      end;

      // Strip the wrapping semicolons we added at the start
      NewPath := TrimSemicolons(NewPath);

      // Clean up any double semicolons
      NewPath := RemoveSubstring(NewPath, ';;');
      NewPath := TrimSemicolons(NewPath);

      SetUserPath(NewPath);
    end;
  end;
end;