# MoeWallpaper 交接文档

> 给新加入的合作者。读完这份文档，你应该能在 15 分钟内跑起项目，并知道下一步该做什么、什么不能做。
>
> 最后更新：2026-10-29 · 分支 `main`。
> **具体状态以 `git log -1` 为准，本文不追提交号**——写死在文档里的哈希一定会过期，然后误导下一个读它的人。

---

## 0. 一页速览

- **是什么**：Windows 桌面壁纸应用（二次元向），Tauri 2 + 原生 TypeScript。
- **不是什么**：不是 Electron 项目，不用 React/Vue，不支持 macOS/Linux。
- **三个卖点**（写进 README 的，别丢）：安装包 **1.81 MB**（实测，不是"5–10 MB 级别"
  这种模糊说法）；图源用 JSON 声明、不做插件运行时；不内置任何图片。
- **现在**：能联网拉图、能下载缓存、能一键换桌面壁纸；界面有图源下拉 + 缩略图网格 +
  详情页（点缩略图看大图，页面上「设为壁纸」/「下载」）+ 搜索 + 翻页 + 分辨率筛选 +
  三种主题，其中玻璃主题是**真 Acrylic**（桌面透上来，亮/暗两套已用整屏截图验过）。
  壁纸格式支持 JPEG / PNG / WebP / GIF（webp 先转码成 JPEG，不依赖商店扩展）。
- **范围（2026-10-31 定的）**：`v0.1.x` **只做内置图源**；让用户自己接图源放到 `v0.2.0`；
  **桌面挂件挪到 `v0.3.0`**（挂件不产出内容，而且要先有稳定的图源格式——理由写在
  README 的 Roadmap 下面）。
  所以 `sfwQuery` 强制校验、图源格式向后兼容这些"自定义图源前置条件"**不阻塞现在的发布**
- **签名**：现在**不签**（仓库已于 2026-10-31 转 public，但签名只对公开分发有意义，
  而公开分发的第一个版本还没发）。方案与命令见 `docs/SIGNING.md`。发布页必须写
  SHA-256，这条不因为有没有签名而变
- **代码量**：约 90 个受管文件，Rust 约 2,440 行、前端 TS 约 690 行（都含注释和测试）。
  **你读得完。**

### 交互铁律（已经定死，别自己改）

> **点缩略图 → 打开详情页看大图，页面上有「下载」和「设为壁纸」两个按钮。**

**绝不点一下就换掉桌面。** 壁纸应用里，用户点一张缩略图九成是想"看看这张"，不是"我现在就要它当桌面"。点一下就把桌面换掉，是反人类设计。

这条优先级高于任何"简洁""少点一下""体验流畅"的说法。

---

## 1. 项目现在到哪儿了

### 已完成（有测试、有提交）

| 模块 | 内容 | 位置 |
|---|---|---|
| 仓库基础设施 | LICENSE(MIT)、README(中英)、PR 模板、.gitattributes、.editorconfig | 根目录 |
| 缓存路径 | 路径函数，全部从 `cache_dir()` 派生 + `current.jpg` 固定落点 | `src-tauri/src/fs/paths.rs` |
| 目录创建 | `ensure_dir` / `ensure_base_layout`（启动时调用） | `src-tauri/src/fs/dirs.rs` |
| 原子写文件 | 写 .tmp → fsync → rename | `src-tauri/src/fs/files.rs` |
| URL → 文件名 | FNV-1a 哈希 + 扩展名猜测 | `src-tauri/src/fs/naming.rs` |
| 图源配置解析 | JSON → Rust 结构体（serde） | `src-tauri/src/sources/model.rs` + `loader.rs` |
| 响应提取 | 点号路径取值 + 按映射提取图片列表 | `src-tauri/src/sources/extract.rs` |
| 联网拉图 | 共享连接池、`{page}`/`{pageIndex}`/`{offset}`/`{limit}` 替换、只放行 GET | `src-tauri/src/sources/fetch.rs` |
| 出参转换 | `ImageRef → ImageDto`，缩略图回退 | `src-tauri/src/sources/dto.rs` |
| 下载缓存 | 命中就不联网，原子写入 | `src-tauri/src/wallpaper/download.rs` |
| 设壁纸 | Win32 `SystemParametersInfoW` 手工 FFI | `src-tauri/src/wallpaper/set.rs` |
| 当前壁纸 | 规整成 JPEG 后写到固定的 `current.jpg`，与缓存解耦 | `src-tauri/src/wallpaper/current.rs` |
| **格式兼容** | webp / PNG / GIF → JPEG（`image` crate，只开 4 个解码器）。已修掉"current.jpg 里装着 webp/PNG"的债 | `src-tauri/src/wallpaper/normalize.rs` |
| **窗口背景材质** | 真 Acrylic：`transparent: true` + `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE)`，跟随系统亮暗 | `src-tauri/src/backdrop.rs` + `src/styles.css` |
| 四个命令 | `list_sources` / `fetch_images` / `set_wallpaper` / `save_image` | `src-tauri/src/commands/mod.rs` |
| 真实图源 | 360 / safebooru / wallhaven / nekos.best / lolicon，全部免费、无需 API key | `sources/builtin/sources.json` |
| 内容提示 | `contentNote`：图源名旁标出"尺度较大"（目前只有 lolicon）。**是提示，不是过滤器** | `sources/builtin/sources.json` + `src/main.ts::syncSourceUi` |
| 关键词搜索 | 每个图源可声明自己的搜索地址；中文走本地词表翻成英文标签 | `src-tauri/src/sources/search.rs` + `search/terms.json` |
| 前端 | 图源下拉、缩略图网格、四态、toast | `src/main.ts` + `src/lib/api.ts` + `src/lib/mock.ts` |
| **CI** | GitHub Actions：Rust（fmt/clippy/test）+ 前端（tsc/vite），见 §2.5 | `.github/workflows/ci.yml` |

**测试：102 个（96 个自动跑，6 个标记 `#[ignore]` 手动跑——它们会真的改桌面或真的联网）。`cargo clippy -D warnings` 零告警。**

### 未做（这就是接下来的活）

- ✅ 玻璃主题（v0.1.1 起跟随系统亮/暗）；**v0.1.2 末升级成真 Acrylic**：
  `transparent: true` + `DwmSetWindowAttribute(38)`，桌面/后窗真的透上来并被模糊
