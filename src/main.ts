import { invoke } from "@tauri-apps/api/core";

/** 和后端 WallpaperSource 的序列化结果对应（camelCase）。 */
interface WallpaperSource {
  id: string;
  name: string;
  enabled?: boolean;
  homepage?: string | null;
}

const statusEl = document.getElementById("status");
const listEl = document.getElementById("sources");

function render(sources: WallpaperSource[]) {
  if (!listEl) return;

  listEl.innerHTML = sources
    .map((s) => `<li><strong>${s.name}</strong> <code>${s.id}</code></li>`)
    .join("");
}

async function load() {
  try {
    const sources = await invoke<WallpaperSource[]>("list_sources");
    render(sources);

    if (statusEl) {
      statusEl.textContent = `已载入 ${sources.length} 个图源`;
    }
  } catch (err) {
    if (statusEl) {
      statusEl.textContent = `载入失败：${String(err)}`;
    }
  }
}

window.addEventListener("DOMContentLoaded", load);
