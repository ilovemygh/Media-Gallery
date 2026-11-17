param(
    [Parameter(Mandatory = $true)]
    [string]$ApkDir,

    [Parameter(Mandatory = $true)]
    [string]$Version,

    [Parameter(Mandatory = $false)]
    [string]$AppVersion = "",

    [Parameter(Mandatory = $true)]
    [string]$VersionCode,

    [Parameter(Mandatory = $false)]
    [switch]$ExpectReleaseSigned
)

$ErrorActionPreference = "Stop"
if (-not $AppVersion) {
    $AppVersion = $Version
}

$SdkRoot = if ($env:ANDROID_SDK_ROOT) { $env:ANDROID_SDK_ROOT } else { $env:ANDROID_HOME }
if (-not $SdkRoot -or -not (Test-Path $SdkRoot)) {
    throw "ANDROID_SDK_ROOT / ANDROID_HOME is not configured."
}

$BuildToolsDir = Get-ChildItem -Path (Join-Path $SdkRoot "build-tools") -Directory |
    Sort-Object Name -Descending |
    Select-Object -First 1 -ExpandProperty FullName
if (-not $BuildToolsDir) {
    throw "No Android SDK build-tools version found under $SdkRoot\build-tools"
}

$ApkSigner = Join-Path $BuildToolsDir "apksigner.bat"
if (-not (Test-Path $ApkSigner)) {
    $ApkSigner = Join-Path $BuildToolsDir "apksigner"
}
$Aapt2 = Join-Path $BuildToolsDir "aapt2.exe"
if (-not (Test-Path $Aapt2)) {
    $Aapt2 = Join-Path $BuildToolsDir "aapt2"
}
if (-not (Test-Path $ApkSigner)) { throw "apksigner not found in $BuildToolsDir" }
if (-not (Test-Path $Aapt2)) { throw "aapt2 not found in $BuildToolsDir" }

Add-Type -AssemblyName System.IO.Compression.FileSystem

$ExpectedApks = @(
    @{
        File = "MediaGallery-$Version-android-arm64-v8a.apk"
        RequiredEntries = @("lib/arm64-v8a/libcore_api.so", "lib/arm64-v8a/libflutter.so")
    },
    @{
        File = "MediaGallery-$Version-android-armeabi-v7a.apk"
        RequiredEntries = @("lib/armeabi-v7a/libcore_api.so", "lib/armeabi-v7a/libflutter.so")
    },
    @{
        File = "MediaGallery-$Version-android-universal.apk"
        RequiredEntries = @(
            "lib/arm64-v8a/libcore_api.so",
            "lib/armeabi-v7a/libcore_api.so",
            "lib/x86_64/libcore_api.so",
            "lib/arm64-v8a/libflutter.so",
            "lib/armeabi-v7a/libflutter.so",
            "lib/x86_64/libflutter.so"
        )
    }
)

foreach ($spec in $ExpectedApks) {
    $apkPath = Join-Path $ApkDir $spec.File
    if (-not (Test-Path $apkPath)) {
        throw "Expected APK missing: $apkPath"
    }
    $sizeMb = [Math]::Round((Get-Item $apkPath).Length / 1MB, 2)
    Write-Output "Verifying $($spec.File) ($sizeMb MB)..."

    # 1. Verify native Rust & Flutter .so entries
    $zip = [System.IO.Compression.ZipFile]::OpenRead($apkPath)
    try {
        $entryNames = @($zip.Entries | ForEach-Object { $_.FullName })
        foreach ($req in $spec.RequiredEntries) {
            if ($entryNames -notcontains $req) {
                throw "$($spec.File) is missing required native library: $req"
            }
        }
    } finally {
        $zip.Dispose()
    }

    # 2. Verify APK signature via apksigner
    $sigOut = (& $ApkSigner verify --verbose --print-certs $apkPath 2>&1) -join "`n"
    if ($LASTEXITCODE -ne 0) {
        throw "apksigner verify failed for $($spec.File):`n$sigOut"
    }
    if ($ExpectReleaseSigned -and ($sigOut -match 'CN=Android Debug')) {
        throw "$($spec.File) was signed with the Android Debug keystore, not the release keystore!"
    }

    # 3. Verify package ID, versionName, and versionCode via aapt2
    $badging = (& $Aapt2 dump badging $apkPath 2>&1) -join "`n"
    if ($LASTEXITCODE -ne 0) {
        throw "aapt2 dump badging failed for $($spec.File):`n$badging"
    }
    if ($badging -notmatch "package:\s+name='io\.mediagallery\.app'") {
        throw "$($spec.File) has unexpected package name in badging:`n$badging"
    }
    if ($badging -notmatch "versionCode='$([regex]::Escape($VersionCode))'") {
        throw "$($spec.File) versionCode did not match '$VersionCode':`n$badging"
    }
    if ($badging -notmatch "versionName='$([regex]::Escape($AppVersion))'") {
        throw "$($spec.File) versionName did not match '$AppVersion':`n$badging"
    }

    Write-Output "  OK: native libs, signature, and metadata verified ($AppVersion+$VersionCode)"
}