- ✅ 国内图源：360 壁纸（v0.1.1）；2026-10-31 又接上 safebooru，并把 lolicon 的图片域名钉到
  `i.pixiv.re`。**5 个图源实测 4 个能直连**，只有 wallhaven 不行。依据在 `docs/SOURCES-CN.md` §9–§10
- ✅ 五个图源都能"换一批"：360 用 `randomOffsetMax`、safebooru 用 `randomPageIndexMax`，
  其余三个接口本身就是随机的。**接新图源必须先回答"第 1 页是不是永远同一批"**——
  见 `docs/SOURCES-CN.md` §0.1 第 1 条
- ✅ webp：**产品决定"不为了体积砍体验"**，加了 `image` 解码器并把 `current.jpg` 统一转成
  JPEG（安装包 +0.23 MB，已实测）。夜轻图源仍不接，卡在"纯文本行 + 无缩略图"
- ✅ 发布：`v0.1.2` tag + GitHub Release（带 NSIS 安装包）
  https://github.com/Remblued/MoeWallpaper/releases/tag/v0.1.2
- ❌ 定时切换
- ✅ 仓库已公开（2026-10-31）。转 public 前逐条查过：历史里**没有**密钥/凭据类字符串；
  README 的下载链接与 CI 徽章匿名可访问（都 200）；Release 资产**匿名可下载**、字节哈希与
  本地被测文件一致；LICENSE、PR 模板、`.gitignore` 都在。**唯一没洗的**：`d0d6373` 那次
  提交里还留着 `bench/make_icon.py` 的旧个人路径（HEAD 已清，历史没重写，见债表）
- ⚠️ Electron 基线已经全部测完（安装包 115.57 MB / 安装后 356.77 MB / 冷启动
  104.5 ms / 空闲 133.5 MB 私有）。MoeWallpaper `v0.1.2` 的最新一组：
  **2.04 MB / 5.99 MB / 39.5 ms / 231.0 MB 无图、307.7 MB 有图**。**空闲内存这一项我们
  仍然是输的，而且要看跟哪个状态比**：和空白窗口对齐只能拿"无图"那格（231.0 对 133.5，
  多 97.5 MB）；有图那格不能拿去和空白窗口比。别在外面只说"内存 307.7 MB"。
  内存一律报**中位 + 区间**，摆幅不再追查（换设备差异更大）。
  详见 `docs/BENCHMARK.md` §5

---

## 2. 跑起来

### 环境

Windows 11 + Rust stable(MSVC) + Node ≥20 + Visual Studio C++ 工作负载 + Windows SDK。

### 命令（注意两个目录）

```powershell
# 装依赖（只需一次）
cd D:\meowallpaper\Meowallpaper
npm install

# 开发：编译 Rust + 起前端服务 + 开窗口
npm run tauri dev

# 后端的三道门（在 src-tauri 里跑）
cd src-tauri
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test

# 前端的类型检查（在仓库根跑）
cd ..
npx tsc --noEmit
```

### 三个目录的区别（新手最容易搞混）

| 命令 | 必须在哪跑 | 原因 |
|---|---|---|
| `cargo *` | `src-tauri\` | cargo 靠 `Cargo.toml` 找项目，只往上级找 |
| `npm *` / `npx *` | 仓库根 | 靠 `package.json` |
| `git *` | 哪都行 | 靠 `.git`，会往上找 |

### 2.5 CI（`.github/workflows/ci.yml`）

**CI 是什么**：仓库里放一个 YAML，GitHub 每次 push / PR 就开一台干净的虚拟机，把你写在
里面的命令跑一遍。红了就在 PR 上标出来——**它替你做的是"别人不用先本地配环境就能相信这
个提交能过"**，不是"帮你测体积"。

两个 job，各跑各的：

| job | 跑什么 | 为什么放 `windows-latest` |
|---|---|---|
| `rust` | `cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` / `cargo test` | 代码里有 `user32` / `dwmapi` / `advapi32` 的 FFI 和 `#[cfg(windows)]`，Linux 上编不过 |
| `web` | `npm ci` / `npx tsc --noEmit` / `npx vite build` | 纯前端，放 `ubuntu-latest` 更便宜也更快 |

几个有意为之的决定：

- **CI 里不 `tauri build`。** release 构建带 LTO + `codegen-units = 1`，一次几分钟，
  而且装出来的包在 CI 里没人用。**体积/冷启动/内存必须真机测**（`docs/BENCHMARK.md` §1
  要求固定机器、固定防护状态），CI 只保证"编得过、测试过、lint 干净、类型对"。
- **`npm ci` 而不是 `npm install`**：锁文件说什么装什么，CI 里不允许悄悄升级依赖。
- **`concurrency` 会取消同一分支的上一个 run**：连续 push 时省额度。
- `#[ignore]` 的 6 个测试（联网 / 改桌面 / 转码）**CI 不跑**，它们本来就是手动门。
- 仓库转 public 之后 Actions 对标准 runner **免费**（之前 private 时 Windows runner 按 2 倍
  计费）。工作流本身一次 3–5 分钟。

**本地等价命令**就是上面那四条（fmt / clippy / test / tsc）；`npx vite build` 是打包
前那次构建，顺手也跑一下。

**第一次真实运行（2026-10-31，`f4d4ad2`）**：两个 job 全绿——前端 16 秒，Rust 冷缓存
约 12 分钟（`Swatinem/rust-cache` 起了作用之后会快很多）。这一步的意义是：CI 红了就只
可能是这三道门本身，不会再出现"本地能过、CI 不能过"的模糊地带。

### 2.6 发布构建（`.github/workflows/release.yml`，手动触发）

**为什么另有一个工作流**：SignPath 只签"由 GitHub 工作流构建、并且先作为 workflow
artifact 存在过"的产物，本地 `npm run tauri build` 的 exe 它不签。所以"要签名"这件事反
过来要求"发布包也能在 CI 里构建出来"。

它做五件事：按 tag 检出 → `npm ci` + `npx tauri build --bundles nsis` → 把安装包上传为
artifact →（若配置了 SignPath 的 vars）提交签名并取回签名产物 → 算 SHA-256 并 `gh release
upload --clobber` 附到同名 Release。**不自动触发**，只在 Actions 页面手动 Run workflow
（或 `gh workflow run release.yml -f tag=v0.1.2`）。

