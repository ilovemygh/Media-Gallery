# PRD: Media Gallery (working title)

**Alternative names:** Media Finder, Media Manager
**Version:** 0.1 (Draft)
**Platforms:** Desktop (Windows first; macOS/Linux via same codebase) + Android
**Document purpose:** Define what the product is, what it must do, and how it should look and behave, so an engineering agent can build it without guessing.

---

## 1. Summary

Media Gallery is a cross-platform app that **scans every media file on a phone and a PC, shows them visually** (thumbnails, covers, folder previews), and makes them easy to **find, rename, move, and play**.

It also acts as a **"serverless Google Drive"**: the user's own phone and PC are the storage. The cloud only holds a tiny index and thumbnails, never the files themselves. Files move directly between devices over Wi-Fi, Bluetooth, or a wired connection.

It includes a built-in **YouTube downloader** (yt-dlp) and an optional **Google/YouTube account link**.

## 2. Problem

- File managers show filenames and generic icons, so media is hard to browse.
- Moving files between phone and PC is clumsy (cables, MTP, Bluetooth dialogs, uploading to a cloud first).
- Music and videos are scattered across two devices.
- Files have meaningless names (`IMG_2031.jpg`, `VID_0042.mp4`) and renaming in bulk is painful.
- Downloading from YouTube needs separate tools, and results land in random folders.

## 3. Goals and Non-goals

### Goals
1. One visual library of all media from every connected device.
2. Fast browsing: thumbnails first, filenames second.
3. Easy alias/rename, filter, and search (including by date and time).
4. Direct device-to-device transfer and sync with no server and no full upload to the cloud.
5. Built-in music player that merges phone and PC music.
6. Built-in YouTube download (video MP4 / audio MP3) that saves straight into the library.
7. Simple, familiar UI (sidebar, top bar, details panel, drag and drop).

### Non-goals (v1)
- iOS app.
- Storing file contents in any cloud.
- Video or audio editing (only cover/thumbnail editing).
- Streaming from YouTube (download only).
- Sync across different networks (remote access over the internet is v2).
- Social or sharing-with-other-people features.

## 2b. Target users

- People with an Android phone and a PC who move media between them often.
- Students and creators with many videos, screenshots, and recordings.
- Music collectors who want phone and PC music in one place.

---

## 4. Platforms and recommended stack

The user wants one product on PC and Android. **Recommended** (agent may justify a change in writing):

| Area | Recommendation |
|---|---|
| App framework | Flutter (single codebase for Android + Windows/macOS/Linux) |
| Local database | SQLite (e.g. `drift`) with FTS5 for search |
| Android media access | MediaStore API (+ scoped storage); `MANAGE_EXTERNAL_STORAGE` only if truly needed |
| Desktop file scan | Recursive scan + OS file watcher |
| Thumbnails/frames | ffmpeg on desktop; `MediaMetadataRetriever` on Android |
| Audio tags and covers | Read/write ID3 / MP4 / FLAC tags (e.g. TagLib bindings) |
| YouTube download | yt-dlp (+ ffmpeg). Bundled binary on desktop; `youtubedl-android` library on Android |
| Device discovery | mDNS / DNS-SD on LAN |
| Transfer | HTTPS + WebSocket over LAN, chunked, resumable, hash-verified |
| Wired | USB via ADB port-forward or USB tethering (exposes a LAN-like link), MTP as fallback |
| Bluetooth | Bluetooth classic / BLE file channel, used for pairing and small/medium files |
| Cloud (metadata only) | Google Sign-In; Google Drive `appDataFolder` for index + thumbnails; YouTube Data API v3 |

---

## 5. Key concepts (glossary)

- **Device Node:** a phone or PC running the app. Each node owns and serves its own files.
- **Media Index:** a database of every media file (path, type, size, hash, dates, metadata, alias, thumbnail).
- **Remote item:** a file that lives on another device. It appears locally with its thumbnail and metadata but is not downloaded yet.
- **Sync state:** `Local`, `Remote` (cloud badge), `Downloading`, `Synced`, `Error`.
- **Alias:** a friendly display name stored in the database. It does **not** change the real file name unless the user chooses "Apply as real rename".
- **Cover:** the image shown for a video or audio file (thumbnail for video, album art for audio).

---

## 6. Functional requirements

Priority: **P0** = MVP, **P1** = important, **P2** = later.

