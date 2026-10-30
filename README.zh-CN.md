# MoeWallpaper

**轻量的 Windows 二次元壁纸应用。** Tauri 2 · 原生 TypeScript · 不内置任何图片。

[English](README.md) · [轻量数据怎么测](docs/BENCHMARK.md) · [MIT 许可](LICENSE)

> **状态：`v0.1.0` 之前，边做边公开。**
> ✅ 已完成 · 🚧 进行中 · 📋 计划中。只有 [Roadmap](#roadmap) 里的内容才算数。

## 这个项目想解决什么

桌面壁纸应用通常是 Electron 应用。这本身是合理的工程选择，代价是：为了"显示一
格图片、把它设成壁纸"这一件事，用户要先下载 100 MB 以上。

MoeWallpaper 押一个判断：壁纸应用不需要为每个应用内置一整套浏览器运行时。围绕这个
判断有三条承诺：

1. **安装包小。** 实测 **1.69 MB**，不是估算。见
   [体积：测量值，不是宣称值](#体积测量值不是宣称值)。
2. **图源是数据，不是代码。** 加一个图源 = 改一个 JSON 文件。没有插件运行时、没有脚本
   层、没有每个图源都要编译的原生模块。
3. **不内置图片。** 仓库里没有一张壁纸。图源指向公开 API，只有缩略图和当前壁纸缓存在
   用户本地。

## 体积：测量值，不是宣称值

下表中每个数字都来自同一台机器、同一套方法、同一天、**release** 构建。`TBD` 表示
*尚未测量*——**目前四格全部测过，没有 `TBD`**。测量方法见
[docs/BENCHMARK.md](docs/BENCHMARK.md)。

| 指标 | MoeWallpaper | 最小 Electron 应用 | 怎么测 |
| --- | --- | --- | --- |
| 安装包体积（`.exe`） | **1.69 MB** | **115.57 MB** | 构建产出的安装包文件大小 |
| 安装后磁盘占用 | **5.19 MB** | **356.77 MB** | 安装目录下所有文件大小之和 |
| 冷启动到窗口可见 | **17.5 ms**（中位数） | **104.5 ms**（中位数） | 启动 10 次取中位数，从 CreateProcess 到窗口出现 |
| 空闲内存（启动 60 秒后） | **218 MB**（私有） | **133.5 MB**（私有） | 应用进程 + webview 子进程之和，不是只算主进程 |

安装包体积差 **约 68 倍**（115.57 MB 对 1.69 MB），冷启动快 **约 6 倍**。

测量环境（见 [BENCHMARK 记录](docs/BENCHMARK.md)）：Intel i7-14650HX / 31.6 GB /
Windows 11 build 26200.9711 / release 构建（LTO + strip）/ commit `74d4170`。

**两个必须一起看的口径说明：**

- **安装包不含 WebView2 运行时。** Windows 11 自带；Windows 10 首次安装会额外下载
  约 100 MB。所以这不是"在任何机器上都只下 1.69 MB"。
- **冷启动是「窗口出现」，不是「画面渲染完成」。** 后者会再晚几百毫秒。

空闲内存报的是**私有内存**（218 MB），不是工作集（382 MB）——工作集把多个进程
共享的 DLL 重复计算了，数字会虚高。

**但这一项我们输了：218 MB 对 133.5 MB，多 85 MB。** 我们不打包浏览器内核，借的是
系统 WebView2；它多起 3 个助手进程，其中 GPU 进程一项就占 117 MB 私有。Electron
自带的 Chromium 在同样空白页面上只要 133.5 MB。**这笔账记在这里，不淡化。**

Electron 那 356.77 MB 不是虚胖出来的：光 `BaselineElectron.exe` **一个文件**就
238.23 MB，比本项目整个安装目录（5.19 MB）大 46 倍。

这个方向优化不出什么，别指望靠优化图片加载把它降下来——内存大头根本不在我们这边。

两条规则保证这张表不掺水：

- 基线是**最小 Electron 应用**（用 electron-builder 打包的 hello world），不是
  VS Code、Discord 这类完整产品。拿壁纸应用去和代码编辑器比，就是偷换概念。
- 这个 hello world 只开一个**空白窗口**、不加载任何图片。所以 115.57 MB 是
  Electron 的**下限**，不是上限——一个真能用的 Electron 壁纸应用只会更大。
- 一旦测量结果和宣称冲突，**改宣称**。数字是产品，宣传语不是。

## 功能（v0.1.0）

| | 功能 | 状态 |
| --- | --- | --- |
| 1 | 从 JSON 图源拉图 | 🚧 |
| 2 | 缩略图预览 | 🚧 |
| 3 | 一键设壁纸 | 🚧 |
| 4 | 定时切换 | 🚧 |
| 5 | JSON 图源配置 | 🚧 |
| — | 脚手架（Tauri 2 + Vite + TypeScript） | ✅ |

### v0.1.0 明确不做

搜索、收藏、账号、云同步、自动更新、多显示器。小本身就是功能，不在列表里的一律延后。

## 声明式图源

一个图源就是一个 JSON 对象：**怎么问**，以及**怎么读答案**。

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

- 内置定义放在 [`sources/builtin/`](sources/builtin/)，类型契约在
  [`src/lib/sources/schema.ts`](src/lib/sources/schema.ts)。
- `{limit}` 和 `{page}` 是应用唯一会替换的占位符。
- **默认 SFW。** 图源只返回能标记为安全的内容，除非用户显式关闭；默认值永远不是"关闭"。
- 图源配置里永远不放 API Key。需要 Key 的图源不进内置列表。

## 你的数据在哪

- 无账号、无遥测、无统计。
- 缩略图与当前壁纸缓存在应用本地数据目录（`%LOCALAPPDATA%\<应用标识>\`），不进仓库。
- 清缓存是应用内的普通操作，不需要重装。

## 从源码构建

环境要求（括号内为本机实测版本）：

- Windows 10/11 x64（实测 Windows 11 build 26200）
- [Rust](https://rustup.rs/) stable + MSVC 工具链（实测 1.99.1）
- Visual Studio Build Tools（C++ 工作负载）+ Windows SDK（实测 MSVC 14.51 / SDK 10.0.26100）
- Node.js ≥ 20（实测 24.17.0）
- WebView2 运行时（当前 Windows 11 已预装）

```powershell
npm install            # 把前端工具链装进 node_modules/
npm run tauri dev      # 编译 Rust 侧、启动 Vite、打开应用窗口
npm run tauri build    # release 构建；安装包出现在 src-tauri/target/release/bundle/
```

只跑 `npm run build` 只会做 TypeScript 类型检查 + 打包静态前端，**不产生应用**。

## 目录结构

```text
.
├─ src/                         # 前端（原生 TypeScript，Vite 构建）
│  ├─ main.ts                   # 入口：挂载 UI
│  ├─ lib/                      # 与框架无关的逻辑，可单测
│  │  └─ sources/schema.ts      # 图源 JSON 的类型契约
│  ├─ ui/                       # 📋 DOM 渲染与事件绑定
│  └─ assets/                   # 随前端分发的静态资源
├─ sources/builtin/             # 随应用分发的图源定义（只有 JSON，没有图片）
├─ src-tauri/                   # Rust 侧
│  ├─ src/main.rs               # 进程入口，调用库
│  ├─ src/lib.rs                # 应用装配、Tauri Builder
│  ├─ src/commands/             # 📋 暴露给前端的 #[tauri::command]
│  ├─ src/sources/              # 📋 加载与拉取图源
│  ├─ src/wallpaper/            # 📋 设置与缓存壁纸
│  ├─ Cargo.toml
│  └─ tauri.conf.json
├─ docs/BENCHMARK.md            # 体积与速度数字是怎么来的
├─ LICENSE
└─ README.md · README.zh-CN.md
```

标 📋 的目录现在故意不存在：git 不跟踪空目录，目录要靠"第一个真实文件"建立，而不是
先建一个空架子。

## Roadmap

| 版本 | 范围 | 状态 |
| --- | --- | --- |
| `v0.1.0` | 上面五个功能 | 🚧 |
| `v0.2.0` | 桌面挂件 | 📋 |
| `v0.3.0` | 桌面整理 | 📋 |
| `v0.4.0` | 插件式图源（前提：声明式格式真的不够用了） | 📋 |
| 未排期 | CI、自动更新、界面多语言 | 📋 |

范围可以缩小。版本只在上一版真正可用时才发。

## 参与

这是公开写的第一批开源代码，问题与纠错都欢迎，尤其是针对测量方法的——别人复现不了的
数字就是 bug。

请不要提交包含壁纸图片文件的 PR。

## 许可

[MIT](LICENSE) © 2026 Remblued