签名那一步用 `if: vars.SIGNPATH_PROJECT_SLUG != ''` 挡着：没配变量就只构建、只附件，不会
因为缺 token 而红。申请与接入清单见 `docs/SIGNPATH.md`。

**第一次真实运行（2026-10-31，release run #1）**：全绿。步骤顺序与设计一致，签名那一步
`skipped`（没配变量），`Attach to release` 也按参数跳过。产物：

```text
unsigned-installer artifact  2,115,812 B   （安装包 ZIP）
installer artifact           2,116,077 B   （安装包 + .sha256）
解开的安装包                 2,134,310 B   sha256 CBF2ADFE488AC11E3B228FD9A8910FE02AD24403A6E2DF9F2574175AA3268ADA
```

**⚠️ CI 构建的安装包和本地构建的不是同一串字节。** 本地那一版是 2,135,537 B /
`2440651A…`（发布页和 BENCHMARK 记的就是它），CI 这一版是 2,134,310 B / `CBF2ADFE…`。
差异来自 rustc / NSIS 版本与时间戳，不是代码不同。**结论：哈希必须跟着"哪一次构建"走**，
换构建环境就要重新记录；签名会再改一次字节，所以签名发布之后三处哈希（README、
BENCHMARK、Release 说明）都要更新。

### 2.7 main 的保护（规则集，不是经典分支保护）

2026-10-31 起 `main` 上有一条名为 `main-protection` 的**规则集**：

| 规则 | 效果 |
| --- | --- |
| `deletion` | 不能删 `main` |
| `non_fast_forward` | **不能 force-push**（真正要防的那件事） |
| `required_status_checks` | PR 合并前必须过 CI 的两个 check（`Rust (fmt / clippy / test)`、`Frontend (tsc / vite build)`），不要求分支最新 |
| `bypass_actors` | 维护者（`Remblued`，actor id 174602037）可直推 |

**为什么不用经典分支保护**（踩过一次）：经典保护里勾了 "Require status checks" 之后，
**连直接 push 也会被拒**——报 `GH006: Protected branch update failed` /
"2 of 2 required status checks are expected"，因为新提交还没有任何 check run；而
`enforce_admins = true` 又让 owner 绕不过去。对单人项目来说那等于"每次改文档都要开 PR"。
规则集支持 `bypass_actors`：规则对别人照样生效，维护者能直推。

改设置的逻辑（可复现）：建规则集 → 删掉经典保护。**别只建不删**：两套同时存在时，经典保护
仍然会拦 push。规则集的 API 是 `POST /repos/{owner}/{repo}/rulesets`，经典保护是
`DELETE /repos/{owner}/{repo}/branches/main/protection`。

---

## 3. 代码地图

```
Meowallpaper/
├─ index.html                  ← 窗口骨架：只有 #source-select / #status / #grid / #toast 四个锚点
├─ src/                        ← 前端（原生 TS）
│  ├─ main.ts                  ← 入口：拉数据 → 渲染网格 → 点击处理
│  ├─ styles.css               ← 全部样式
│  └─ lib/
│     ├─ api.ts                ← 全项目唯一出现 invoke( 的地方
│     ├─ mock.ts               ← 假数据 + USE_MOCK 开关
│     └─ sources/schema.ts     ← 图源配置的类型（暂时没人 import，别删）
├─ sources/
│  ├─ builtin/sources.json             ← 随应用分发的图源（include_str! 编进 exe）
│  └─ examples/                        ← 不打包：示例配置模板 + 假 API 响应
├─ src-tauri/src/
│  ├─ lib.rs                   ← 组装：注册命令 + 启动时建缓存目录
│  ├─ commands/mod.rs          ← 门面层：只有 #[tauri::command]，不做逻辑
│  ├─ fs/                      ← paths(路径) / dirs(建目录) / files(原子写) / naming(URL→文件名)
│  ├─ sources/                 ← model(结构) / loader(加载) / extract(提取) / fetch(联网) / dto(出参)
│  └─ wallpaper/               ← download(下载缓存) / current(当前壁纸) / set(调 Win32)
├─ .github/
│  ├─ pull_request_template.md ← 合并前的检查清单
│  └─ workflows/               ← ci.yml（三道门 + 前端构建）/ release.yml（手动，打包 + 可选签名）
├─ DOWNLOAD.md                 ← 下载页：也是 SignPath 申请要求的"写明签名由 SignPath 提供"的那一页
├─ PRIVACY.md                  ← 隐私政策：不收集任何数据，写清本地存什么、请求发给谁
└─ docs/
   ├─ BENCHMARK.md             ← 四个数字怎么测、原始值、踩过的坑
   ├─ SIGNING.md               ← 代码签名：五种方案的资格与价格、命令、本项目的决定
   ├─ SIGNPATH.md              ← SignPath 申请：表单字段逐格填法、Reputation 怎么写、批准后 5 步
   ├─ SOURCES-CN.md            ← 图源调研与实测（§0.1 是接入验收清单）
   └─ HANDOVER.md              ← 本文件
```

### 一条铁律：门面与实现分离

`commands/` 里**只做三件事**：收参数 → 调实现 → 转错误。
真正的逻辑放在对应业务目录（`sources/`、`wallpaper/`、`fs/`）。

理由：命令层每加一个函数都要改 `lib.rs` 的注册表，改动频率高；实现层会反复重构。**两者的变更节奏不同，就该分开。**

---

## 4. 命令契约（双方并行开发的依据）

**这是最重要的部分。** 前端和后端各自按这张表写，不用等对方。

### 已实现（四个命令，前端都接得上）

```rust
list_sources() -> Result<Vec<WallpaperSource>, String>

// 拉一页图。page 从 1 开始。query 不传 = 浏览，传了 = 搜索
fetch_images(source_id: String, page: u32, query: Option<String>) -> Result<Vec<ImageDto>, String>

// 下载 → 缓存 → 复制到 current.jpg → 设成桌面壁纸（一步到位）
set_wallpaper(image_url: String) -> Result<(), String>

// 下载 → 缓存 → 存到用户的「图片」目录，返回绝对路径
save_image(image_url: String) -> Result<String, String>
```

### `save_image` 的行为约定（已实现，别改这些）

