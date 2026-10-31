# Download

> This page is the project's canonical download page. It is also the page referenced by our
> [SignPath Foundation](https://signpath.org/) application, which is why it says plainly what
> the signing status is — including when it is *not* signed yet.

## Get the installer

Latest release: <https://github.com/Remblued/MoeWallpaper/releases/latest>

The only artifact is the NSIS installer, `MoeWallpaper_<version>_x64-setup.exe`. There is no
portable ZIP, no auto-updater, and nothing is downloaded after installation except wallpapers
from the sources you pick inside the app.

## Code signing

**Code signing for MoeWallpaper is provided by the [SignPath Foundation](https://signpath.org/)**,
which offers free code signing to open-source projects: the private key stays on SignPath's HSM
and they vouch that the binary was built from this repository.

**Status of the current release (`v0.1.2`): not signed yet.** The SignPath application is in
progress; until it is approved, the installer will show a SmartScreen warning and some antivirus
products may block it. Two concrete consequences, so nobody is surprised:

- Windows SmartScreen may say "Windows protected your PC" — that is what an unsigned, new
  publisher looks like, not a sign that something is wrong with the file.
- Verify the download with the SHA-256 published on the release page instead of trusting a
  signature (see below). Once signing is live, this page will say so and the hash will be
  re-recorded for the signed build.

## Verify what you downloaded

```powershell
Get-FileHash .\MoeWallpaper_0.1.2_x64-setup.exe -Algorithm SHA256
Get-AuthenticodeSignature .\MoeWallpaper_0.1.2_x64-setup.exe   # Status: NotSigned (for now)
```

The expected hash for each release is on that release's page, and the build it came from is
recorded in [docs/BENCHMARK.md](BENCHMARK.md) §5 (machine, toolchain, exact bytes). A hash
belongs to one build: rebuilding the same tag in CI produces different bytes, so always compare
against the hash published next to *that* file.

> Please do not redistribute modified builds under this name. If you build from source, say so.

## Requirements

- Windows 10 or 11, x64.
- WebView2 runtime: included in Windows 11; Windows 10 downloads it during installation
  (the installer uses the Microsoft bootstrapper, roughly 100 MB on first install).
- No account, no telemetry, no bundled images. See [PRIVACY.md](PRIVACY.md).

## Build it yourself

```powershell
npm install
npm run tauri dev          # development
npm run tauri build        # release, produces the NSIS installer
```

Toolchain requirements and the full gate list (`cargo fmt` / `clippy` / `test`, `tsc`, `vite
build`) are in [docs/HANDOVER.md](docs/HANDOVER.md) §2.
