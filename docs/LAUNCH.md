# 传播清单（为什么它现在是技术工作的一部分）

SignPath Foundation 2026-10-31 拒了我们的申请，理由**只和传播度有关**：他们要看
"community adoption（stars/forks/contributors）、external articles、independent references or
discussions、institutional backing、sustained activity"。原文在 `docs/SIGNPATH.md` §0。

所以这份清单不是"运营 KPI"，而是**下次申请要交的证据**。目标不是刷数字，是**留下可核验的
外部链接**：一篇写清楚为什么这个应用值得用的文章，比 100 个来路不明的 star 有用。

## 发之前先备好（半天）

- [x] README 顶部：一句话是什么 + 下载入口 + 未签名状态 + CI 徽章
- [x] `DOWNLOAD.md` / `PRIVACY.md`：下载页与"不收集数据"的说明
- [x] `docs/BENCHMARK.md`：四个数字 + 测量方法（这是我们的差异化，别人抄不走）
- [ ] 3–5 张图：主界面（亮/暗/Acrylic 各一）、详情页、和 Electron 基线的体积对比图
- [ ] 一段 15 秒的录屏/GIF：打开 → 点一张图 → 设为壁纸（比截图文案有说服力）
- [ ] 第一个 tag 之后别再改 `v0.1.2` 的哈希（要改就发新版）

## 渠道（按"读者匹配度 × 门槛"排序）

| 渠道 | 语言 | 注意 |
| --- | --- | --- |
| **V2EX**（分享创造节点） | 中文 | 必须发在"分享创造"，标题写清"做了什么、为什么"；**别只贴链接**，正文要能自洽读完 |
| **少数派 / 知乎** | 中文 | 适合写"我为什么不用 Electron 做壁纸应用"这种一篇长文；知乎问题下回答比单开文章更容易被看到 |
| **Bilibili / 抖音** | 中文 | 15–30 秒"安装包 2 MB，打开就能换壁纸"的对比视频；标题里放体积对比最抓人 |
| **Reddit** `r/opensource`、`r/AnimeWallpaper`、`r/rust` | 英文 | 每个 sub 规则不同：`r/rust` 更认可"技术取舍"文章，别当广告发 |
| **Hacker News（Show HN）** | 英文 | 标题格式 `Show HN: MoeWallpaper – a 2 MB Windows wallpaper app (Tauri, no bundled browser)`；发出去后头两小时要盯评论 |
| **Lobsters / r/tauri** | 英文 | 量小但精准，适合发"我们怎么把安装包压到 2 MB" |
| **GitHub Topics / awesome-tauri 列表** | 英文 | 给 `awesome-tauri` 之类提 PR 加一行，是最便宜的长期曝光 |

## 两份草稿

### V2EX（中文，发"分享创造"）

```
标题：做了个 2 MB 的 Windows 二次元壁纸应用（Tauri，不打包浏览器）

正文：

背景：市面上的壁纸应用基本是 Electron 打包的，为了"显示一格图、把它设成壁纸"这一件事，
先要下 100 MB 以上。我不太能接受，就用 Tauri 2 + 原生 TypeScript 写了一个。

实测数字（同一台机器、release 构建，方法和原始值都在仓库 docs/BENCHMARK.md 里）：
- 安装包 2.04 MB（最小 Electron 基线是 115.57 MB）
- 安装后 5.99 MB
- 冷启动到窗口可见 39.5 ms
- 空闲内存 231 MB（无图）/ 307 MB（20 张图解码后）——这一项我们是输的，输给 Electron

几个我自己比较在意的点：
- 图源是 JSON 声明的数据，不是插件运行时；接新源改配置就行
- 不内置任何图片，仓库里一张壁纸都没有
- 玻璃主题是真 Acrylic（DwmSetWindowAttribute），桌面真的透上来
- webp/PNG/GIF 都能设成壁纸（先转码成 JPEG，不依赖商店扩展）
- 已知问题都写在 README 和 docs 里，包括"内存比 Electron 差"这一条

安装包没有代码签名（申请 SignPath 免费签名被拒，原因是我这个项目还没有传播度——先来攒），
所以首次运行可能被 SmartScreen 拦一下，发布页有 SHA-256 可以核对。

仓库：https://github.com/Remblued/MoeWallpaper
只有 Windows 版，MIT。
```

### Show HN（英文）

```
Show HN: MoeWallpaper – a 2 MB Windows wallpaper app that borrows the system WebView2

I wanted a wallpaper app that doesn't ship a browser engine for a job this small, so I built one
with Tauri 2 and plain TypeScript.

Measured on one machine, release build, method and raw values in the repo:
- installer 2.04 MB (minimal Electron baseline: 115.57 MB)
- installed 5.99 MB
- cold start to visible window 39.5 ms
- idle memory 231 MB with an empty grid, 307 MB with 20 images decoded — this is the metric we
  lose, and the README says so instead of hiding it

Things I found worth writing down: the default source returned full-size originals in its
"thumbnail" field (so the grid decoded 20 1440p wallpapers), and Windows can't read WebP without a
Store extension, so images are transcoded to JPEG before being handed to
SystemParametersInfoW.

Not code signed yet (SignPath Foundation declined for lack of project visibility, which is fair),
so the release page publishes a SHA-256 instead.

https://github.com/Remblued/MoeWallpaper
```

## 记录表（每次发完填一行）

| 日期 | 渠道 | 链接 | 结果（浏览/评论/star 变化） |
| --- | --- | --- | --- |
| | | | |

**别做的事**：买 star、开小号互捧、把同一个帖复制到五个 sub。他们要的是"独立引用与讨论"，
刷出来的数字在人工审核眼里一眼能看出来，而且会让整份材料不可信（和我们
`docs/BENCHMARK.md` §0 的"不许编数字"是同一条规矩）。

## 什么时候重新申请

不用等一个精确数字（他们没公开门槛）。按他们列的信号，准备到**至少满足三条**再去：

1. 有 ≥1 篇外部文章/视频，不是自己发的；
2. 有 ≥1 处独立讨论（Reddit 帖、HN 评论区、V2EX 回帖都算），里面有人真的用过；
3. star / fork / 下载量是**自然增长**的，不是一次性冲上来的；
4. 仓库保持连续提交（CI、release、文档都算）。

到时候把 `docs/SIGNPATH.md` §3 的表单字段再填一遍即可，`release.yml` 里的签名步骤不用重写。