### A. Scanning and library
- **A1 (P0)** Scan all storage for media and build the Media Index. Supported: mp4, mkv, webm, mov, avi, mp3, m4a, flac, wav, ogg, png, jpg/jpeg, gif, webp, heic.
- **A2 (P0)** Incremental rescan using file watchers / MediaStore change notifications.
- **A3 (P0)** Scan runs in the background, shows progress, and never blocks the UI.
- **A4 (P1)** User can include/exclude folders from scanning.
- **A5 (P1)** Duplicate detection using size + partial hash (flag only, never delete automatically).

### B. Gallery views
- **B1 (P0)** Three main views: **Videos**, **Images**, **Audio**.
- **B2 (P0)** Folder-aware browsing: opening a folder shows its media as thumbnails (images, video thumbnails, audio covers), not just filenames.
- **B3 (P0)** Grid sizes (small / medium / large) and a list view.
- **B4 (P0)** Sort by name, date, size, duration, type.
- **B5 (P1)** Grouping by date, folder, or device.
- **B6 (P0)** Built-in viewers: image viewer, video player, audio player.

### C. Search, filter, alias
- **C1 (P0)** Search by name, alias, date/time, type, folder, device.
- **C2 (P0)** **Auto-alias:** every file automatically gets a suggested alias generated from its file name, date, and location (see section 9). Search matches against both real names and aliases.
- **C3 (P0)** Rename a single file: edit alias, or apply as real rename.
- **C4 (P1)** Bulk rename with a pattern, e.g. `{date} {place} {name}`, with live preview and **undo**.
- **C5 (P1)** Filters: type, date range, time of day, size, duration, resolution, device, sync state.
- **C6 (P1)** Saved searches / smart folders.

### D. Connectivity and transfer
- **D1 (P0)** Pair phone and PC using a QR code or 6-digit PIN (one-time).
- **D2 (P0)** Auto-discover paired devices on the same Wi-Fi/LAN.
- **D3 (P0)** Send files PC to phone and phone to PC (drag and drop on PC, share/select on phone).
- **D4 (P1)** Wired mode over USB.
- **D5 (P1)** Bluetooth mode (slower; recommended for small files and as a fallback).
- **D6 (P0)** **Automatic transport choice** (wired > Wi-Fi LAN > Wi-Fi Direct > Bluetooth), with a manual override in settings.
- **D7 (P0)** Transfers are chunked, resumable, verified by hash, and shown in a transfer queue with progress, speed, pause, and cancel.
- **D8 (P1)** Connection status always visible in the top bar (Connect button turns into a status chip).

### E. Sync ("serverless drive")
- **E1 (P0)** Each device publishes its Media Index (metadata + thumbnails only) so other devices can browse it **without downloading files**.
- **E2 (P0)** Remote items show a thumbnail and a cloud badge. A **Download / Sync** button pulls the real file on demand.
- **E3 (P1)** Optional auto-sync rules (e.g. "always keep Camera photos synced to PC").
- **E4 (P1)** Preview/stream a remote file over LAN without saving it.
- **E5 (P0)** Works **without a Google account** on the same LAN (account only adds cross-session index sync and YouTube features).
- **E6 (P0)** If the owning device is offline, its remote items stay browsable (cached thumbnails) but Download is disabled with a clear message.

### F. Music player
- **F1 (P0)** Play, pause, seek, next/previous, shuffle, repeat, volume, queue.
- **F2 (P0)** Unified music library merging phone and PC tracks (de-duplicated view, source device shown).
- **F3 (P1)** Group by artist, album, folder, device; playlists.
- **F4 (P1)** Background playback on Android with media notification and lock screen controls; media keys on PC.
- **F5 (P0)** Tracks downloaded from YouTube appear automatically in the music library.

### G. YouTube
- **G1 (P0)** **Download from YouTube** popup: paste link, choose MP4 or MP3, quality, destination device/folder, then download via yt-dlp.
- **G2 (P0)** Downloads are saved automatically to the device library, tagged with title and thumbnail as cover.
- **G3 (P1)** Download queue with progress and error messages; playlist links supported.
- **G4 (P1)** Optional Google login also connects the user's YouTube account.
- **G5 (P1)** YouTube dropdown includes **My Channel** and **YouTube Studio** (open in browser).
- **G6 (P2)** **Upload** entry (open YouTube upload page, or direct upload of a selected video via API).

### H. Account
- **H1 (P1)** Optional Google login (never required to use the app).
- **H2 (P1)** Logout removes tokens and cached cloud index from the device.

