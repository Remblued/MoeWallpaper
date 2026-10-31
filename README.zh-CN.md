# MoeWallpaper

**轻量的 Windows 二次元壁纸应用。** Tauri 2 · 原生 TypeScript · 不内置任何图片。

[English](README.md) · [下载 v0.1.2](https://github.com/Remblued/MoeWallpaper/releases/tag/v0.1.2) · [轻量数据怎么测](docs/BENCHMARK.md) · [MIT 许可](LICENSE)

[![CI](https://github.com/Remblued/MoeWallpaper/actions/workflows/ci.yml/badge.svg)](https://github.com/Remblued/MoeWallpaper/actions/workflows/ci.yml)

> **安装包还没有代码签名。** 首次运行可能被 SmartScreen 警告，部分杀软会直接拦。
> 请用发布页上的 SHA-256 校验；签名方案（以及为什么 EV 证书不值得买）写在
> [docs/SIGNING.md](docs/SIGNING.md)。

> **状态：`v0.1.2`。** ✅ 已完成 · 🚧 进行中 · 📋 计划中。只有 [Roadmap](#roadmap) 里的内容才算数。

## 这个项目想解决什么

桌面壁纸应用通常是 Electron 应用。这本身是合理的工程选择，代价是：为了"显示一
格图片、把它设成壁纸"这一件事，用户要先下载 100 MB 以上。

MoeWallpaper 押一个判断：壁纸应用不需要为每个应用内置一整套浏览器运行时。围绕这个
判断有三条承诺：

1. **安装包小。** 实测 **2.04 MB**，不是估算。见
   [体积：测量值，不是宣称值](#体积测量值不是宣称值)。
   （其中 0.23 MB 是 webp/PNG 解码器——为了"设为壁纸"在任何机器上都成立，见下。）
2. **图源是数据，不是代码。** 加一个图源 = 改一个 JSON 文件。没有插件运行时、没有脚本
   层、没有每个图源都要编译的原生模块。
3. **不内置图片。** 仓库里没有一张壁纸。图源指向公开 API，只有缩略图和当前壁纸缓存在
   用户本地。

## 网络要求

**装完打开就能看图，不需要代理。**

实测（2026-10-31，Windows 11，本机 Wi-Fi 直连、系统代理关闭、无 TUN 隧道）：
内置 5 个图源里 **4 个能直连**。

| 图源 | 直连 | 备注 |
|---|---|---|
| 360 壁纸 · 动漫卡通 | ✅ | 排在第一位，打开就是它 |
| Safebooru | ✅ | 动漫插画；服务端按 `width:>=1920` 过滤，带缩略图，支持标签搜索 |
| nekos.best | ✅ | 只提供 neko 分类；**不支持关键词搜索**（它的搜索是模糊排序，会返回不相干的图） |
| Lolicon · Pixiv | ✅ | 图片走 `i.pixiv.re` 镜像（官方 `i.pximg.net` 直连不通）；**界面会标注「尺度较大」** |
| Wallhaven | ❌ 连接超时 | 只有这个需要代理 |

**"能直连"不是永久属性，是出口网络和时刻的属性。** 同一天里 nekos.best 上午还返回
403（按 IP 拉黑）、下午就通了；lolicon 也出现过 403。原因在外网出口，不在代码。
所以上表的意思是"实测于这个网络、这个时刻"，不是"国内任何情况下都可用"。

程序会**自动读取 Windows 的系统代理设置**（「Internet 选项」里那个），不需要在应用里
再配一遍。代理软件开着、系统代理处于开启状态，访问境外图源时才会走它。

三个已知的坑：

- **nekos.best 按 IP 拉黑**。返回 403，正文写着 "Your IP address has been
  automatically blocked by our security system"。共享出口 IP 的节点很容易中招，
  换一个节点即可。
- **只开代理软件、不开系统代理，等于没代理。** 系统代理关着它就直连。
- **360 没有内容分级参数**，只有分类。我们固定用「动漫卡通」（`cid=26`），但它不像
  wallhaven 的 `purity` 或 lolicon 的 `r18` 那样可以用参数保证内容。这条限制写在
  [docs/SOURCES-CN.md](docs/SOURCES-CN.md) 里，没有藏起来。
## 体积：测量值，不是宣称值

下表中每个数字都来自同一台机器、同一套方法、同一天、**release** 构建。`TBD` 表示
*尚未测量*——**目前四格全部测过，没有 `TBD`**。测量方法见
[docs/BENCHMARK.md](docs/BENCHMARK.md)。

| 指标 | MoeWallpaper `v0.1.2` | 最小 Electron 应用 | 怎么测 |
| --- | --- | --- | --- |
| 安装包体积（`.exe`） | **2.04 MB** | **115.57 MB** | 构建产出的安装包文件大小 |
| 安装后磁盘占用 | **5.99 MB** | **356.77 MB** | 安装目录下所有文件大小之和 |
| 冷启动到窗口可见 | **39.5 ms**（中位数） | **104.5 ms**（中位数） | 启动 10 次取中位数，从 CreateProcess 到窗口出现 |
| 空闲内存（启动 60 秒后） | **231.0 MB**（私有，界面无图）<br>**307.7 MB**（私有，20 张图已加载） | **133.5 MB**（私有，空白窗口） | 整个进程树，不是只算主进程；两种界面状态都测 |

安装包体积差 **约 57 倍**（115.57 MB 对 2.04 MB），安装后差 **约 60 倍**，冷启动快
**约 2.6 倍**。

测量环境（见 [BENCHMARK 记录](docs/BENCHMARK.md)）：Intel i7-14650HX / 31.6 GB /
Windows 11 build 26200 / release 构建（LTO + strip）/ `v0.1.2`（含 360 缩略图修复、
webp 转码、真 Acrylic）。

**四个必须一起看的口径说明：**

- **安装包不含 WebView2 运行时。** Windows 11 自带；Windows 10 首次安装会额外下载
  约 100 MB。所以这不是"在任何机器上都只下 2.04 MB"。
- **这 0.23 MB 是故意花的。** 1.81 → 2.04 MB，涨的全部是 webp/PNG 解码器：没装商店的
  webp 扩展时 `SystemParametersInfoW` 认不了 webp，"设为壁纸"会静默失败。我们选择
  "安装包大一点、功能一定可用"，所以 `current.jpg` 现在一定是真 JPEG。
- **冷启动是「窗口出现」，不是「画面渲染完成」。** 后者会再晚几百毫秒。刚装完的
  第一发实测 96 ms（杀软扫描 + 冷读），常态中位数才是 39.5 ms。
- **空闲内存必须说明界面状态。** 图片解码的账主要落在私有内存上：同一个 exe，
  界面里一张图都没有时 231.0 MB（3 次中位，215.1–238.8），加载完默认图源那 20 张图后
  307.7 MB（3 次中位，299.3–313.2）。只报一个数等于把两种完全不同的场景混成一句。

内存报的是**私有内存**，不是工作集（无图 384.2 MB / 有图 405.5 MB）——工作集把多个
进程共享的 DLL 重复计算了。

**这一项我们输了，但要看跟谁比。** Electron 基线是**一个不加载任何图片的空白窗口**，
能和它对齐的只有我们"无图"那一栏：**231.0 MB 对 133.5 MB，多 97.5 MB（1.73 倍）**。
差在进程数上：我们 7 个进程，Electron 4 个，空载时 GPU 进程一个就占 115–137 MB 私有。
**有图的 307.7 MB 不能拿去和空白窗口的 133.5 MB 比**——那等于拿"20 张壁纸"比"什么都不
显示"。Electron 应用显示同样 20 张图也会涨，只是这个最小基线测不了那一面。

Electron 那 356.77 MB 不是虚胖出来的：光 `BaselineElectron.exe` **一个文件**就
238.23 MB，比本项目整个安装目录（5.99 MB）大 40 倍。

**图片的代价已经量出来了，而且降过一次。** 一开始默认图源 360 的 `thumbnailUrl` 映射
到 `url_thumb`，实测它返回的就是 2560×1440 原图（不是缩略图），于是 20 格网格解码 20 张
全尺寸壁纸：有图 465.8 MB、比无图多 267 MB（[SOURCES-CN §2](docs/SOURCES-CN.md)）。改成
接口真正按尺寸产出的 `img_1280_800` 之后，**有图降到 293.7 MB（−172 MB），私有值极差从
187–580 MB 收窄到 274–298 MB**。加了 webp 和 Acrylic 之后是 307.7 MB / 231.0 MB，
差值约 77 MB——那 20 张 1280×800 图本身的代价。再往下就得限制网格张数或后端缩放，
**没做，也没测**。

**内存的摆幅不打算再追。** 同一台机器、同一构建还有 215–314 MB 这个区间，换一台设备
差异只会更大。所以对外一律报**中位数 + 区间**，不承诺精确到个位的数字。详见
[BENCHMARK](docs/BENCHMARK.md) §5。

**上一版（`a65d7ed`）记的 233 MB 已作废**：那次量的是 wallhaven 缩略图的界面状态，
和这一版的默认图源不是一回事，而且从没测过"无图"状态。旧数字保留在
[BENCHMARK 记录](docs/BENCHMARK.md) 里，不删，但不要和新数字混着引用。

两条规则保证这张表不掺水：

- 基线是**最小 Electron 应用**（用 electron-builder 打包的 hello world），不是
  VS Code、Discord 这类完整产品。拿壁纸应用去和代码编辑器比，就是偷换概念。
- 这个 hello world 只开一个**空白窗口**、不加载任何图片。所以 115.57 MB 是
  Electron 的**下限**，不是上限——一个真能用的 Electron 壁纸应用只会更大。
- 一旦测量结果和宣称冲突，**改宣称**。数字是产品，宣传语不是。

## 功能（v0.1.2）

| 功能 | 状态 |
| --- | --- |
| 从 JSON 图源拉图（360 / safebooru / wallhaven / nekos.best / lolicon） | ✅ |
| 缩略图网格 + 骨架屏 + 加载失败占位 | ✅ |
| 图片详情页：大图、设为壁纸、下载、打开原网页 | ✅ |
| 关键词搜索（中文词表 + 各图源自己的搜索端点） | ✅ |
| 翻页 | ✅ |
| 分辨率筛选（≥1080p / ≥1440p / ≥4K） | ✅ |
| 深色 / 浅色 / 玻璃三种主题，记住选择；**玻璃主题是系统 Acrylic，桌面真的透上来** | ✅ |
| 一键设壁纸（Win32 `SystemParametersInfo`） | ✅ |
| **JPEG / PNG / WebP / GIF 都能设成壁纸**（webp 先转码成 JPEG，不依赖商店扩展） | ✅ |
| 存到「图片」目录，重名不覆盖 | ✅ |
| 本地缓存（缩略图 + 当前壁纸） | ✅ |
| 声明式 JSON 图源 | ✅ |
| 国内直连图源 | ✅ 360 壁纸（默认图源） |
| CI（fmt / clippy / test / tsc / vite，GitHub Actions） | ✅ |
| 定时切换 | ❌ 明确不做，见下 |

### v0.1.0 明确不做

收藏、账号、云同步、自动更新、多显示器、定时切换。小本身就是功能，不在列表里的一律延后。

**定时切换是主动砍掉的**，不是没来得及：它需要一个常驻定时器和"切到下一张"的状态机，
而关键词搜索能用更少的代码解决同一个需求——"我想看这个主题的图"。少一个功能，少一堆
要维护的边界情况。

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
| `v0.1.1` | **国内直连图源**（360 壁纸动漫分类）+ 偏移量分页、合并尺寸字段、图片 https 升级 | ✅ |
| `v0.1.2` | 玻璃主题**跟随系统亮/暗**（半透明，不做全透明）；新增 safebooru 图源、lolicon 内容标注 | ✅ |
| `v0.2.0` | **桌面挂件** + **允许用户接入自己的 API**（`v0.1.x` 只做内置图源，这是决定） | 📋 |
| `v0.3.0` | 桌面挂件 | 📋 |
| `v0.4.0` | 插件式图源（前提：声明式格式真的不够用了） | 📋 |
| 未排期 | CI、自动更新、界面多语言、详情页焦点陷阱 | 📋 |

`v0.1.1` 就是原来计划放在 `v0.2.0` 里的国内源：没有它，应用在国内是"装完打开是空的"，
比任何新功能都紧急。`v0.2.0` 因此让位给两块新东西——桌面挂件，以及**让用户自己接 API**
（不用改代码、不用等我们加源）。

`v0.1.0` 还没正式发过 tag，`v0.1.1` 是第一个真正能用的版本。

范围可以缩小。版本只在上一版真正可用时才发。

## 参与

这是公开写的第一批开源代码，问题与纠错都欢迎，尤其是针对测量方法的——别人复现不了的
数字就是 bug。

请不要提交包含壁纸图片文件的 PR。

## 许可

[MIT](LICENSE) © 2026 Remblued