- 目标目录：`%USERPROFILE%\Pictures\MoeWallpaper\`
- **重名不覆盖**：`xxx.png` 已存在就存成 `xxx (1).png`
- 返回值是绝对路径，前端可以直接提示「已保存到 …」
- 先走缓存：刚设过壁纸的图，保存是瞬间完成的，不会重新下载

**为什么不能复用 `set_wallpaper` 里的下载**：缓存会被清、会被覆盖，而用户"下载"下来的东西必须放在他自己的目录里、由他自己支配。**这两个是完全不同的语义，共用一个函数迟早出事。**

**详情页不需要新的后端命令。** `ImageDto.imageUrl` 就已经是原图地址，`<img src={imageUrl}>` 直接显示即可——大图不需要经过 Rust。

### 搜索（已实现）

只有配了 `search` 块的图源才支持搜索。还是同一个命令，多传一个 `query`：

```ts
invoke("fetch_images", { sourceId, page: 1 })                    // 浏览
invoke("fetch_images", { sourceId, page: 1, query: "风景" })      // 搜索
```

后端做三件事：

1. **选模板**：有关键词且图源支持搜索 → 用 `search.url`，否则用 `request.url`
2. **翻译**：`search.english === true` 时查 `search/terms.json`（中文 → 英文标签）。
   查不到就**原样发出去**，不猜、不拆词、不音译
3. **编码**：交给 `Url::parse`，中文关键词自动百分号编码

图源的**实测**搜索行为（不是查文档）：

| 图源 | 参数 | 中文 | 英文 | 日文 | 备注 |
|---|---|---|---|---|---|
| wallhaven | `q` | ❌ 返回 0 条（所以要翻译） | ✅ | ✅ | |
| lolicon | `keyword` | ✅ 直接用（不翻译） | ✅ | ✅ | |
| safebooru | `tags=` | ✅ 翻成英文再转下划线 | ✅ | ❌ | `underscoreTags: true`，`hatsune miku` 0 条 / `hatsune_miku` 20 条 |
| 360 | 无 | — | — | — | 接口没有搜索 |
| nekos.best | 无 | — | — | — | **搜索已摘除**，见 `docs/SOURCES-CN.md` §9.5 |

nekos.best 为什么没有：它的 `/search` 是相关性排序、永不为空
（`zzzznotexist` 也返回 5 张），图片结果只有画师名可比对。
再配下去就是"配了但静默不准"，所以按 `model.rs` 的规矩直接不配。

**所以前端的提示语很关键**：搜不到结果时不要写"没有结果"，要写
**"试试英文关键词，或换个图源"**——因为很可能只是这个词没被词表覆盖。

### `ImageDto` —— 前端拿到的形状（**已定，别改**）

```rust
pub struct ImageDto {
    pub image_url: String,            // 原图地址，设壁纸用
    pub thumbnail_url: String,        // 缩略图；后端保证一定有（缺了就回退成原图）
    pub page_url: Option<String>,     // 作品页，用于标注来源
}
```

对应 TypeScript：

```ts
interface ImageDto {
  imageUrl: string;
  thumbnailUrl: string;
  pageUrl: string | null;
}
```

**为什么 `thumbnail_url` 是 `String` 而不是 `Option<String>`**：回退逻辑（没有缩略图就用原图）只该写一次，写在后端。如果前端拿到 `Option`，每个用到它的地方都得处理一次 `null`。

**这是通用原则：能在一处规整好的数据，不要留成"每个使用者都要处理"的形状。**

### SFW 是怎么保证的（卖点之一，别弄丢）

**不是靠一个 `sfwOnly: true` 的布尔开关**——那种字段只要没有代码消费它，就是口号。

真正的机制是 `filter.sfwQuery`：配置里写裸参数（`purity=100`、`r18=0`），
`fetch.rs::apply_sfw_query` 在发请求前拼到地址末尾。

| 图源 | 安全来自哪 |
|---|---|
| wallhaven | `sfwQuery: "purity=100"` |
| lolicon | `sfwQuery: "r18=0"` + 图片域名钉在 `i.pixiv.re` + `contentNote: "尺度较大"`（**提示，不是保护**） |
| nekos.best | API 本身只提供安全内容，不配 |
| 360 | 只有分类没有分级，固定 `cid=26 动漫卡通`——**这是本项目的短板，不是保护** |
| safebooru | 分级写进了 URL 的 `tags=rating:general`，**没有走 `sfwQuery`** |

**所以配图源时的规矩是：只要那个 API 有 NSFW 参数，`sfwQuery` 就必须写。**
写了才受保护，忘了就没有——这一点只能靠 review 挡，测试挡不住。

**`sfwQuery` 的一个已知局限（safebooru 就是受害者）**：它是**静态追加**的，
而 booru 类接口要求所有条件都塞进**同一个 `tags` 参数**。如果 browse 用
`sfwQuery: "tags=rating:general"`、search 模板里再写 `tags={query}`，
最终地址里会出现两个 `tags`，PHP 取**最后一个**——搜索词被静默丢掉，
界面表现是"搜什么都回到首页那批图"。
所以这一类图源只能把分级写进 URL 模板（安全参数仍在，只是不在 `sfwQuery` 这个格子里）。
要根治得让格式支持"安全条件与搜索词合并"，属于改格式，不是改 JSON。

---

## 5. 我们定的十条规矩

违反任何一条都会在 review 时被打回。

1. **一个提交只做一件事。** 主题是"加缩略图网格"，就只改前端那三个文件；顺手改的 README 单独一个提交。
2. **提交信息用 Conventional Commits**：`feat:` 新功能 / `fix:` 修 bug / `docs:` / `refactor:` / `test:` / `chore:`。标题不超过 72 字符，正文说"为什么"。
3. **提交前三道门全绿**：`cargo fmt`、`cargo clippy --all-targets -- -D warnings`、`cargo test`。前端加 `npx tsc --noEmit`。
4. **新功能必须有测试**，至少覆盖"正常"和"一种出错"。
5. **永不提交**：`target/`、`node_modules/`、`dist/`、`.env`、任何**壁纸或截图**。
   例外：应用图标（`src-tauri/icons/`、`app-icon.png`）必须提交——它们属于代码资产，
   不是内容。
6. **路径只在 `fs/paths.rs` 里拼**。别处一律调那里的函数。
7. **命令放 `commands/`，实现放业务目录。**
8. **错误信息必须带上下文**：哪个路径、哪个字段、哪个操作。`format!("创建目录失败 {:?}: {}", path, e)`。
9. **跨语言字段命名**：JSON/TS 用 camelCase，Rust 用 snake_case，靠 `#[serde(rename_all = "camelCase")]` 转换。**别在两边统一成同一种拼法。**
10. **共享文件只有 A 能改**：`README.md`、`Cargo.toml`、`package.json`、`docs/`、`src-tauri/src/lib.rs`。B 要改，先说一声。

