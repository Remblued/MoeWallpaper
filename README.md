# MoeWallpaper

**Lightweight anime wallpapers for Windows.** Tauri 2 · vanilla TypeScript · no bundled images.

[简体中文](README.zh-CN.md) · [How the size numbers are measured](docs/BENCHMARK.md) · [MIT License](LICENSE)

> **Status: pre-`v0.1.0`, built in public.**
> ✅ done · 🚧 in progress · 📋 planned. Only the [Roadmap](#roadmap) describes what actually ships.

## The idea

Desktop wallpaper apps are usually Electron apps. That is a reasonable engineering
choice, but it means a 100 MB+ download before the user sees a single picture, for an
app whose main job is "show a grid of images and set one as the wallpaper".

MoeWallpaper makes one bet — a wallpaper app does not need a full browser runtime
bundled per app — and three commitments to go with it:

1. **Small install.** Measured at **1.81 MB**, not estimated. See
   [Size, measured not claimed](#size-measured-not-claimed).
2. **Sources are data, not code.** Adding an image source means editing a JSON file.
   There is no plugin runtime, no scripting layer, no per-source native module.
3. **No bundled images.** This repository contains no wallpaper files. Sources point
   at public APIs; only thumbnails and the current wallpaper are cached, locally.

## Network requirement

**Install it, open it, and you see images. No proxy needed.**

Measured on 2026-10-31 (Windows 11, direct Wi-Fi, system proxy off, no TUN tunnel):
**4 of the 5 built-in sources work on a direct connection.**

| Source | Direct | Note |
|---|---|---|
| 360 Wallpaper · anime | ✅ | Listed first, so it is what you see on launch |
| Safebooru | ✅ | Anime illustrations; `width:>=1920` is filtered server-side, thumbnails included, tag search supported |
| nekos.best | ✅ | neko category only; **no keyword search** (its search is fuzzy ranking and returns unrelated images) |
| Lolicon · Pixiv | ✅ | Images go through the `i.pixiv.re` mirror (upstream `i.pximg.net` does not connect); **labelled "尺度较大" in the UI** |
| Wallhaven | ❌ connection timeout | The only one that needs a proxy |

**"Reachable" is a property of the exit network and the moment, not of the site.** On the
same day nekos.best answered 403 in the morning (IP-based blocking) and 200 in the
afternoon; lolicon has done the same. The cause sits outside the app. So the table above
means "measured on this network, at that time" — not "guaranteed available in mainland
China".

The app **reads the Windows system proxy setting automatically** (the one under
"Internet Options"); there is nothing to configure inside the app. It is used only when
reaching the overseas sources.

Three known traps:

- **nekos.best blocks by IP.** It answers 403 with "Your IP address has been
  automatically blocked by our security system". Shared exit IPs hit this often;
  switching node fixes it.
- **A proxy client with the system proxy switched off is the same as no proxy.**
- **360 has no content flag.** We pin the anime category (`cid=26`), but unlike
  wallhaven's `purity` or lolicon's `r18` there is no parameter that guarantees the
  content. That limitation is written down in [docs/SOURCES-CN.md](docs/SOURCES-CN.md)
  rather than hidden.
## Size, measured not claimed

Every number in this table is produced on one machine, with one method, on the same
day, from a **release** build. `TBD` means *not measured yet* — **all four cells are
measured, there is no `TBD` left**. The method lives in [docs/BENCHMARK.md](docs/BENCHMARK.md).

| Metric | MoeWallpaper `v0.1.2` | Minimal Electron app | How it is measured |
| --- | --- | --- | --- |
| Installer size (`.exe`) | **1.81 MB** | **115.57 MB** | file size of the installer produced by the build |
| Installed size on disk | **5.39 MB** | **356.77 MB** | sum of all files under the install directory |
| Cold start to visible window | **41 ms** (median) | **104.5 ms** (median) | median of 10 launches, from CreateProcess to first window |
| Idle memory (60 s after launch) | **218.3 MB** (private, empty grid)<br>**293.7 MB** (private, 20 images loaded) | **133.5 MB** (private, blank window) | whole process tree, not just the main process; both window states measured |

The installer is **about 64× smaller** (115.57 MB versus 1.81 MB), the installed
footprint is **about 66× smaller**, and cold start is **about 2.5× faster**.

Measurement environment (see the [BENCHMARK record](docs/BENCHMARK.md)):
Intel i7-14650HX / 31.6 GB / Windows 11 build 26200 / release build
(LTO + strip) / `v0.1.2` plus the 360-thumbnail fix.

**Three caveats that belong with these numbers:**

- **The installer does not include the WebView2 runtime.** Windows 11 ships it;
  Windows 10 downloads roughly 100 MB on first install. So this is not
  "1.81 MB everywhere".
- **Cold start is *window appears*, not *page rendered*.** The latter is a few
  hundred milliseconds later. The very first launch after installing measured
  204 ms (AV scan + cold file read); the steady median is 41 ms.
- **Idle memory is meaningless without the window state.** Decoded images land in
  private memory: the same exe sits at 218.3 MB with an empty grid (median of 5,
  211.0–245.9) and 293.7 MB once the default source's 20 images are on screen
  (median of 3, 274.5–297.8). Reporting a single number would merge two different
  scenarios.

Idle memory is reported as **private**, not working set (379.9 MB empty, 403.1 MB
loaded) — summing working sets double-counts DLL pages shared between the app and
WebView2 processes.

**This is the metric we lose, but the comparison has to match.** The Electron
baseline is **a blank window that loads no images**, so the only row it can be
compared with is our *empty grid*: **218.3 MB against 133.5 MB, 84.8 MB more
(1.63×)**. The gap shows up in the process count: we run 7 processes, Electron 4,
and our idle GPU process alone holds 115–137 MB private. **The loaded 293.7 MB must
not be compared with the blank 133.5 MB** — that would be "20 wallpapers" against
"nothing on screen". An Electron app showing the same 20 images would grow too;
this minimal baseline simply cannot measure that side.

That 356.77 MB is not padding either: a single file, `BaselineElectron.exe`, is
238.23 MB — 44× the entire installed footprint of this project (5.39 MB).

**What the images cost is now measured, and it has already been cut once.** The
default source (360) originally mapped `thumbnailUrl` to `url_thumb`, which turned
out to return the 2560×1440 original rather than a thumbnail, so the 20-card grid
decoded 20 full-resolution wallpapers: 465.8 MB loaded, 267 MB more than the empty
grid ([SOURCES-CN §2](docs/SOURCES-CN.md)). Mapping it to `img_1280_800`, the size
variant the API actually produces, took the loaded figure from **465.8 MB to
293.7 MB (−172 MB)** and narrowed the spread from 187–580 MB to 274–298 MB. The
remaining ~75 MB is the cost of "20 1280×800 images"; going lower would mean fewer
cards or server-side scaling — **not done, not measured**.

**The 233 MB recorded for `a65d7ed` is retired**: that run measured a
wallhaven-thumbnail window, which is not the state this version ships with, and
the "no images" state had never been measured at all. The old figure stays in the
[BENCHMARK record](docs/BENCHMARK.md); do not quote the two interchangeably.

Two rules keep this honest:

- The baseline is a **hello-world Electron app** packaged with electron-builder, not
  VS Code or Discord. Comparing a wallpaper app to a code editor would be cheating.
- That hello world opens a **blank window** and loads no images. So 115.57 MB is
  Electron's **floor**, not its ceiling — a real Electron wallpaper app is larger.
- If a measurement contradicts a claim, **the claim changes**. The numbers are the
  product; the marketing line is not.

## Features (v0.1.0)

| Feature | Status |
| --- | --- |
| Fetch from declarative JSON sources (360 / safebooru / wallhaven / nekos.best / lolicon) | ✅ |
| Thumbnail grid with skeleton and broken-image fallback | ✅ |
| Detail view: full image, set as wallpaper, download, open source page | ✅ |
| Keyword search (Chinese tag dictionary, per-source search endpoints) | ✅ |
| Pagination | ✅ |
| Resolution filter (≥1080p / ≥1440p / ≥4K) | ✅ |
| Dark / light / glass themes, remembered; **glass follows the system light/dark setting** | ✅ |
| One-click wallpaper (Win32 `SystemParametersInfo`) | ✅ |
| Save to Pictures without overwriting | ✅ |
| Local cache (thumbnails, current wallpaper) | ✅ |
| Declarative JSON sources | ✅ |
| Domestic (CN) sources | ✅ 360 Wallpaper (the default source) |
| Scheduled rotation | ❌ deliberately cut, see below |

### Not in v0.1.0

Favourites, accounts, cloud sync, auto-update, multi-monitor, scheduled rotation.
Being small is the feature; anything not listed above is deliberately postponed.

**Scheduled rotation was cut on purpose**, not left unfinished: it needs a resident
timer and a "go to the next one" state machine, while keyword search solves the same
want — "show me this kind of picture" — with far less code and far fewer edge cases.

## Declarative sources

A source is a JSON object describing **how to ask** and **how to read the answer**:

```json
{
  "id": "example",
  "name": "Example source (template)",
  "enabled": false,
  "request": {
    "url": "https://example.invalid/api/posts?limit={limit}&page={page}",
    "headers": { "Accept": "application/json" }
  },
  "response": {
    "itemsPath": "data",
    "imageUrl": "file_url",
    "thumbnailUrl": "sample_url",
    "tags": "tags"
  },
  "filter": { "sfwOnly": true }
}
```

- Built-in definitions live in [`sources/builtin/`](sources/builtin/); the contract is
  typed in [`src/lib/sources/schema.ts`](src/lib/sources/schema.ts).
- `{limit}` and `{page}` are the only placeholders the app substitutes.
- **SFW by default.** A source only returns content it can mark as safe unless the user
  explicitly opts out; the default is never "off".
- Source definitions never contain API keys. A source that needs one is out of scope
  for the built-in list.

## Where your data goes

- No account, no telemetry, no analytics.
- Thumbnails and the current wallpaper are cached under the app's local data
  directory (`%LOCALAPPDATA%\<app identifier>\`), never in this repository.
- Clearing the cache is a normal app action, not a reinstall.

## Build from source

Requirements (tested versions in brackets):

- Windows 10/11 x64 (tested: Windows 11 build 26200)
- [Rust](https://rustup.rs/) stable, MSVC toolchain (tested: 1.99.1)
- Visual Studio Build Tools with the C++ workload + Windows SDK (tested: MSVC 14.51, SDK 10.0.26100)
- Node.js ≥ 20 (tested: 24.17.0)
- WebView2 Runtime (preinstalled on current Windows 11)

```powershell
npm install            # installs the frontend toolchain into node_modules/
npm run tauri dev      # builds the Rust side, starts Vite, opens the app
npm run tauri build    # release build; the installer lands in src-tauri/target/release/bundle/
```

`npm run build` alone type-checks the TypeScript and builds the static frontend; it
does **not** produce an app.

## Project layout

```text
.
├─ src/                         # frontend (vanilla TypeScript, built by Vite)
│  ├─ main.ts                   # entry point: mounts the UI
│  ├─ lib/                      # framework-free logic, unit-testable
│  │  └─ sources/schema.ts      # the JSON source contract
│  ├─ ui/                       # 📋 DOM rendering and event wiring
│  └─ assets/                   # static assets shipped with the frontend
├─ sources/builtin/             # shipped source definitions (JSON only, no images)
├─ src-tauri/                   # Rust side
│  ├─ src/main.rs               # process entry, calls the library
│  ├─ src/lib.rs                # app setup, Tauri builder
│  ├─ src/commands/             # 📋 #[tauri::command] surface exposed to the frontend
│  ├─ src/sources/              # 📋 load and fetch sources
│  ├─ src/wallpaper/            # 📋 set and cache wallpapers
│  ├─ Cargo.toml
│  └─ tauri.conf.json
├─ docs/BENCHMARK.md            # how the size and speed numbers are produced
├─ LICENSE
└─ README.md · README.zh-CN.md
```

Directories marked 📋 do not exist yet on purpose: git does not track empty
directories, so a folder is created when its first real file is written, not before.

## Roadmap

| Version | Scope | Status |
| --- | --- | --- |
| `v0.1.1` | **A domestic source that works without a proxy** (360 anime category), plus offset paging, combined-resolution fields and an image https upgrade | ✅ |
| `v0.1.2` | Glass theme **follows the system light/dark setting** (semi-transparent, never fully transparent); new safebooru source and a content note on lolicon | ✅ |
| `v0.2.0` | **Desktop widget** and **user-supplied APIs** | 📋 |
| `v0.3.0` | Desktop widget (桌面挂件) | 📋 |
| `v0.4.0` | Pluggable sources, if the declarative format ever stops being enough | 📋 |
| unscheduled | CI, auto-update, UI i18n, focus trap in the detail view | 📋 |

`v0.1.1` is the domestic source that was originally scheduled for `v0.2.0`. Without it
the app is empty on first launch inside mainland China, which outranks any new feature.
`v0.2.0` therefore moves to two different things: a desktop widget, and letting users
plug in their own API without waiting for us to add a source.

No `v0.1.0` tag was ever published, so `v0.1.1` is the first genuinely usable release.

Scope may shrink. Versions are cut when the previous one is genuinely usable.

## Contributing

This is a first open-source project, written in public so the process is visible.
Issues and corrections are welcome, especially about the benchmark method — a number
nobody can reproduce is a bug.

Please do not open pull requests that add wallpaper image files to the repository.

## License

[MIT](LICENSE) © 2026 Remblued
