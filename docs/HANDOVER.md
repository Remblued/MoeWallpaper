# MoeWallpaper 交接文档

> 给新加入的合作者。读完这份文档，你应该能在 15 分钟内跑起项目，并知道下一步该做什么、什么不能做。
>
> 最后更新：2026-10-28 · 当前分支 `main`，最新提交 `d58a4d6`

---

## 0. 一页速览

- **是什么**：Windows 桌面壁纸应用（二次元向），Tauri 2 + 原生 TypeScript。
- **不是什么**：不是 Electron 项目，不用 React/Vue，不支持 macOS/Linux。
- **三个卖点**（写进 README 的，别丢）：安装包 5–10MB；图源用 JSON 声明、不做插件运行时；不内置任何图片。
- **现在**：能读配置列出图源，能按配置从（假的）API 响应里提取图片列表。**还不能联网，还不能设壁纸。**
- **代码量**：54 个文件，其中真正写的代码不到 500 行。**你读得完。**

---

## 1. 项目现在到哪儿了

### 已完成（有测试、有提交）

| 模块 | 内容 | 位置 |
|---|---|---|
| 仓库基础设施 | LICENSE(MIT)、README(中英)、.gitattributes、.editorconfig | 根目录 |
| 缓存路径 | 4 个路径函数，全部从 `cache_dir()` 派生 | `src-tauri/src/fs/paths.rs` |
| 目录创建 | `ensure_dir` / `ensure_base_layout` | `src-tauri/src/fs/dirs.rs` |
| 原子写文件 | 写 .tmp → fsync → rename | `src-tauri/src/fs/files.rs` |
| 图源配置解析 | JSON → Rust 结构体（serde） | `src-tauri/src/sources/model.rs` + `loader.rs` |
| 响应提取 | 点号路径取值 + 按映射提取图片列表 | `src-tauri/src/sources/extract.rs` |
| 一个真实命令 | `list_sources`：把内置图源列表交给前端 | `src-tauri/src/commands/mod.rs` |
| 界面 | 显示图源名字（极简） | `index.html` + `src/main.ts` |

**测试：14 个，全绿。**

### 未做（这就是接下来的活）

- ❌ 联网拉图（`reqwest` 还没加）
- ❌ 设置壁纸（Win32 `SystemParametersInfoW` 还没碰）
- ❌ 缩略图下载与缓存
- ❌ 定时切换
- ❌ 图标还是 Tauri 默认 logo
- ❌ 没有 CI

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
├─ index.html                  ← 窗口的骨架
├─ src/                        ← 前端（原生 TS）
│  ├─ main.ts                  ← 入口：invoke 取数据 → 渲染 DOM
│  └─ lib/sources/schema.ts    ← 前端侧的图源类型（手抄自 model.rs，见 §7）
├─ sources/
│  ├─ builtin/example.sources.json      ← 内置图源配置（会被编进 exe）
│  └─ examples/example.response.json    ← 假 API 响应，测试和文档用
├─ src-tauri/src/
│  ├─ lib.rs                   ← 组装：注册命令
│  ├─ commands/mod.rs          ← 门面层：只有 #[tauri::command]，不做逻辑
│  ├─ fs/                      ← paths(路径) / dirs(建目录) / files(原子写) / naming(URL→文件名)
│  ├─ sources/                 ← model(结构) / loader(加载) / extract(提取) / fetch(联网) / dto(出参)
│  └─ wallpaper/               ← set(调 Win32) / download(下载缓存)
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

### 已实现

```rust
list_sources() -> Result<Vec<WallpaperSource>, String>
```

### 待实现（A 负责，B 按此表先写界面）

```rust
// 拉一页图。page 从 1 开始。返回 20~30 条。
fetch_images(source_id: String, page: u32) -> Result<Vec<ImageDto>, String>

// 下载并把这张图设为桌面壁纸
set_wallpaper(image_url: String) -> Result<(), String>
```

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

---

## 5. 我们定的十条规矩

违反任何一条都会在 review 时被打回。

1. **一个提交只做一件事。** 主题是"加缩略图网格"，就只改前端那三个文件；顺手改的 README 单独一个提交。
2. **提交信息用 Conventional Commits**：`feat:` 新功能 / `fix:` 修 bug / `docs:` / `refactor:` / `test:` / `chore:`。标题不超过 72 字符，正文说"为什么"。
3. **提交前三道门全绿**：`cargo fmt`、`cargo clippy --all-targets -- -D warnings`、`cargo test`。前端加 `npx tsc --noEmit`。
4. **新功能必须有测试**，至少覆盖"正常"和"一种出错"。
5. **永不提交**：`target/`、`node_modules/`、`dist/`、任何图片文件、`.env`。
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