---

## 6. 分工

### 原则：按目录切，不按功能切

两个人同时改同一个文件 = 每天解决冲突。所以边界划在**目录**上：

| | A（Remblued） | B（新同学） |
|---|---|---|
| 负责目录 | `src-tauri/**`、`sources/**` | `src/**`、`index.html` |
| 主要语言 | Rust | TypeScript / CSS |
| 共享文件 | 只有 A 能改 | 只读 |

**好处**：B 完全不需要碰 Rust，A 不需要碰 CSS。冲突面接近于零。

### A 的任务

| # | 任务 | 状态 |
|---|---|---|
| A1 | `fetch_source_images`：联网拉图 | ✅ |
| A2 | `ImageRef → ImageDto`（缩略图回退） | ✅ |
| A3 | `set_wallpaper`：Win32 FFI | ✅ |
| A4 | `ensure_cached`：下载 + 缓存 | ✅ |
| **A5** | `save_image`：存到用户的图片目录 | ✅ |
| **A6** | 接真实图源：wallhaven / nekos.best / lolicon | ✅ |
| **A7** | 搜索后端：`search` 配置 + `search/terms.json` 中文词表 | ✅ |
| **A8** | 真·窗口背景材质（`transparent` + `DwmSetWindowAttribute`，见 B8） | ✅ `src-tauri/src/backdrop.rs` |
| ~~A9~~ | ~~定时切换~~ | **砍掉**，不作为卖点 |

### B 的任务

| # | 任务 | 状态 |
|---|---|---|
| B1 | `src/lib/api.ts` 薄封装 | ✅ |
| B2 | `src/lib/mock.ts` 假数据 | ✅ |
| B3 | 缩略图网格 | ✅ |
| B4 | 四态：加载中 / 空 / 出错 | ✅ |
| B5 | 点击 → 调 `set_wallpaper` | ✅ **已被 B6 取代**：点击现在打开详情页，不再直接换桌面 |
| **B6** | **图片详情页** | ✅ `5790aad` |
| **B7** | 主题色 | ⬜ |
| **B8** | 半透明：先 CSS 后真 Acrylic | ✅ 真 Acrylic 已上窗，亮/暗都截图验过 |
| **B9** | 搜索框 | ✅ `5790aad` |

**B6 / B9 的合并记录（2026-10-30，`1a89bfb`）**

B 这次是用 `git push` 交的分支 `feat/b-batch-oct30` + PR，不是网页上传，照规矩做的。
合并前逐行看过，规格里的硬要求都在：三个关闭方式（× / Esc / 点遮罩）、加载失败仍可
关闭、`enabled !== false` 过滤、`query` 可选不破坏旧调用、离线 data-URI mock。

审出一个缺陷并已单独修（`ec6f43c`）：`handleSetWallpaper` 只在 catch 里恢复按钮，
成功后会永远停在「设置中…」的禁用态。`handleDownload` 原本就用 `finally` 写对了，
所以改成一致而不是再加一种写法。

### B9 搜索框

后端已经能搜了，只差界面。

- 图源下拉旁边加一个搜索框 + 搜索按钮（回车也能触发）
- **当前图源不支持搜索时**：搜索框禁用，`placeholder` 写"该图源不支持搜索"
  （判断依据：`WallpaperSource` 上有没有 `search` 字段）
- 搜索时 `page` 固定 1；清空搜索框 = 回到浏览模式
- 搜不到结果时，空状态文案写 **"没搜到，试试英文关键词或换个图源"**——
  不是"没有结果"，因为很可能只是这个词没被中文词表覆盖
- 搜索中要有加载态（复用现有骨架屏）

`sourceId` 要同步给 `api.ts`：

```ts
export async function fetchImages(
  sourceId: string,
  page: number,
  query?: string,          // ← 新增，不传 = 浏览
): Promise<ImageDto[]> {
  return invoke<ImageDto[]>("fetch_images", { sourceId, page, query });
}
```

**注意**：`query` 是可选参数，不传时 Tauri 会把它当成 `None`，后端按浏览处理。所以这个改动**不会破坏现有调用**。

### B6 图片详情页（规格）

现在点缩略图直接设壁纸，**这个行为必须改掉**（见 §0 交互铁律）。

- 点缩略图 → 在**当前窗口内**打开覆盖层（不新开窗口，也不用原生对话框）
- 覆盖层里显示 `imageUrl` 大图：等比缩放，**不裁切、不变形**
- 两个主按钮：**「设为壁纸」**（主操作）、**「下载」**
- 三个关闭方式，缺一不可：右上角 × 、按 `Esc` 、点遮罩空白处
- 大图加载中要有提示；加载失败要在覆盖层里显示错误，且仍能关闭（否则用户被卡死）
- 键盘可达：两个按钮能 Tab 到、`Enter` 触发
- 设壁纸时按钮禁用 + 文案变「设置中…」，结果用现有 toast
- 「下载」接 A5 的 `save_image`，成功后提示「已保存到 <路径>」

### B7 主题色

- 所有颜色改成 CSS 变量（`--accent`、`--bg`、`--panel` …），**规则里不许再出现写死的色值**
- 一个色板选择器，选完存 `localStorage`，下次启动读回来
- 纯前端，不用碰 Rust

### B8 半透明——**分两步，别一步到位**

1. **先做 CSS 半透明**：`background: rgba(...)` + `backdrop-filter: blur()`。跨系统一致，零风险，视觉效果已经够。
2. 真·窗口透明要改 `tauri.conf.json` 的 `window.transparent`，那是 A 的活（A8），而且**不同 Windows 版本表现不一致**，还要处理 webview 背景。

**别一上来就动窗口属性。** 先用 CSS 把"看起来是半透明"做出来，等主题系统稳定了再考虑真的透明。

**B 为什么能不等 A**：§4 的契约已经定了，缺的命令照着签名和 mock 写就行。等 A 实现了，把 `USE_MOCK` 关掉就接上。

---