### I. Cover and thumbnail editing
- **I1 (P0)** Video thumbnail is generated by capturing a frame at a **random point within the first third of the video**. Once chosen, it is **saved to cache and synced to cloud, and stays fixed**.
- **I2 (P1)** User can re-roll, pick a frame, or replace it with a custom image.
- **I3 (P0)** Audio cover editing in the right sidebar: replace, remove, or restore the cover (writes embedded tag art with user confirmation).
- **I4 (P0)** Cover crop with aspect presets: **4:3, 9:16, 16:9, 1:1, custom**.

### J. Folders, drag and drop, shortcuts
- **J1 (P0)** Drag and drop files between folders in the app and to/from the OS.
- **J2 (P0)** **Spring-loaded folders:** hovering a dragged item over a folder (about 700 ms) opens it. This also applies to folders in the left sidebar.
- **J3 (P0)** Pin folders as shortcuts to a **bottom bar**; customizable (add, remove, reorder, rename).
- **J4 (P1)** Drop a file on a device chip to send it to that device.

---

## 7. Desktop UI specification

### 7.1 Layout

```
+------+-----------------------------------------------------------------+
| [=]  | [Connect] [ Search................... ] [YT v]        [ Google ] |
|      +-----------------------------------------------+-----------------+
| Home |                                               | DETAILS         |
|      |            MAIN CONTENT AREA                  | (right sidebar) |
| LIB  |        (grid of thumbnails / list)            |  cover preview  |
| Vid  |                                               |  name / alias   |
| Img  |                                               |  metadata       |
| Aud  |                                               |  change cover   |
|      |                                               |  aspect ratio   |
| YT   |                                               |  send / sync    |
|      |                                               |                 |
+------+-----------------------------------------------+-----------------+
| [pinned folder] [pinned folder] [pinned folder] [+]   (bottom bar)      |
+-------------------------------------------------------------------------+
```

### 7.2 Left sidebar
- Big icons with labels. Sections:
  - **Home** (recent media, devices overview, storage, quick actions).
  - **Library** (name chosen by the product; alternatives: *Collections*, *Browse*): **Videos**, **Images**, **Audio**.
  - **YouTube**: Download, Downloads history, My Channel.
  - **Optional extra category: Devices & Transfers**: connected devices, transfer queue, sync rules. (Also optional: Favorites, Recent, Settings.)
- **Only the left sidebar is closable.** A toggle icon at the **top-left of the sidebar** collapses it to **icons only** and expands it again.
- When open, its **width is resizable** by dragging its edge (remembered between sessions).
- Folders inside it are drop targets with spring-loaded opening.