### A 的任务（按顺序）

| # | 任务 | 状态 | 验收标准 |
|---|---|---|---|
| A1 | 实现 `sources/fetch.rs::fetch_source_images` | 脚手架已好，**函数体待写** | 对着 `example.invalid` 报出可读的错（带状态码和 URL） |
| A2 | `ImageRef → ImageDto`（含缩略图回退） | ✅ 完成 | 单测：缺 thumbnail 时回退成 image_url |
| A3 | 实现 `wallpaper/set.rs::set_wallpaper`（Win32 FFI） | 脚手架已好，**函数体待写** | 能真的把桌面换掉；失败时有明确错误 |
| A4 | 实现 `wallpaper/download.rs::ensure_cached` | 脚手架已好，**函数体待写** | 重复设置同一张，第二次不发网络请求 |

三个待写函数在文件里都标了 `todo!`，上方注释写了具体步骤。
写完后把 `let _ = xxx;` 那几行占位代码删掉。

### B 的任务（按顺序，**不用等 A**）

| # | 任务 | 验收标准 |
|---|---|---|
| B1 | `src/lib/api.ts`：把 `invoke` 包一层，做成薄封装 | 只有一个文件里出现 `invoke(` |
| B2 | `src/lib/mock.ts`：造 24 条假 `ImageDto` | B1 里的开关一个常量就能切 mock / 真数据 |
| B3 | 缩略图网格（CSS Grid） | 窗口拉宽拉窄，网格自适应；不出现横向滚动条 |
| B4 | 三种状态：加载中 / 空 / 出错 | 每个状态都有可见文案，不是白屏 |
| B5 | 点缩略图 → 调 `set_wallpaper` | 点击后有"设置中…"提示 |

**B 为什么能不等 A**：因为 §4 的契约已经定了。B 对着 `ImageDto` 写界面，用 mock 数据，等 A 的 `fetch_images` 好了，把 `mock.ts` 关掉就接上。

**这就是"契约先行"的价值，也是这次协作你能学到的最重要的一件事。**

---

## 7. 已知的技术债（知道就好，先别动）

| 债 | 位置 | 什么时候还 |
|---|---|---|
| 前端类型是手抄 Rust 结构体，改一边不会同步另一边 | `schema.ts` ↔ `model.rs` | 结构体超过 5 个时考虑 `ts-rs` 自动生成 |
| `ToggleChange.kind` 那种"用 String 当枚举"的写法 | 暂无（参考项目里有） | 见到就改成 `enum` |
| 模板残留：`src/assets/*.svg` 已无人引用 | `src/assets/` | 做 UI 时顺手删 |
| 图标还是 Tauri 默认 logo | `src-tauri/icons/` | 定稿视觉时用 `npm run tauri icon` |
| README 的实测数字全是 `TBD` | `README.md` | 第一次 `npm run tauri build` 之后 |
| 没有 CI | — | 等 `fetch_images` 稳定后 |

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

---

## 9. 如果你用 AI 辅助写代码

这条很重要，用错了会让你"看起来写了代码，其实什么都不会"。

### 每次开新对话，先让 AI 读这三份

```
docs/HANDOVER.md
src-tauri/src/sources/model.rs
src/lib/sources/schema.ts
```

不读这三份，AI 会建议你装 React、改字段名、换目录结构——**每一条都会和现有约定打架。**

### 五条使用要求

1. **一次只让它改一个文件、一个功能。** 一次让它"重构整个前端"，你会得到一堆看不懂的代码。
2. **改完自己跑测试**，不跑就提交等于没做。
3. **它给你的每个新函数，问它"这个函数什么时候会返回错误"**。答不上来就别合并。
4. **不许它升级依赖版本**（`Cargo.toml` / `package.json` 的版本号只有 A 能动）。
5. **报错先整段贴给 AI，别自己翻译成中文描述。** 原始报错信息里的 `error[E0425]`、行号、`help:` 都是关键线索。

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
- ❌ 为了让警告消失而加 `#[allow(...)]`（先问"这个警告是不是在说实话"）
- ❌ 改 `tauri.conf.json` 里的 `identifier`（改了等于换一个应用，用户数据全丢）
- ❌ 往仓库里放任何图片文件
- ❌ 没跑过测试就说"做完了"
- ❌ 在 `main` 上直接跑 `git pull` 而不加 `--rebase`（会产生无意义的合并提交）

**开始干活前，永远先做这一步：**

```powershell
git pull --rebase origin main
```
