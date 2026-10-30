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

## Network requirement (read this first)

**All three built-in sources are hosted outside mainland China, and none of them is
reachable from a direct connection here.** This is not a misconfiguration and not an
open bug — wallhaven, nekos.best and lolicon are all blocked at the carrier or campus
network level. Someone tested it: with no proxy, all three sources fail to connect.

So you need outbound access, for example Clash or v2ray running as a system proxy.

The app **reads the Windows system proxy setting automatically** (the one under
"Internet Options"). There is nothing to configure inside the app. If the proxy client
is running and the system proxy is on, the app uses it.

How to tell whether you are set up:

- your browser can open <https://wallhaven.cc/> → the app can too
- your browser cannot → the app will fail, but the error carries the status code and
  **the server's own words**, not just "load failed"

Three known traps:

- **nekos.best blocks by IP.** It answers 403 with "Your IP address has been
  automatically blocked by our security system". Shared exit IPs on cheap VPN nodes
  hit this often; switching node fixes it. Not an app problem.
- **A proxy client with the system proxy switched off is the same as no proxy.** The
  app falls back to a direct connection, and we just established that direct does not
  work.
- **Domestic sources do not exist yet.** Candidates have been researched and tested,
  see [docs/SOURCES-CN.md](docs/SOURCES-CN.md): the 360 wallpaper category list works
  on a direct connection, but wiring it up needs three capabilities first.

## Size, measured not claimed

Every number in this table is produced on one machine, with one method, on the same
day, from a **release** build. `TBD` means *not measured yet* — **all four cells are
measured, there is no `TBD` left**. The method lives in [docs/BENCHMARK.md](docs/BENCHMARK.md).

| Metric | MoeWallpaper | Minimal Electron app | How it is measured |
| --- | --- | --- | --- |
| Installer size (`.exe`) | **1.81 MB** | **115.57 MB** | file size of the installer produced by the build |
| Installed size on disk | **5.38 MB** | **356.77 MB** | sum of all files under the install directory |
| Cold start to visible window | **29.5 ms** (median) | **104.5 ms** (median) | median of 10 launches, from CreateProcess to first window |
| Idle memory (60 s after launch) | **233 MB** (private) | **133.5 MB** (private) | all app + webview processes, not just the main one |

The installer is **about 64× smaller** (115.57 MB versus 1.81 MB) and cold start is
**about 3.5× faster**.

Measurement environment (see the [BENCHMARK record](docs/BENCHMARK.md)):
Intel i7-14650HX / 31.6 GB / Windows 11 build 26200.9711 / release build
(LTO + strip) / commit `74d4170`.

**Two caveats that belong with these numbers:**

- **The installer does not include the WebView2 runtime.** Windows 11 ships it;
  Windows 10 downloads roughly 100 MB on first install. So this is not
  "1.81 MB everywhere".
- **Cold start is *window appears*, not *page rendered*.** The latter is a few
  hundred milliseconds later.

Idle memory is reported as **private** (233 MB), not working set (399 MB) — summing
working sets double-counts DLL pages shared between the app and WebView2 processes.

**This is the metric we lose: 233 MB against 133.5 MB, 99 MB more.** We do not bundle
a browser engine; we borrow the system WebView2, which spawns three extra helper
processes and puts 117 MB private into its GPU process alone. Electron's bundled
Chromium holds the same blank page in 133.5 MB. **It is written down here rather than
softened.**

That 356.77 MB is not padding either: a single file, `BaselineElectron.exe`, is
238.23 MB — 44× the entire installed footprint of this project (5.38 MB).

That 117 MB GPU process is not something this project can optimise away, and no amount
of work on image loading would move it — the memory is not on our side of the fence.

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
| Fetch from declarative JSON sources (wallhaven / nekos.best / lolicon) | ✅ |
| Thumbnail grid with skeleton and broken-image fallback | ✅ |
| Detail view: full image, set as wallpaper, download, open source page | ✅ |
| Keyword search (Chinese tag dictionary, per-source search endpoints) | ✅ |
| Pagination | ✅ |
| Resolution filter (≥1080p / ≥1440p / ≥4K) | ✅ |
| Dark / light theme, remembered | ✅ |
| One-click wallpaper (Win32 `SystemParametersInfo`) | ✅ |
| Save to Pictures without overwriting | ✅ |
| Local cache (thumbnails, current wallpaper) | ✅ |
| Declarative JSON sources | ✅ |
| Domestic (CN) sources | 📋 see [Roadmap](#roadmap) |
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
| `v0.1.0` | The whole feature table above | 🚧 feature-complete, pending release checks |
| `v0.2.0` | **Domestic (CN) sources** — all three sources are unusable without a proxy | 📋 |
| `v0.3.0` | Desktop widget (桌面挂件) | 📋 |
| `v0.4.0` | Pluggable sources, if the declarative format ever stops being enough | 📋 |
| unscheduled | CI, auto-update, UI i18n, focus trap in the detail view | 📋 |

**On domestic sources**: this is the most important gap right now. The difficulty is
that most free Chinese endpoints return a single image (a 302 to one picture) rather
than a list, and this project's source format only reads lists. 360 Wallpaper has an
anime category, but it is not a public API, its terms are unclear, and its `start`
parameter is an offset rather than a page number. Supporting that shape means adding a
second kind of source, which is what v0.2.0 is for.

Scope may shrink. Versions are cut when the previous one is genuinely usable.

## Contributing

This is a first open-source project, written in public so the process is visible.
Issues and corrections are welcome, especially about the benchmark method — a number
nobody can reproduce is a bug.

Please do not open pull requests that add wallpaper image files to the repository.

## License

[MIT](LICENSE) © 2026 Remblued
