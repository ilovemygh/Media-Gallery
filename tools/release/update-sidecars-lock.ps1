$ErrorActionPreference = "Stop"

$LockPath = Join-Path $PSScriptRoot "sidecars.lock"

# 1. Resolve latest yt-dlp release tag & SHA-256
$ytResp = Invoke-WebRequest -UseBasicParsing "https://github.com/yt-dlp/yt-dlp/releases/latest" -MaximumRedirection 5
if ($ytResp.BaseResponse.ResponseUri.AbsoluteUri -notmatch '/tag/([^/]+)$') {
    throw "Could not resolve latest yt-dlp tag from $($ytResp.BaseResponse.ResponseUri)"
}
$ytTag = $Matches[1]
$ytSumsText = (curl.exe -fsSL "https://github.com/yt-dlp/yt-dlp/releases/download/$ytTag/SHA2-256SUMS") -join "`n"
$ytLine = ($ytSumsText -split "`n") | Where-Object { $_ -match '^[0-9a-fA-F]{64}\s+yt-dlp\.exe\s*$' } | Select-Object -First 1
if (-not $ytLine -or $ytLine -notmatch '^([0-9a-fA-F]{64})\s+yt-dlp\.exe') {
    throw "Could not find yt-dlp.exe checksum in SHA2-256SUMS for $ytTag"
}
$ytSha = $Matches[1].ToLower()

# 2. Resolve newest dated BtbN/FFmpeg-Builds autobuild tag & win64-lgpl-8.1 asset SHA-256
$ffReleasesHtml = (curl.exe -fsSL "https://github.com/BtbN/FFmpeg-Builds/releases") -join "`n"
$ffTagMatch = [regex]::Match($ffReleasesHtml, 'autobuild-[0-9-]+')
if (-not $ffTagMatch.Success) {
    throw "Could not locate any BtbN/FFmpeg-Builds autobuild-* tag"
}
$ffTag = $ffTagMatch.Value
$ffSumsText = (curl.exe -fsSL "https://github.com/BtbN/FFmpeg-Builds/releases/download/$ffTag/checksums.sha256") -join "`n"
$ffLine = ($ffSumsText -split "`n") | Where-Object { $_ -match 'win64-lgpl-8\.1\.zip' } | Select-Object -First 1
if (-not $ffLine) {
    $ffLine = ($ffSumsText -split "`n") | Where-Object { $_ -match 'win64-lgpl\.zip' } | Select-Object -First 1
}
if (-not $ffLine -or $ffLine -notmatch '^([0-9a-fA-F]{64})\s+(\S+)') {
    throw "Could not locate win64-lgpl zip checksum in $ffTag"
}
$ffSha = $Matches[1].ToLower()
$ffAsset = $Matches[2].Trim()
$ffVersion = if ($ffAsset -match '^ffmpeg-(.+)-win64-lgpl') { $Matches[1] } else { $ffTag }

$lockObj = [ordered]@{
    schema_version = 1
    updated_at     = (Get-Date).ToUniversalTime().ToString("yyyy-MM-dd")
    sidecars       = [ordered]@{
        ffmpeg_win64_lgpl = [ordered]@{
            repo          = "BtbN/FFmpeg-Builds"
            tag           = $ffTag
            version       = $ffVersion
            asset         = $ffAsset
            url           = "https://github.com/BtbN/FFmpeg-Builds/releases/download/$ffTag/$ffAsset"
            sha256        = $ffSha
            license       = "LGPL-2.1-or-later"
            extract_files = @("bin/ffmpeg.exe", "bin/ffprobe.exe", "LICENSE.txt")
        }
        yt_dlp_win64      = [ordered]@{
            repo    = "yt-dlp/yt-dlp"
            tag     = $ytTag
            version = $ytTag
            asset   = "yt-dlp.exe"
            url     = "https://github.com/yt-dlp/yt-dlp/releases/download/$ytTag/yt-dlp.exe"
            sha256  = $ytSha
            license = "Unlicense"
        }
    }
}

$json = ($lockObj | ConvertTo-Json -Depth 6) -replace "`r`n", "`n"
[System.IO.File]::WriteAllText($LockPath, "$json`n", [System.Text.UTF8Encoding]::new($false))

Write-Output "Updated $LockPath"
Write-Output "  ffmpeg: $ffTag / $ffAsset ($ffSha)"
Write-Output "  yt-dlp: $ytTag ($ytSha)"