## 7. 已知的技术债（知道就好，先别动）

| 债 | 位置 | 什么时候还 |
|---|---|---|
| **缓存会记住错误**：下载到一张 HTML 错误页，也会被永久当成"这张图"，不会自愈（**现在至少会报错**：`normalize` 认出"不是能识别的图片"，不会把错误页设成壁纸） | `wallpaper/download.rs` | 加「清空缓存」按钮；临时解法：删掉 `%LOCALAPPDATA%\com.remblued.moewallpaper` |
| ~~`current.jpg` 里可能装着 PNG / WebP 数据，靠 Windows 按内容识别格式~~ | `wallpaper/current.rs` + `normalize.rs` | **已还**：落盘前统一转成 JPEG（已是 JPEG 的原样透传），解不出来就报错。见 `docs/SOURCES-CN.md` §7.3 |
| 安全参数靠"记得配"——图源没写 `sfwQuery` 就没有任何内容限制，而这一点只有 code review 能保证 | `sources/builtin/sources.json` | **`v0.2.0` 做用户自接图源时**必须在加载阶段强制校验：要么配 `sfwQuery`，要么显式标记"API 本身只提供安全内容"。这条也是"自定义图源为什么排在挂件前面"的原因之一 |
| 中文词表只收了约 150 个高频词，长尾词翻不了（`search/terms.json`） | `search/terms.json` | 持续加词；搜不到时 UI 已提示改用英文 |
| 英文译名对不上目标站标签就会搜不到（映射正确 ≠ 标签存在） | `search/terms.json` | 加词时**实测一次**：拿翻出来的英文去 wallhaven 搜，看条数是不是 0 |
| 国内免费接口几乎都是"单图 302 / 直接返回一张图"，不是列表；本项目 schema 只吃列表 | `sources/model.rs` | 要支持单图源，得加一种 source 类型（v0.4 插件化时一起做）。**2026-10-31 实测清单见 `docs/SOURCES-CN.md` §7、§9** |
| 360 壁纸有动漫分类（`cid=26`），但**不是公开 API**，条款不明，而且 `start` 是偏移量不是页码 | — | 已在 v0.1.1 接入并排第一；条款与风险见 `docs/SOURCES-CN.md` §5 |
| nekos.best 和 lolicon 不提供缩略图，网格会直接加载原图（一张 2–9MB） | `sources/builtin/sources.json` | 要么限制每页数量，要么后端生成缩略图。**safebooru 有 `preview_url`（7.5 KB），说明这不是"没法做"，是那几个接口不给** |
| ~~**360 的 `url_thumb` 也是原图**：实测 1920×1080 / 2560×1440，不是缩略图~~ | `sources/builtin/sources.json`（360 的 `thumbnailUrl`） | **已还**：改成接口真正按尺寸产出的 `img_1280_800`（`docs/SOURCES-CN.md` §2 有整组字段的实测表）。有图空闲内存 465.8 → 293.7 MB。两条测试盯着它：不联网的挡字段名，`#[ignore]` 的联网量字节数 |
| **空闲内存的私有值不能当单一定值**：同一构建、同一界面，无图 215.1–238.8 MB、有图 299.3–313.2 MB（修 360 缩略图之前有图曾到 187.0–580.5 MB） | 测量口径，不是代码 | **产品决定：不再追这个摆幅**。对外一律报中位数 + 区间，并带上界面状态；换设备差异只会更大。见 `docs/BENCHMARK.md` §5 |
| **webp 每次"设为壁纸"都重新解码 + 重编码**，转换结果不缓存（同一张图设两次要算两遍） | `wallpaper/current.rs::publish_to` | 图片大时几百毫秒。要优化就在缓存目录里存一份 `<hash>.wallpaper.jpg`，命中就直接用 |
| **安装包为 webp 多花 0.23 MB**（`image` crate 的 4 个解码器，exe +0.62 MB） | `src-tauri/Cargo.toml` | 这是有意的交换，不是待还的债；要缩体积只能砍解码器（比如去掉 gif），那会重新变成"某些图设不了壁纸" |
| **Acrylic 在 Windows 10 上的降级没在真机验证过**：`DWMWA_SYSTEMBACKDROP_TYPE` 在 Win10 会失败，代码只保证不崩 + CSS 半透明兜底 | `src-tauri/src/backdrop.rs` | 有 Win10 机器时跑一次 `measure-backdrop.ps1`，确认"无模糊但可读" |
| **`transparent: true` 让进程里多一个 15×15 的辅助顶层窗口**，`Process.MainWindowHandle` 可能指向它 | 测量脚本 / 任何自动化 | 脚本一律用 `bench/measure/lib-window.ps1` 按"标题 + 面积"挑窗口，别信 `MainWindowHandle` |
| ~~`{page}` 只能填页码，不支持偏移量分页~~ | `sources/fetch.rs` | **已还**：`{offset}`（`(page-1)*limit`）和 `{pageIndex}`（`page-1`，Gelbooru 的 `pid`）都在 |
| **`ResponseMapping.tags` 是死配置**：字段存在、`schema.ts` 里有、360 配置里写了 `"tags": "utag"`，但 `ImageRef` 没有 `tags` 字段、`extract_images` 从不读它、前端也没有任何地方显示——**一个没被消费的承诺** | `sources/model.rs`、`sources/builtin/sources.json`、`src/lib/sources/schema.ts` | 二选一：删干净，或把它接进 `ImageRef → ImageDto → UI`。别让它继续挂着 |
| `sfwQuery` 是静态追加，无法与 `{query}` 合并；booru 类接口要求所有条件在同一个 `tags` 参数里，两个 `tags` 会让 PHP 取最后一个、搜索词静默丢失 | `sources/fetch.rs::apply_sfw_query` | safebooru 的绕法是把分级写进 URL 模板；根治要改格式（安全条件与搜索词合并） |
| **"能不能直连"被当成站点属性写进文档**。同一台机器同一天，nekos.best 上午 403、下午 200 | `docs/SOURCES-CN.md`、README | 描述只能写"实测于某时某网络"，不写"国内不可用"。见 `docs/SOURCES-CN.md` §10 第 0 条 |
| **确定性列表在界面上和"缓存坏了"完全无法区分**：`pid=0`/`start=0` 永远返回同一批，用户关掉重开也一样，只会来报"你程序有 bug"。360 和 safebooru 各踩过一次 | `sources/builtin/sources.json` 的 `filter.random*Max` | 接图源时按 `docs/SOURCES-CN.md` §0.1 逐条过；`fetch.rs::deterministic_first_page_sources_must_declare_randomisation` 守着 safebooru 不被改回确定性 |
| 前端类型是手抄 Rust 结构体，改一边不会同步另一边 | `lib/api.ts` ↔ `dto.rs` | 结构体超过 5 个时考虑 `ts-rs` 自动生成 |
| `src/lib/sources/schema.ts` 暂时没人 import | — | 做图源编辑功能时会用到，别当垃圾删了 |
| 并发下载会撑爆内存（一页 24 张 4K ≈ 120MB） | `wallpaper/download.rs` | 做预加载时必须限并发 |
| 详情页没有焦点陷阱（Tab 能跑到覆盖层后面的元素） | `src/main.ts` | 无障碍完善时再加 `inert` 或焦点循环；不影响键盘关闭 |
| `ToggleChange.kind` 那种"用 String 当枚举"的写法 | 暂无（参考项目里有） | 见到就改成 `enum` |
| 360 壁纸是**非公开 API**：无文档、无 SLA，随时可能改或关；而且**没有内容分级参数**，只能固定动漫分类 | `sources/builtin/sources.json` | 挂了就改一段 JSON（图源是数据的好处）；内容限制见 `docs/SOURCES-CN.md` §5 |
| ~~没有 CI~~ | `.github/workflows/ci.yml` | **已还**：Rust 三道门 + 前端 tsc/vite，两个 job。**CI 不打包、不测数字**（体积/内存必须真机测），见 §2.5 |
| **`bench/make_icon.py` 的旧版本里有一个个人路径**（某人的图片目录），出现在提交 `d0d6373` | `bench/make_icon.py` | HEAD 已清掉（改成参数 / `MOE_ICON_SRC`）。**历史没重写**：重写会改掉之后所有提交的哈希、还要移动 `v0.1.2` tag，代价远大于这个泄漏。**别以为历史是干净的** |
| **安装包和 exe 都没有代码签名**（`Get-AuthenticodeSignature` → `NotSigned`）：SmartScreen 会警告、第三方杀软可能拦 | 发布物 | **SignPath Foundation 申请中**（材料与接入清单见 `docs/SIGNPATH.md`，签名步骤已预置在 `.github/workflows/release.yml`）。等它之前：每个 Release 必须写 SHA-256。**不买 EV、不自签发给用户** |

