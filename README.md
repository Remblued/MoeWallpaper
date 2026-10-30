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

1. **Small install.** Measured at **1.69 MB**, not estimated. See
   [Size, measured not claimed](#size-measured-not-claimed).
2. **Sources are data, not code.** Adding an image source means editing a JSON file.
   There is no plugin runtime, no scripting layer, no per-source native module.
3. **No bundled images.** This repository contains no wallpaper files. Sources point
   at public APIs; only thumbnails and the current wallpaper are cached, locally.

## Size, measured not claimed

Every number in this table is produced on one machine, with one method, on the same
day, from a **release** build. `TBD` means *not measured yet* — it will stay `TBD`
until someone can reproduce it. The method lives in [docs/BENCHMARK.md](docs/BENCHMARK.md).

| Metric | MoeWallpaper | Minimal Electron app | How it is measured |
| --- | --- | --- | --- |
| Installer size (`.exe`) | **1.69 MB** | **115.57 MB** | file size of the installer produced by the build |
| Installed size on disk | **5.19 MB** | **356.77 MB** | sum of all files under the install directory |
| Cold start to visible window | **41 ms** (median) | TBD | median of 10 launches, from `Start-Process` to first window |
| Idle memory (60 s after launch) | **212 MB** (private) | TBD | all app + webview processes, not just the main one |

The installer is **about 68× smaller** (115.57 MB versus 1.69 MB).

Measurement environment (see the [BENCHMARK record](docs/BENCHMARK.md)):
Intel i7-14650HX / 31.6 GB / Windows 11 build 26200.9711 / release build
(LTO + strip) / commit `74d4170`.

**Two caveats that belong with these numbers:**

- **The installer does not include the WebView2 runtime.** Windows 11 ships it;
  Windows 10 downloads roughly 100 MB on first install. So this is not
  "1.69 MB everywhere".
- **Cold start is *window appears*, not *page rendered*.** The latter is a few
  hundred milliseconds later.

Idle memory is reported as **private** (212 MB), not working set (371 MB) — summing
working sets double-counts DLL pages shared between the app and WebView2 processes.
The Electron baseline's cold start and idle memory are still `TBD`: both need a
desktop session that can launch GUI processes, following
[docs/BENCHMARK.md](docs/BENCHMARK.md) section 3.

That 356.77 MB is not padding either: a single file, `BaselineElectron.exe`, is
238.23 MB — 46× the entire installed footprint of this project (5.19 MB).

Worth saying plainly: most of that memory is the WebView2 shell — the GPU process
alone holds 117 MB private, against 6 MB for our own process. That is not something
this project can optimise away, and the README should not pretend otherwise.

Two rules keep this honest:

- The baseline is a **hello-world Electron app** packaged with electron-builder, not
  VS Code or Discord. Comparing a wallpaper app to a code editor would be cheating.
- That hello world opens a **blank window** and loads no images. So 115.57 MB is
  Electron's **floor**, not its ceiling — a real Electron wallpaper app is larger.
- If a measurement contradicts a claim, **the claim changes**. The numbers are the
  product; the marketing line is not.

## Features (v0.1.0)

| | Feature | Status |
| --- | --- | --- |
| 1 | Fetch images from declarative JSON sources | 🚧 |
| 2 | Thumbnail grid | 🚧 |
| 3 | Set as wallpaper in one click | 🚧 |
| 4 | Scheduled rotation | 🚧 |
| 5 | JSON source configuration | 🚧 |
| — | Scaffold (Tauri 2 + Vite + TypeScript) | ✅ |

### Not in v0.1.0

Search, favourites, accounts, cloud sync, auto-update, multi-monitor. Being small is
the feature; anything not on the list above is deliberately postponed.

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
| `v0.1.0` | The five features above | 🚧 |
| `v0.2.0` | Desktop widget (桌面挂件) | 📋 |
| `v0.3.0` | Desktop tidy-up (桌面整理) | 📋 |
| `v0.4.0` | Pluggable sources, if the declarative format ever stops being enough | 📋 |
| unscheduled | CI, auto-update, UI i18n | 📋 |

Scope may shrink. Versions are cut when the previous one is genuinely usable.

## Contributing

This is a first open-source project, written in public so the process is visible.
Issues and corrections are welcome, especially about the benchmark method — a number
nobody can reproduce is a bug.

Please do not open pull requests that add wallpaper image files to the repository.

## License

[MIT](LICENSE) © 2026 Remblued
