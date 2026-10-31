# Download

> This page is the project's canonical download page. It also states the signing status plainly,
> including when nothing is signed yet — a download page that only says "free" and hides that is
> not worth much.

## Get the installer

Latest release: <https://github.com/Remblued/MoeWallpaper/releases/latest>

The only artifact is the NSIS installer, `MoeWallpaper_<version>_x64-setup.exe`. There is no
portable ZIP, no auto-updater, and nothing is downloaded after installation except wallpapers
from the sources you pick inside the app.

## Code signing

**None of the current releases are code-signed.** We applied to the
[SignPath Foundation](https://signpath.org/) (free signing for open-source projects: private key on
their HSM, and they vouch that the binary was built from this repository) and were declined on
2026-10-31 — not for anything wrong with the build, but because the project has no public
visibility yet, and that programme requires exactly that (stars, contributors, external articles,
independent discussions). Their paid plans start at $500/year and require a legally registered
organisation for the certificate, so they are not the right answer for a one-person project.

Consequences, so nobody is surprised:

- Windows SmartScreen may say "Windows protected your PC" — that is what an unsigned, unknown
  publisher looks like, not a sign that the file is wrong.
- Some antivirus products block unsigned NSIS installers heuristically.
- **Verify the download by SHA-256 instead** (below). The signing situation will be revisited
  when the project has the visibility the programme asks for; this page will be updated the day
  that changes rather than quietly.

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
