const REPO = "ilovemygh/Media-Gallery";
const RELEASES_API = "https://api.github.com/repos/" + REPO + "/releases/latest";
const FALLBACK_RELEASES = "https://github.com/" + REPO + "/releases/latest";
const ASSET_PATTERNS = {
  "windows-x64-setup.exe": /windows-x64-setup\.exe$/i,
  "windows-x64-portable.zip": /windows-x64-portable\.zip$/i,
  "linux-x86_64.AppImage": /linux-x86_64\.AppImage$/i,
  "linux-x64-portable.tar.gz": /linux-x64-portable\.tar\.gz$/i,
  "android-arm64-v8a.apk": /android-arm64-v8a\.apk$/i,
  "android-universal.apk": /android-universal\.apk$/i,
  "android-armeabi-v7a.apk": /android-armeabi-v7a\.apk$/i,
};

function detectPlatform() {
  const ua = navigator.userAgent.toLowerCase();
  if (/android/.test(ua)) return "android";
  if (/windows/.test(ua)) return "windows";
  if (/linux|x11/.test(ua)) return "linux";
  return null;
}

function setRecommendedPlatform(platform) {
  if (!platform) return;
  const card = document.querySelector('[data-platform="' + platform + '"]');
  if (!card) return;
  document.querySelectorAll(".download-card").forEach((item) => {
    item.classList.remove("recommended");
    const label = item.querySelector(".recommend-label");
    if (label) label.hidden = true;
  });
  card.classList.add("recommended");
  const label = card.querySelector(".recommend-label");
  if (label) label.hidden = false;
  const primary = card.querySelector(".download-button");
  const heroDownload = document.querySelector("#hero-download");
  if (primary && heroDownload) heroDownload.href = primary.href;
  const names = { windows: "Windows", linux: "Linux", android: "Android" };
  document.querySelector("#release-kicker").textContent = "Ready for " + names[platform];
}

function formatDate(value) {
  if (!value) return "";
  return new Intl.DateTimeFormat(undefined, { year: "numeric", month: "short", day: "numeric" }).format(new Date(value));
}

function renderRelease(release) {
  const assets = release.assets || [];
  const releaseStatus = document.querySelector("#release-status");
  document.querySelector("#version-label").textContent = release.tag_name || "Latest release";
  document.querySelector("#published-label").textContent = formatDate(release.published_at);
  document.querySelector("#release-kicker").textContent = "Latest release " + (release.tag_name || "");
  const releaseDot = releaseStatus.querySelector(".status-dot");
  releaseStatus.replaceChildren(releaseDot, document.createTextNode("Latest release " + (release.tag_name || "")));

  document.querySelectorAll("[data-asset]").forEach((link) => {
    const pattern = ASSET_PATTERNS[link.dataset.asset];
    const asset = assets.find((item) => pattern && pattern.test(item.name));
    if (asset) {
      link.href = asset.browser_download_url;
      link.setAttribute("download", asset.name);
      link.dataset.available = "true";
    } else {
      link.href = release.html_url || FALLBACK_RELEASES;
      link.removeAttribute("download");
      link.dataset.available = "false";
      link.title = "This file is not attached to the latest release. View all release files.";
    }
  });

  const checksum = assets.find((asset) => /SHA256SUMS\.txt$/i.test(asset.name));
  if (checksum) document.querySelector("#checksums-link").href = checksum.browser_download_url;
  const selectedPlatform = detectPlatform();
  const selectedCard = selectedPlatform && document.querySelector('[data-platform="' + selectedPlatform + '"]');
  if (selectedCard) document.querySelector("#hero-download").href = selectedCard.querySelector(".download-button").href;
  document.querySelector("#release-note").textContent = "Version " + (release.tag_name || "latest") + " · Windows · Linux · Android";
}

async function loadLatestRelease() {
  try {
    const response = await fetch(RELEASES_API, {
      headers: { Accept: "application/vnd.github+json" },
      cache: "no-store",
    });
    if (!response.ok) throw new Error("Release lookup returned " + response.status);
    renderRelease(await response.json());
  } catch (error) {
    const status = document.querySelector("#release-status");
    const statusDot = status.querySelector(".status-dot");
    status.replaceChildren(statusDot, document.createTextNode("Release details are unavailable right now"));
    document.querySelector("#release-note").textContent = "Downloads are hosted on GitHub";
    document.querySelector("#version-label").textContent = "Release details unavailable";
    document.querySelector("#published-label").textContent = "";
    document.querySelectorAll("[data-asset]").forEach((link) => {
      link.href = FALLBACK_RELEASES;
      link.removeAttribute("download");
    });
    document.querySelector("#checksums-link").href = FALLBACK_RELEASES;
  }
}

document.querySelector("#year").textContent = new Date().getFullYear();
setRecommendedPlatform(detectPlatform());
loadLatestRelease();
