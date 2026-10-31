// Launcher workaround for packaging from inside an AI harness (DSH / any Electron
// host that runs node as `YourApp.exe`).
//
// Symptom: `npm run tauri build` -> `error: unrecognized subcommand
// 'D:\...\YourApp.exe'`.
// Cause: @tauri-apps/cli/tauri.js reads process.argv[0] to guess its own binary
// name; when that is an Electron host exe the guess fails and it prepends the path
// as a subcommand. Rewriting argv before requiring the CLI fixes it.
//
// Usage, from the repository root (the directory that holds package.json):
//   node bench/measure/tauri-cli-workaround.js build
//   node bench/measure/tauri-cli-workaround.js build --bundles nsis
//
// This mirrors `npm run tauri build` (the CLI runs beforeBuildCommand itself, i.e.
// `tsc && vite build`). Do not "fix" this by changing package.json or downgrading
// the CLI: the problem is the host process, not the project.

const path = require('path');

process.argv = ['node.exe', 'tauri.js', ...process.argv.slice(2)];
require(path.join(process.cwd(), 'node_modules', '@tauri-apps', 'cli', 'tauri.js'));
