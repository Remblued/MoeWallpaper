# MoeWallpaper 交接文档

> 给新加入的合作者。读完这份文档，你应该能在 15 分钟内跑起项目，并知道下一步该做什么、什么不能做。
>
> 最后更新：2026-10-29 · 分支 `main`。
> **具体状态以 `git log -1` 为准，本文不追提交号**——写死在文档里的哈希一定会过期，然后误导下一个读它的人。

---

## 0. 一页速览

- **是什么**：Windows 桌面壁纸应用（二次元向），Tauri 2 + 原生 TypeScript。
- **不是什么**：不是 Electron 项目，不用 React/Vue，不支持 macOS/Linux。
- **三个卖点**（写进 README 的，别丢）：安装包 5–10MB；图源用 JSON 声明、不做插件运行时；不内置任何图片。
- **现在**：能联网拉图、能下载缓存、能一键换桌面壁纸；界面有图源下拉 + 缩略图网格。**还缺"看大图"这一步。**
- **代码量**：62 个文件，Rust 约 820 行、前端约 520 行（都含注释和测试）。**你读得完。**

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
| 当前壁纸 | 复制到固定的 `current.jpg`，与缓存解耦 | `src-tauri/src/wallpaper/current.rs` |
| 四个命令 | `list_sources` / `fetch_images` / `set_wallpaper` / `save_image` | `src-tauri/src/commands/mod.rs` |
| 真实图源 | 360 / safebooru / wallhaven / nekos.best / lolicon，全部免费、无需 API key | `sources/builtin/sources.json` |
| 内容提示 | `contentNote`：图源名旁标出"尺度较大"（目前只有 lolicon）。**是提示，不是过滤器** | `sources/builtin/sources.json` + `src/main.ts::syncSourceUi` |
| 关键词搜索 | 每个图源可声明自己的搜索地址；中文走本地词表翻成英文标签 | `src-tauri/src/sources/search.rs` + `search/terms.json` |
| 前端 | 图源下拉、缩略图网格、四态、toast | `src/main.ts` + `src/lib/api.ts` + `src/lib/mock.ts` |

**测试：95 个（90 个自动跑，5 个标记 `#[ignore]` 手动跑——它们会真的改桌面或真的联网）。`cargo clippy -D warnings` 零告警。**

### 未做（这就是接下来的活）

- ✅ 玻璃主题（v0.1.1，窗口内 `backdrop-filter`，不是真 Mica/Acrylic）
- ❌ 真 Mica/Acrylic：要 `transparent: true` + `DwmSetWindowAttribute`，改动会动到窗口层
- ✅ 国内图源：360 壁纸（v0.1.1）；2026-10-31 又接上 safebooru，并把 lolicon 的图片域名钉到
  `i.pixiv.re`。**5 个图源实测 4 个能直连**，只有 wallhaven 不行。依据在 `docs/SOURCES-CN.md` §9–§10
- ✅ 五个图源都能"换一批"：360 用 `randomOffsetMax`、safebooru 用 `randomPageIndexMax`，
  其余三个接口本身就是随机的。**接新图源必须先回答"第 1 页是不是永远同一批"**——
  见 `docs/SOURCES-CN.md` §0.1 第 1 条
- ❌ 定时切换
- ❌ 没有 CI
- ⚠️ Electron 基线已经全部测完（安装包 115.57 MB / 安装后 356.77 MB / 冷启动
  104.5 ms / 空闲 133.5 MB 私有）。**但空闲内存这一项我们是输的**：218 MB 对
  133.5 MB，别在外面说"内存更省"。详见 `docs/BENCHMARK.md` §5

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
│  └─ pull_request_template.md ← 合并前的检查清单
└─ docs/
   ├─ BENCHMARK.md             ← 安装包体积怎么测（还没测）
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
| **A8** | 窗口半透明（改 `tauri.conf.json`，见 B8） | ⬜ |
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
| **B8** | 半透明 | ⬜ |
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
| **缓存会记住错误**：下载到一张 HTML 错误页，也会被永久当成"这张图"，不会自愈 | `wallpaper/download.rs` | 加「清空缓存」按钮；临时解法：删掉 `%LOCALAPPDATA%\com.remblued.moewallpaper` |
| `current.jpg` 里可能装着 PNG 数据，靠 Windows 按内容识别格式 | `wallpaper/current.rs` | 如果哪天换壁纸失效，**优先怀疑这里** |
| 安全参数靠"记得配"——图源没写 `sfwQuery` 就没有任何内容限制，而这一点只有 code review 能保证 | `sources/builtin/sources.json` | 做用户自定义图源时，在加载阶段强制校验：要么配 `sfwQuery`，要么显式标记"API 本身只提供安全内容" |
| 中文词表只收了约 150 个高频词，长尾词翻不了（`search/terms.json`） | `search/terms.json` | 持续加词；搜不到时 UI 已提示改用英文 |
| 英文译名对不上目标站标签就会搜不到（映射正确 ≠ 标签存在） | `search/terms.json` | 加词时**实测一次**：拿翻出来的英文去 wallhaven 搜，看条数是不是 0 |
| 国内免费接口几乎都是"单图 302 / 直接返回一张图"，不是列表；本项目 schema 只吃列表 | `sources/model.rs` | 要支持单图源，得加一种 source 类型（v0.4 插件化时一起做）。**2026-10-31 实测清单见 `docs/SOURCES-CN.md` §7、§9** |
| 360 壁纸有动漫分类（`cid=26`），但**不是公开 API**，条款不明，而且 `start` 是偏移量不是页码 | — | 已在 v0.1.1 接入并排第一；条款与风险见 `docs/SOURCES-CN.md` §5 |
| nekos.best 和 lolicon 不提供缩略图，网格会直接加载原图（一张 2–9MB） | `sources/builtin/sources.json` | 要么限制每页数量，要么后端生成缩略图。**safebooru 有 `preview_url`（7.5 KB），说明这不是"没法做"，是那几个接口不给** |
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
| 没有 CI | — | 等真实图源接上后，加一个跑三道门的 workflow |

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