---

## 8. 排错速查（都是真实踩过的）

| 报错 | 原因 | 解法 |
|---|---|---|
| `unexpected argument '--nocapture' found` | `--` 忘了写 | `cargo test -- --nocapture` |
| `Unrecognized option: 'capture'` | 选项名错了，只有 `--nocapture` | 同上 |
| `cannot find function 'x'` + `similarly named function` | 拼写 | 照 help 里给的 `-`/`+` 改 |
| `unused import` 紧跟着别的错误 | **连带错误** | 先修第一条，再重新编译 |
| `exists but is inaccessible` | 作用域问题，通常是括号提前闭合 | 检查 `mod tests {}` 的范围 |
| `error: function ... is never used`（`-D warnings` 下） | 写了但还没被调用 | 不是 bug，等接入；**别加 `#[allow(dead_code)]`** |
| `could not find 'Cargo.toml'` | 在错的目录跑 cargo | 进 `src-tauri\` |
| `Port 1420 is already in use` | 上次 dev server 没关 | 结束 `node.exe`，或重启终端 |
| 窗口白屏 | 前端 JS 报错 | 窗口里按 F12 看 Console |
| `include_str!` 找不到文件 | `..` 数量不对 | 从当前 .rs 文件数到目标文件 |
| `json!` 报 `cannot find value x` | 宏里的键没加引号 | `{"x": 1}` 或 `{(var): 1}` |
| `no method named is_files` | 拼写，标准库只有 `is_file` | 照编译器 suggest 改 |
| `cannot find function 'x' in module 'y'` | 路径少了一层模块名 | 看对应 `mod.rs` 里到底 `pub mod` 了什么 |
| 测试**单独跑通过、一起跑失败** | `cargo test` 默认并行，两个测试共用了临时目录/端口 | 每个测试的目录名加自己的标识 |
| `! [rejected] main -> main (fetch first)` | 远端有你没有的提交 | `git fetch` → `git log --oneline origin/main ^main` 看清 → `git pull --rebase` → 再 push。**别 `--force`** |
| `Recv failure: Connection was reset`（`git push/fetch`） | 本机代理节点不稳，**不是 git 配置问题** | 换节点 / 稍后重试；`D:\meowallpaper\push-retry.ps1` 会自动重试 |
| **某个图源加载失败 / 连接超时** | 直连不通。**别再写"海外图源全挂"**——2026-10-31 实测 5 个图源里 4 个能直连（360 / safebooru / nekos.best / lolicon），只有 wallhaven 不通。程序会读 Windows 系统代理（`src-tauri/src/net.rs`），读不到就不走代理 | 先看是**哪一个**源失败，再决定是换节点还是换源。浏览器能开 wallhaven 就是代理配好了 |
| 某个图源 403，正文写着 IP 被拉黑 | nekos.best 按出口 IP 封禁，机场共享 IP 常见 | 换节点，改代码没用。**同一天不同时刻结果可能不同**，别据此断言"这个源废了" |
| 403，正文是 `Just a moment... Enable JavaScript`，响应头带 `Cf-Mitigated: challenge` | **Cloudflare 的 JS 挑战**（konachan.net 就是这样）。TCP 通、应用层拒绝，`reqwest`/`curl` 永远过不去 | 换源。要过这一关得跑无头浏览器，和"轻量"冲突 |
| 列表能拿到、图全裂；单独取图片地址超时 | 图片域名不通。lolicon 的原始域名 `i.pximg.net` **直连超时**（带不带 `Referer` 都一样） | 把图片域名钉到 `i.pixiv.re`（配置里 `proxy=i.pixiv.re`，有测试守着）。实测 `i.pixiv.re`/`i.pixiv.nl` 4/4 可用 |
| 搜一个不存在的词 → 显示"没有结果"而不是报错 | **这是对的**，别当 bug 修。safebooru 空结果返回 200 + 0 字节 body，`fetch.rs::parse_body` 把空正文当空列表 | — |
| **新接的图源每次打开、每次切回都是同一批图** | 该接口的第 1 页是确定性的（`pid=0`、`start=0`），而配置里没配随机 | 配 `randomPageIndexMax`（页码分页）或 `randomOffsetMax`（偏移量分页）。注意随机上限别超过服务端允许的深度（safebooru 是 8000 页）。**别先怀疑缓存**：缩略图缓存按 URL 哈希存，换源不可能命中同一批 URL——先看接口是不是确定性的 |
| **某个图源中英文都搜不到** | 搜索词不是目标站认的形态。booru 的标签是 `hatsune_miku`（下划线、多词 AND），`hatsune miku` 实测 0 条；中文还需先过词表 | 该图源配 `search.underscoreTags: true`；另外**搜索模板别带尺寸过滤**（`width:>=1920` 会把冷门标签吃成 0 条），尺寸交给界面上的分辨率档位 |
| **搜索结果明显不相关 / 搜什么都有一堆图** | 该接口的搜索是**相关性排序而不是精确匹配**（nekos.best 就是这样：`zzzznotexist` 也返回 5 张） | 先搜一个保证不存在的词验证。若它不返回空集，正确做法是**摘掉 `search` 块**让界面禁用，而不是加后置过滤——过滤后搜番名会几乎全空，比现在更像坏了。见 `docs/SOURCES-CN.md` §9.5 |
| 翻到很深的页 → 报错里写着 `API limited due to abuse` | safebooru 对异常 `pid` 返回 **XML** 而不是 JSON | 正常使用（连发 6 次）不会触发；错误信息现在会带上正文开头，能直接认出来 |
| 界面点了没反应 | 前端运行时异常，不是 Rust 崩了 | 窗口里 `F12` 看 Console 第一行红字 |
| 在 AI harness / DSH 里跑 `npm run tauri build` 报 `unrecognized subcommand 'D:\...\DSH Desktop.exe'` | **不是 Tauri 也不是项目的问题**：那个环境里 `node` 其实是 DSH 的 Electron 可执行文件，`process.argv[0]` 是它自己，`@tauri-apps/cli/tauri.js` 推断二进制名失败就把该路径当成子命令 | 用真终端跑 `npm run tauri build`。必须留在 harness 里打包时，先改写 `process.argv` 再 require `tauri.js`（`bench/measure/tauri-cli-workaround.js`）。**别为这个改 `package.json` 或降 CLI 版本** |
| 打包好的 exe 双击不弹窗、也不出界面；PowerShell 启动则几十毫秒就退出，退出码 `-2147483645`，事件日志和 WER 都干净 | **不是你的代码**。装了第三方杀软（本机是 Kaspersky 21.26，`KLIF`/`klflt` 内核驱动 hook 进程创建），未签名的构建会被它拦掉。先看 `root/SecurityCenter2` 和运行中的过滤驱动 | 把项目目录加进杀软排除项，或临时暂停防护。测量时两个基线要在同一防护状态下跑 |

---

## 9. 如果你用 AI 辅助写代码

这条很重要，用错了会让你"看起来写了代码，其实什么都不会"。

### 每次开新对话，先让 AI 读这几份

```
docs/HANDOVER.md
src/lib/api.ts                  ← 前端能调的三个命令，和 ImageDto
src-tauri/src/sources/dto.rs    ← ImageDto 的真实定义（api.ts 是手抄它的）
src-tauri/src/sources/model.rs  ← 图源配置的结构
```

不读这几份，AI 会建议你装 React、改字段名、换目录结构——**每一条都会和现有约定打架。**

### 六条使用要求

1. **一次只让它改一个文件、一个功能。** 一次让它"重构整个前端"，你会得到一堆看不懂的代码。
2. **改完自己跑测试**，不跑就提交等于没做。
3. **它给你的每个新函数，问它"这个函数什么时候会返回错误"**。答不上来就别合并。
4. **不许它升级依赖版本**（`Cargo.toml` / `package.json` 的版本号只有 A 能动）。
5. **报错先整段贴给 AI，别自己翻译成中文描述。** 原始报错信息里的 `error[E0425]`、行号、`help:` 都是关键线索。
6. **提交走 `git push`，不要用 GitHub 网页的 "Add files via upload"。**（见下）

### 提交走 git，别用网页上传

GitHub 网页的 **"Add files via upload" 不能指定目标路径**，只会把文件整包扔进一个新目录。

上一批前端代码就是这么进来的——落在 `MoeWallpaper-frontend-patch/` 里，目录结构和仓库对不上，多花了整整一轮整合。

正确流程：

```powershell
git switch -c feat/图片详情页
git add src/ index.html
git commit -m "feat(ui): open a detail view instead of setting the wallpaper"
git push -u origin feat/图片详情页
```

然后在 GitHub 上开 PR。

### 提示词模板

```
我在 D:\meowallpaper\Meowallpaper 做 MoeWallpaper（Tauri 2 + 原生 TS）。