### 7.3 Top bar
- Placed **beside** the left sidebar (not above it), spanning the main area.
- Order left to right: **Connect** button (pairs/connects phone and PC, becomes a status chip when connected), **search bar**, **YouTube icon dropdown**, **Google account button** (optional login, at the far right like a profile button).
- **YouTube dropdown** (styled like YouTube's create/profile menu): **Upload**, **Download from YouTube**, **Your channel**, **YouTube Studio**. Items that need login show a "Sign in" prompt.
- **Download from YouTube** opens a modal with a link field, format and quality selectors, destination, and a Download button.

### 7.4 Main content area
- **Videos:** grid of video thumbnails (see I1). Shows duration badge, sync badge (cloud icon for remote items), and device icon. Remote items show the thumbnail plus a **Download / Sync** button.
- **Images:** justified or uniform grid; folder cards show a 2x2 preview of contents.
- **Audio:** list/grid with cover art, title, artist, duration, source device, and a play button.
- Folder cards are visible alongside files so browsing feels like a gallery, not a file tree.

### 7.5 Right details sidebar
- Appears when an item is selected (similar to File Explorer's details pane); collapsible per session.
- Shows: large cover/thumbnail, name, alias (editable), type, size, dates, duration, resolution or audio tags, location (if available), device, sync state, path.
- **Change cover** button, **crop with aspect presets (4:3, 9:16, 16:9, 1:1, custom)**, apply to videos and audio.
- Actions: Send to device, Download/Sync, Rename, Open folder, Delete (to recycle bin only).

### 7.6 Bottom bar
- Customizable **pinned folder shortcuts** (icon + name). Right-click to rename or remove; drag to reorder; drop files onto a shortcut to move them there.

### 7.7 Visual style
- Clean, modern, light and dark themes, large touch-friendly icons, smooth thumbnail loading (placeholder, then fade-in), keyboard shortcuts for common actions.

## 8. Android UI specification

- Same features, mobile layout: **bottom navigation** (Home, Library, YouTube, Devices).
- Library has tabs for Videos / Images / Audio.
- Top bar: search, Connect status chip, YouTube menu, account avatar.
- Details panel becomes a **bottom sheet**.
- Mini player at the bottom, full-screen player on tap.
- Share-sheet integration: "Send with Media Gallery".

---

## 9. Auto-alias and search rules

Alias suggestions are generated at scan time and updated when metadata changes.

| Source | Rule |
|---|---|
| File name | Remove prefixes (`IMG_`, `VID_`, `Screenshot_`), replace `_` and `-` with spaces, split camelCase, trim numbers |
| Date | EXIF date for photos; creation/recorded date for videos; fallback to file modified date. Format e.g. `2026-03-14` |
| Time | Used for filters ("morning", "evening") and optional alias token |
| Location | EXIF/video GPS to place name through reverse geocoding. Must be optional and respect privacy (offline lookup preferred; online lookup behind a setting) |
| Tags | Audio: `Artist - Title`; video from YouTube: video title |
| Device | Source device name |

- Default alias template: `{date} {place} {name}` (user-editable).
- Aliases are **non-destructive** by default. "Apply as real rename" renames the file on disk, records it in an undo log, and updates the index.
- Search index covers real name, alias, tags, date, place, device. Results should appear as the user types.

---

## 10. Connectivity and sync architecture

### 10.1 Principles
- **No file contents in the cloud.** Cloud holds only the metadata index and small thumbnails.
- **Every device is authoritative for its own files.** This avoids merge conflicts: each device only writes its own index records.
- Works offline on LAN; Google account is an optional enhancement.

### 10.2 Pairing and security
- Pair with QR code or 6-digit PIN; exchange public keys; all traffic is TLS-encrypted and mutually authenticated.
- Paired devices list with "Forget device".
- Transfers need user confirmation for first-time or unpaired senders.

### 10.3 Transports
1. **Wired (USB):** ADB port-forward or USB tethering link, then the same protocol as LAN. MTP as fallback for plain file access.
2. **Wi-Fi LAN:** mDNS discovery, HTTPS + WebSocket.
3. **Wi-Fi Direct:** for when there is no shared router.
4. **Bluetooth:** pairing, small files, last-resort fallback. The UI must warn that it is slow.

The app picks the fastest available transport automatically (D6).

### 10.4 Index sync
- Each device exports index deltas (new, changed, removed records) and thumbnails.
- With Google login: deltas are stored in Drive `appDataFolder` so devices see each other's libraries even when not connected at that moment.
- Without login: deltas are exchanged directly on connect.

### 10.5 On-demand file access
- Remote item selected: request file by ID, the owner device streams it in chunks to the requester, hash verified, then marked `Synced`.
- Preview mode streams without saving (E4).

### 10.6 Limitations to show users
- If the owning device is off or not reachable, files cannot be downloaded (E6).
- Cross-network access (e.g. phone on mobile data) needs a relay or WebRTC and is **out of scope for v1**.

---

## 11. Data model (minimum)

- **device**: id, name, type (phone/pc), public_key, last_seen, transports
- **media**: id, device_id, path, type, mime, size, hash, created_at, modified_at, duration, width, height, gps_lat, gps_lon, place, tags (json), alias, auto_alias, cover_id, sync_state
- **cover**: id, media_id, source (generated/custom/embedded), file_path, cloud_id, aspect, crop (json), frame_time_ms
- **transfer**: id, media_id, from_device, to_device, status, bytes_done, bytes_total, transport, error
- **pinned_folder**: id, device_id, path, label, icon, position
- **rename_log**: id, media_id, old_name, new_name, applied_at (for undo)
- **download_job**: id, url, format, quality, status, output_media_id, error
- **settings**: key, value (sidebar width, theme, transport priority, alias template, scan folders)

---

## 12. YouTube integration details

- Download via **yt-dlp** (+ ffmpeg for MP3 conversion and merging). Keep yt-dlp updatable inside the app (YouTube changes often).
- Metadata (title, uploader, thumbnail) written to tags; thumbnail saved as cover.
- Files saved automatically to the default library folder for that device (Videos / Music), configurable.
- Google login uses OAuth. Scopes requested: minimum needed (identity, Drive appDataFolder, YouTube read-only for channel info; upload scope only when Upload is built).
- **Compliance note:** downloading may conflict with YouTube's Terms of Service and with content owners' rights, and an app with a downloader generally cannot be listed on Google Play. Distribute as direct APK / desktop installer, and show a short "download only content you have the right to" notice.

---

## 13. Non-functional requirements

- **Performance:** scan 100,000 files without freezing the UI; first thumbnails visible within 2 s of opening a folder; thumbnails lazy-loaded and cached (LRU, size-limited).
- **Offline-first:** everything local works with no internet.
- **Privacy:** no analytics of file names or locations leaving the device by default; location lookup optional; clear data-deletion option.
- **Security:** encrypted transfers, paired-device allow list, tokens in OS secure storage.
- **Reliability:** resumable transfers, safe rename with undo, deletes go to trash.
- **Battery (Android):** background scan throttled; foreground service only for active transfers/downloads/playback.
- **Accessibility:** keyboard navigation on desktop, scalable text, sufficient contrast.
- **Localization-ready:** all strings externalized (English first, Indonesian likely next).

---

## 14. Milestones

| # | Milestone | Outcome |
|---|---|---|
| M0 | Foundation | Project setup, DB schema, theming, navigation shell with the sidebar, top bar, bottom bar |
| M1 | Desktop library (MVP core) | Scan, gallery views, thumbnails, details panel, search, alias, cover editing, drag and drop |
| M2 | Android app | Scan via MediaStore, same views, mobile layout, music player |
| M3 | Pairing + LAN transfer | QR/PIN pairing, discovery, send files both ways, transfer queue |
| M4 | Index sync + on-demand download | Remote items, cloud badges, Download/Sync, optional Google login |
| M5 | YouTube | Download popup, yt-dlp on desktop and Android, dropdown menu |
| M6 | Wired + Bluetooth | USB mode, Bluetooth fallback, auto transport selection |
| M7 | Polish | Bulk rename, auto-sync rules, duplicates, performance pass, installers |

---

## 15. Risks

| Risk | Mitigation |
|---|---|
| Bluetooth and Wi-Fi Direct APIs differ a lot across Android versions/vendors | Build LAN first; treat Bluetooth as fallback; test on several devices |
| Android storage permissions get stricter | Use MediaStore/scoped storage; request broad access only if needed |
| yt-dlp breaks when YouTube changes | Built-in updater; surface clear error messages |
| Play Store and ToS issues with downloader | Side-load distribution; keep downloader modular |
| "Serverless" limits (device must be on) | Cache thumbnails/metadata, explain states clearly in UI |
| Large libraries make the UI slow | Virtualized lists, paged DB queries, thumbnail cache |
| Reverse geocoding privacy | Offline by default, online lookup opt-in |

---

## 16. Assumptions and open questions

### Assumptions made while writing this PRD
1. The prompt mentioned "login to Google" twice in the top bar. It is treated as **one optional Google account button** at the far right.
2. "Alias" means a **display name stored in the app**, with an optional "apply as real rename" action.
3. "Saved in cache and cloud" for thumbnails means a **local cache plus the small metadata store in the user's Google Drive app data**, not a server owned by the product.
4. The name for the second sidebar category is **Library**.
5. Desktop target is Windows first.
6. Remote access outside the same network is v2.

### Open questions for the product owner
1. Final product name?
2. Should Bluetooth and wired transfers be in the first release, or LAN first?
3. Is a Google account required for any feature, or always optional? (Current plan: always optional.)
4. Is the YouTube **Upload** feature really needed in v1?
5. Is a macOS/Linux release wanted at launch, or Windows + Android only?
6. Should deleting files be allowed from inside the app?

---

## 17. MVP acceptance criteria

1. Launch the desktop app; it scans a folder with 10,000 mixed media files and shows Videos, Images, and Audio views with thumbnails, without freezing.
2. Video thumbnails come from a random frame in the first third, and stay the same after restart.
3. Selecting a file opens the right sidebar; changing an audio cover with a 16:9 crop saves and shows immediately.
4. Searching "2026 beach" finds files by auto-alias (date + place) as well as by real file name.
5. Dragging a file over a folder opens it after about 700 ms and the drop moves the file.
6. The left sidebar collapses to icons only, expands again, and remembers its width.
7. The phone and PC pair via QR code on the same Wi-Fi; a 1 GB video sent from PC to phone completes with hash verification and can resume after interruption.
8. The PC shows phone videos as thumbnails with a cloud badge; pressing Download pulls the real file.
9. Pasting a YouTube link in the popup downloads an MP3 or MP4 into the library with title and cover.
10. The app works fully on LAN with no Google login.
