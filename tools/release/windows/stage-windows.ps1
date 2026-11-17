param(
    [Parameter(Mandatory = $false)]
    [string]$Version = "",

    [Parameter(Mandatory = $false)]
    [string]$OutputDir = "",

    [Parameter(Mandatory = $false)]
    [string]$CacheDir = "",

    [Parameter(Mandatory = $false)]
    [switch]$SkipFlutterBuild,

    [Parameter(Mandatory = $false)]
    [switch]$VerifyInstaller,

    [Parameter(Mandatory = $false)]
    [switch]$AllowMissingVcRedist
)

$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..\..")).Path
$AppDir = Join-Path $RepoRoot "app"
$LockPath = Join-Path $RepoRoot "tools\release\sidecars.lock"
$IssPath = Join-Path $PSScriptRoot "installer.iss"

if (-not $Version) {
    $PubspecRaw = Get-Content -Raw -Path (Join-Path $AppDir "pubspec.yaml")
    if ($PubspecRaw -match '(?m)^version:\s*([^+\r\n]+)') {
        $Version = $Matches[1].Trim()
    } else {
        throw "Could not determine version from app/pubspec.yaml; pass -Version explicitly."
    }
}
$Version = $Version.TrimStart("v")

if (-not $OutputDir) {
    $OutputDir = Join-Path $RepoRoot "dist\windows"
}
$OutputDir = [System.IO.Path]::GetFullPath($OutputDir)
if (-not $CacheDir) {
    $CacheDir = Join-Path $RepoRoot "build-cache\sidecars"
}
$CacheDir = [System.IO.Path]::GetFullPath($CacheDir)

$NumericVersion = if ($Version -match '^(\d+\.\d+\.\d+)') { "$($Matches[1]).0" } else { "0.0.0.0" }

$StageParent = Join-Path $OutputDir "stage-root"
$PortableFolderName = "MediaGallery-$Version-windows-x64"
$StageDir = Join-Path $StageParent $PortableFolderName

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null
New-Item -ItemType Directory -Force -Path $CacheDir | Out-Null

if (Test-Path $StageParent) {
    Remove-Item -Recurse -Force $StageParent
}
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null

if (-not $SkipFlutterBuild) {
    Push-Location $AppDir
    try {
        flutter pub get
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
        flutter build windows --release
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    } finally {
        Pop-Location
    }
}

$ReleaseBuildDir = Join-Path $AppDir "build\windows\x64\runner\Release"
if (-not (Test-Path (Join-Path $ReleaseBuildDir "media_gallery.exe"))) {
    throw "Missing Flutter Windows release output at $ReleaseBuildDir"
}
if (-not (Test-Path (Join-Path $ReleaseBuildDir "core_api.dll"))) {
    throw "Missing core_api.dll in $ReleaseBuildDir"
}

Write-Output "Copying Flutter Windows release bundle from $ReleaseBuildDir..."
Copy-Item -Path (Join-Path $ReleaseBuildDir "*") -Destination $StageDir -Recurse -Force

# 1. Copy Microsoft Visual C++ Redistributable x64 CRT DLLs app-locally
$VcRedistDlls = @()
$VsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
if (Test-Path $VsWhere) {
    $VsPath = (& $VsWhere -latest -products * -property installationPath | Select-Object -First 1)
    if ($VsPath) {
        $CrtDirs = Get-ChildItem -Path (Join-Path $VsPath "VC\Redist\MSVC") -Directory -ErrorAction SilentlyContinue |
            Sort-Object Name -Descending |
            ForEach-Object { Join-Path $_.FullName "x64\Microsoft.VC143.CRT" } |
            Where-Object { Test-Path $_ }
        $CrtDir = $CrtDirs | Select-Object -First 1
        if ($CrtDir) {
            $VcRedistDlls = Get-ChildItem -Path $CrtDir -Filter "*.dll" -File
            foreach ($dll in $VcRedistDlls) {
                Copy-Item -Path $dll.FullName -Destination (Join-Path $StageDir $dll.Name) -Force
            }
            Write-Output "Bundled $($VcRedistDlls.Count) VC++ CRT DLLs from $CrtDir"
        }
    }
}
if ($VcRedistDlls.Count -eq 0 -and -not $AllowMissingVcRedist) {
    throw "Could not locate Microsoft.VC143.CRT x64 redistributable DLLs. Install VS VC++ Redistributable component or pass -AllowMissingVcRedist."
}