先读 docs/HANDOVER.md，以及我下面指的文件，再看我的问题。
遵守文档 §5 的十条规矩，特别是：一个提交只做一件事、新功能要测试、
错误信息要带上下文。

我负责 src/** 和 index.html，不要改 src-tauri/** 和任何共享文件。

现在的问题是：<具体描述>
```

---

## 10. 绝对不要做的事

- ❌ 不看 `git status` 就 `git add -A`（会把不该提交的东西塞进历史，且很难清）
- ❌ `git push --force`
- ❌ 用 GitHub 网页上传文件代替 `git push`（路径会错，还得别人帮你搬）
- ❌ **点缩略图直接设壁纸**（已定死：必须先打开详情页，见 §0）
- ❌ 为了让警告消失而加 `#[allow(...)]`（先问"这个警告是不是在说实话"）
- ❌ 改 `tauri.conf.json` 里的 `identifier`（改了等于换一个应用，用户数据全丢）
- ❌ 往仓库里放任何图片文件
- ❌ 没跑过测试就说"做完了"
- ❌ 在 `main` 上直接跑 `git pull` 而不加 `--rebase`（会产生无意义的合并提交）
- ❌ 为了"让它先跑起来"去改别人的领地（先提 issue 或说一声，比事后拆雷便宜）

**开始干活前，永远先做这一步：**

```powershell
git pull --rebase origin main
```