# 2. Download & SHA-256 verify pinned sidecars from tools/release/sidecars.lock
$Lock = Get-Content -Raw -Path $LockPath | ConvertFrom-Json

function Get-VerifiedDownload {
    param(
        [string]$Url,
        [string]$ExpectedSha256,
        [string]$DestPath
    )
    $expected = $ExpectedSha256.ToLowerInvariant()
    if (Test-Path $DestPath) {
        $existing = (Get-FileHash -Path $DestPath -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($existing -eq $expected) {
            Write-Output "Using cached verified file: $DestPath"
            return
        }
        Remove-Item -Force $DestPath
    }
    Write-Output "Downloading $Url..."
    curl.exe -fsSL --retry 3 --retry-delay 2 -o $DestPath $Url
    if ($LASTEXITCODE -ne 0) {
        throw "Failed to download $Url"
    }
    $actual = (Get-FileHash -Path $DestPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
        Remove-Item -Force $DestPath -ErrorAction SilentlyContinue
        throw "SHA-256 mismatch for $Url! Expected $expected, got $actual"
    }
    Write-Output "SHA-256 verified ($expected): $(Split-Path -Leaf $DestPath)"
}

$FfmpegSpec = $Lock.sidecars.ffmpeg_win64_lgpl
$FfmpegZipPath = Join-Path $CacheDir $FfmpegSpec.asset
Get-VerifiedDownload -Url $FfmpegSpec.url -ExpectedSha256 $FfmpegSpec.sha256 -DestPath $FfmpegZipPath

$FfmpegExtractDir = Join-Path $CacheDir "ffmpeg-extract"
if (Test-Path $FfmpegExtractDir) { Remove-Item -Recurse -Force $FfmpegExtractDir }
Expand-Archive -Path $FfmpegZipPath -DestinationPath $FfmpegExtractDir -Force

$FfmpegBin = Get-ChildItem -Path $FfmpegExtractDir -Recurse -Filter "ffmpeg.exe" -File | Select-Object -First 1
$FfprobeBin = Get-ChildItem -Path $FfmpegExtractDir -Recurse -Filter "ffprobe.exe" -File | Select-Object -First 1
$FfmpegLicense = Get-ChildItem -Path $FfmpegExtractDir -Recurse -Filter "LICENSE.txt" -File | Select-Object -First 1
if (-not $FfmpegBin -or -not $FfprobeBin) {
    throw "Extracted archive $FfmpegZipPath did not contain ffmpeg.exe and ffprobe.exe"
}
Copy-Item -Path $FfmpegBin.FullName -Destination (Join-Path $StageDir "ffmpeg.exe") -Force
Copy-Item -Path $FfprobeBin.FullName -Destination (Join-Path $StageDir "ffprobe.exe") -Force

$YtdlpSpec = $Lock.sidecars.yt_dlp_win64
$YtdlpCached = Join-Path $CacheDir "yt-dlp-$($YtdlpSpec.version).exe"
Get-VerifiedDownload -Url $YtdlpSpec.url -ExpectedSha256 $YtdlpSpec.sha256 -DestPath $YtdlpCached
Copy-Item -Path $YtdlpCached -Destination (Join-Path $StageDir "yt-dlp.exe") -Force

# 3. Verify bundled sidecar binaries and LGPL configuration
$FfmpegVersionOut = (& (Join-Path $StageDir "ffmpeg.exe") -version) -join "`n"
if ($LASTEXITCODE -ne 0) { throw "Bundled ffmpeg.exe -version failed" }
if ($FfmpegVersionOut -match '--enable-gpl' -or $FfmpegVersionOut -match '--enable-nonfree') {
    throw "Bundled ffmpeg.exe appears to be GPL/nonfree rather than LGPL! Output:`n$FfmpegVersionOut"
}
& (Join-Path $StageDir "ffprobe.exe") -version | Out-Null
if ($LASTEXITCODE -ne 0) { throw "Bundled ffprobe.exe -version failed" }
$YtdlpVer = & (Join-Path $StageDir "yt-dlp.exe") --version
if ($LASTEXITCODE -ne 0) { throw "Bundled yt-dlp.exe --version failed" }
Write-Output "Verified sidecars: yt-dlp=$YtdlpVer, ffmpeg LGPL check passed"

# 4. Write THIRD_PARTY_LICENSES.txt
$FfmpegLicenseText = if ($FfmpegLicense) { Get-Content -Raw -Path $FfmpegLicense.FullName } else { "GNU Lesser General Public License v2.1 or later (https://www.gnu.org/licenses/old-licenses/lgpl-2.1.html)" }
$Notices = @"
Media Gallery v$Version - Third-Party Licenses & Notices
=======================================================

1. Media Gallery
   License: MIT OR Apache-2.0
   Source: https://github.com/ilovemygh/Media-Gallery

2. FFmpeg & ffprobe (Sidecar Executables)
   Version: $($FfmpegSpec.version) ($($FfmpegSpec.tag))
   License: $($FfmpegSpec.license) (built without --enable-gpl or --enable-nonfree)
   Source / Build scripts: https://github.com/$($FfmpegSpec.repo) and https://ffmpeg.org
   Note: ffmpeg.exe and ffprobe.exe are invoked out-of-process as separate sidecars.
   Users may replace ffmpeg.exe and ffprobe.exe in the application directory with any
   compatible build.

   --- FFmpeg LICENSE.txt ---
$FfmpegLicenseText

3. libmpv / media_kit (Video & Audio Playback Engine)
   License: MIT (media_kit Dart/C++ wrappers) + LGPL v2.1+ (libmpv dynamic shared library)
   Source: https://github.com/media-kit/media-kit and https://mpv.io
   Note: Dynamically linked (e.g. mpv-2.dll when bundled) so users may upgrade or
   replace the shared library independently under LGPL v2.1+.

4. yt-dlp (Sidecar Executable)
   Version: $($YtdlpSpec.version)
   License: The Unlicense
   Source: https://github.com/$($YtdlpSpec.repo)

5. GeoNames Offline City Dataset (via reverse_geocoder)
   License: Creative Commons Attribution 4.0 International (CC-BY-4.0)
   Source: https://www.geonames.org

6. Microsoft Visual C++ Runtime Libraries
   Redistributable CRT DLLs distributed under the Microsoft Visual Studio License Terms.

7. Flutter SDK, Rust Standard Library, and Cargo / Pub Dependencies
   Distributed under their respective MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause,
   ISC, and CC0-1.0 licenses. See repository Cargo.toml and app/pubspec.yaml.
"@
$Notices = $Notices -replace "`r`n", "`n"
[System.IO.File]::WriteAllText((Join-Path $StageDir "THIRD_PARTY_LICENSES.txt"), $Notices, [System.Text.UTF8Encoding]::new($false))

# 5. Optional Authenticode code signing helper
$CertPfxB64 = $env:WINDOWS_CERT_PFX_BASE64
$CertPassword = $env:WINDOWS_CERT_PASSWORD
$SignTool = $null
$TempPfx = $null

if (-not [string]::IsNullOrWhiteSpace($CertPfxB64) -and -not [string]::IsNullOrWhiteSpace($CertPassword)) {
    $SignTool = Get-ChildItem -Path "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Recurse -Filter "signtool.exe" -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match '\\x64\\signtool\.exe$' } |
        Sort-Object FullName -Descending |
        Select-Object -First 1 -ExpandProperty FullName
    if (-not $SignTool) {
        throw "WINDOWS_CERT_PFX_BASE64 is set, but signtool.exe (x64) was not found."
    }
    $TempPfx = Join-Path ([System.IO.Path]::GetTempPath()) "mg-codesign-$([Guid]::NewGuid().ToString('N')).pfx"
    [System.IO.File]::WriteAllBytes($TempPfx, [Convert]::FromBase64String($CertPfxB64))
} else {
    Write-Output "Authenticode code signing skipped (WINDOWS_CERT_PFX_BASE64 / WINDOWS_CERT_PASSWORD not configured)."
}

function Invoke-CodeSign {
    param([string]$TargetPath)
    if ($TempPfx -and $SignTool) {
        & $SignTool sign /f $TempPfx /p $CertPassword /fd SHA256 /tr "http://timestamp.digicert.com" /td SHA256 $TargetPath
        if ($LASTEXITCODE -ne 0) { throw "signtool sign failed for $TargetPath" }
        & $SignTool verify /pa $TargetPath
        if ($LASTEXITCODE -ne 0) { throw "signtool verify failed for $TargetPath" }
    }
}

try {
    Invoke-CodeSign -TargetPath (Join-Path $StageDir "media_gallery.exe")
    Invoke-CodeSign -TargetPath (Join-Path $StageDir "core_api.dll")

    # 6. Build portable .zip
    $PortableZipPath = Join-Path $OutputDir "MediaGallery-$Version-windows-x64-portable.zip"
    if (Test-Path $PortableZipPath) { Remove-Item -Force $PortableZipPath }
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::CreateFromDirectory($StageParent, $PortableZipPath, [System.IO.Compression.CompressionLevel]::Optimal, $false)
    Write-Output "Created portable archive: $PortableZipPath"

    # 7. Build Inno Setup .exe installer
    $Iscc = Get-Command "iscc" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
    if (-not $Iscc) {
        foreach ($candidate in @(
            "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
            "${env:ProgramFiles}\Inno Setup 6\ISCC.exe"
        )) {
            if ($candidate -and (Test-Path $candidate)) {
                $Iscc = $candidate
                break
            }
        }
    }
    if (-not $Iscc) {
        throw "Inno Setup 6 (iscc.exe) not found. Install Inno Setup 6 before building the Windows installer."
    }

    $SetupBaseName = "MediaGallery-$Version-windows-x64-setup"
    $SetupExePath = Join-Path $OutputDir "$SetupBaseName.exe"
    if (Test-Path $SetupExePath) { Remove-Item -Force $SetupExePath }

    & $Iscc `
        "/DAppVersion=$Version" `
        "/DAppNumericVersion=$NumericVersion" `
        "/DStageDir=$StageDir" `
        "/DOutputDir=$OutputDir" `
        "/DOutputBaseFilename=$SetupBaseName" `
        $IssPath
    if ($LASTEXITCODE -ne 0) { throw "Inno Setup compilation failed" }
    if (-not (Test-Path $SetupExePath)) { throw "Expected installer missing at $SetupExePath" }

    Invoke-CodeSign -TargetPath $SetupExePath
    Write-Output "Created Windows installer: $SetupExePath"

    # 8. Optional clean silent install -> launch smoke -> uninstall + portable launch verification
    if ($VerifyInstaller) {
        $InstallTestDir = Join-Path ([System.IO.Path]::GetTempPath()) "mg-install-verify-$([Guid]::NewGuid().ToString('N'))"
        $PortableTestDir = Join-Path ([System.IO.Path]::GetTempPath()) "mg-portable-verify-$([Guid]::NewGuid().ToString('N'))"
        try {
            Write-Output "Running silent installer verification into $InstallTestDir..."
            $instProc = Start-Process -FilePath $SetupExePath -ArgumentList "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/DIR=`"$InstallTestDir`"" -Wait -PassThru
            if ($instProc.ExitCode -ne 0) {
                throw "Silent installer exited with code $($instProc.ExitCode)"
            }
            foreach ($reqFile in @("media_gallery.exe", "core_api.dll", "flutter_windows.dll", "ffmpeg.exe", "ffprobe.exe", "yt-dlp.exe", "THIRD_PARTY_LICENSES.txt")) {
                $p = Join-Path $InstallTestDir $reqFile
                if (-not (Test-Path $p)) {
                    throw "Installed bundle missing required file: $p"
                }
            }
            Write-Output "Verifying installed media_gallery.exe launch..."
            $appProc = Start-Process -FilePath (Join-Path $InstallTestDir "media_gallery.exe") -PassThru
            Start-Sleep -Seconds 4
            if ($appProc.HasExited) {
                throw "Installed media_gallery.exe exited prematurely with code $($appProc.ExitCode)"
            }
            $null = $appProc.CloseMainWindow()
            Start-Sleep -Seconds 1
            if (-not $appProc.HasExited) {
                Stop-Process -Id $appProc.Id -Force
            }
            Write-Output "Verified installed media_gallery.exe stays running cleanly."

            $uninst = Join-Path $InstallTestDir "unins000.exe"
            if (-not (Test-Path $uninst)) {
                throw "Uninstaller missing at $uninst"
            }
            $uninstProc = Start-Process -FilePath $uninst -ArgumentList "/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART" -Wait -PassThru
            if ($uninstProc.ExitCode -ne 0) {
                throw "Silent uninstaller exited with code $($uninstProc.ExitCode)"
            }
            Start-Sleep -Seconds 2
            if (Test-Path (Join-Path $InstallTestDir "media_gallery.exe")) {
                throw "media_gallery.exe still exists after silent uninstall!"
            }
            Write-Output "Verified clean silent uninstall."

            Write-Output "Running portable ZIP extraction & launch verification into $PortableTestDir..."
            Expand-Archive -Path $PortableZipPath -DestinationPath $PortableTestDir -Force
            $PortableAppDir = Join-Path $PortableTestDir $PortableFolderName
            foreach ($reqFile in @("media_gallery.exe", "core_api.dll", "flutter_windows.dll", "ffmpeg.exe", "ffprobe.exe", "yt-dlp.exe", "THIRD_PARTY_LICENSES.txt")) {
                $p = Join-Path $PortableAppDir $reqFile
                if (-not (Test-Path $p)) {
                    throw "Portable bundle missing required file: $p"
                }
            }
            $portProc = Start-Process -FilePath (Join-Path $PortableAppDir "media_gallery.exe") -WorkingDirectory $PortableAppDir -PassThru
            Start-Sleep -Seconds 4
            if ($portProc.HasExited) {
                throw "Portable media_gallery.exe exited prematurely with code $($portProc.ExitCode)"
            }
            $null = $portProc.CloseMainWindow()
            Start-Sleep -Seconds 1
            if (-not $portProc.HasExited) {
                Stop-Process -Id $portProc.Id -Force
            }
            Write-Output "Verified portable media_gallery.exe launch."
        } finally {
            if (Test-Path $InstallTestDir) {
                Remove-Item -Recurse -Force $InstallTestDir -ErrorAction SilentlyContinue
            }
            if (Test-Path $PortableTestDir) {
                Remove-Item -Recurse -Force $PortableTestDir -ErrorAction SilentlyContinue
            }
        }
    }
} finally {
    if ($TempPfx -and (Test-Path $TempPfx)) {
        Remove-Item -Force $TempPfx -ErrorAction SilentlyContinue
    }
}
