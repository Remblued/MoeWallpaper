# 申请 SignPath Foundation（免费开源签名）——材料与接入清单

> 一句话：SignPath Foundation 给开源项目**免费**的代码签名证书，私钥在他们的 HSM 上，
> **不需要个人身份公证**；他们验证并背书的是"这个二进制由这个仓库的 GitHub 工作流构建"。
> 申请入口：<https://signpath.org/apply>（表单要到他们的 Web 应用里填）。

## 0. 状态（2026-10-31）

**已提交，已收到确认邮件，等人工审核。** 原文要点：

> "We've received your application and will review it shortly. Our team will get back to you
> within the next few business days with next steps or any follow-up questions."

- **不要重复提交。** 表单是 HubSpot 嵌入的（`js-eu1.hsforms.net`），第一次提交时前端一直转圈
  但其实**已经送达**——重复提交只会被节流或制造重复申请。
- 如果确认邮件里把收件人名字写错了（会用来创建 SignPath 账号），**回一封邮件更正**，
  别等账号建好再改。
- 审核期间能做的都列在 §5.0；等他们回信问问题就照 §3 的材料答。

## 1. 为什么这一步我不能替你做

申请表需要**你自己的 signpath.io 账号、邮箱验证和项目/维护者信息**，点"提交"之后走的是
人工审核。我能做、也已经做的是：把要填的材料整理好（§3 可直接抄）、把**前置条件中的代码
部分**做完（`release.yml`）、把批准后的接入步骤写成清单（§5）。

## 2. 前置条件（按他们的文档核对过，2026-08 版）

| 条件 | 我们的状态 |
| --- | --- |
| 仓库**公开**、有明确的 OSI 许可证 | ✅ public（2026-10-31），MIT |
| **下载页要存在，并且页面上必须写明"代码签名由 SignPath Foundation 提供"** | ✅ [DOWNLOAD.md](../DOWNLOAD.md)，README 顶部也写了 |
| 隐私政策页（收集数据才必须；我们不收集，但表单问链接） | ✅ [PRIVACY.md](../PRIVACY.md) |
| 产物必须**由 GitHub 工作流构建**，且先作为 **workflow artifact** 提交给 SignPath | ✅ `.github/workflows/release.yml`，2026-10-31 首次运行全绿 |
| 所有 job 必须跑在 **GitHub 托管 runner** 上（开源策略要求） | ✅ `runs-on: windows-latest`，别改自托管 |
| 安装 **SignPath GitHub App**（审计日志 / 策略评估用） | ⬜ 待你在 GitHub 上安装 |
| `upload-artifact` 默认存成 **ZIP** → Artifact Configuration 的根元素必须是 `<zip-file>` | ✅ 工作流就是默认 ZIP |

**关键一条**：本地 `npm run tauri build` 出来的安装包**不在签名范围内**——它不满足"由
GitHub 工作流构建"。所以我们以前那种"本机打包、手动传 Release"的流程，要和 CI 打包并存：
`release.yml` 用 `workflow_dispatch` 手动触发（默认不自动跑），产物附到同名 Release。

## 3. 申请表字段（照这个填，标 ⚠️ 的是要你自己决定的）

表格是 2026-10-31 那次申请页上的实际字段。

| 字段 | 填什么 |
| --- | --- |
| **Homepage URL** | `https://github.com/Remblued/MoeWallpaper`（README 就是主页；**别和 Download URL 填同一个**） |
| **Download URL** | `https://github.com/Remblued/MoeWallpaper/blob/main/DOWNLOAD.md`（这页里写明了"代码签名由 SignPath Foundation 提供"） |
| **Privacy Policy URL** | `https://github.com/Remblued/MoeWallpaper/blob/main/PRIVACY.md` |
| **Wikipedia URL**（可选） | 留空 |
| **Tagline** ⚠️ | `A 2 MB anime wallpaper app for Windows that borrows the system WebView2 instead of shipping a browser.` |
| **Category** | `Graphics`。**他们下拉里的真实选项**（2026-10-31 从 signpath.org/projects 抓到）：`Accessibility · Audio · Developer tools · Educational · Gaming · Graphics · Productivity · Science · Other`。**不要选 Accessibility**（那是读屏/辅助技术类）。这格只影响项目出现在哪个分组，不影响审核 |
| **Description** | `MoeWallpaper is a lightweight desktop wallpaper application for Windows. It shows a grid of anime wallpapers fetched from declarative, JSON-described public image sources, and lets the user preview an image, save it, or set it as the desktop wallpaper. It is built with Tauri and the system WebView2 runtime rather than a bundled browser engine, ships no images of its own, and is written in Rust and plain TypeScript. The project publishes reproducible measurements of its size, startup time and memory use, and a SHA-256 hash for every installer it releases.` |
| **Reputation** ⚠️ | **这是最弱的一格，必须诚实填**，见 §3.1 |
| **Maintainer Type** ⚠️ | `Individual`（如果下拉里有 Individual developer / Independent，选它） |
| **Build System** | `GitHub Actions` |
| **First / Last Name** ⚠️ | **你自己的名字**（会用来创建 SignPath 账号） |
| **Email** ⚠️ | 你能收信的邮箱（申请通知 + 账号） |
| **Company Name**（可选） | 留空，除非你有实体 |
| **Primary Discovery Channel** ⚠️ | 你实际上是从哪知道 SignPath 的。如果是这次查资料时由 AI 助手推荐的，就选 `AI assistant` / `Other` 并在下一格写清楚 |
| **Please specify the exact source**（可选） | 例：`Recommended by an AI coding assistant while researching Windows code signing; the trail started at Microsoft's "Code signing options for Windows app developers" page.` |

### 3.1 Reputation 怎么填（不要编）

我们是一个 2026-10-31 才公开的新项目，**没有媒体报道、没有下载统计、没有社区讨论**。
硬编一个"被广泛使用"会让整份申请不可信。可以写的是**可核验的工程证据**：

```text
New project: public repository since 2026-10-31, so there is no adoption data yet - no
media coverage, no download statistics, no community threads. What can be verified today:

- CI runs on every commit and the run history is public: fmt, clippy -D warnings and the
  full test suite on Windows, plus type-check and production build for the frontend
  (https://github.com/Remblued/MoeWallpaper/actions).
- 102 automated tests in the repository, 6 of them explicitly marked as manual because
  they touch the network or the desktop.
- Release integrity is documented rather than claimed: every installer ships with a
  published SHA-256 (e.g. 2440651A5929BE0CF13C819F743A18C885D5193487303C27E46E02886F44FB9A
  for v0.1.2), and the build it came from is recorded with machine, toolchain and raw
  measurement values in docs/BENCHMARK.md.
- A CI release workflow already builds the installer from a tag and stores it as a
  workflow artifact, which is the model this application would rely on.
- Public history with no rewrites; vulnerability reporting enabled.

If reputation evidence is a hard requirement, we would rather be told so and reapply later
than have unverifiable claims on the record.
```

**如果你觉得这格过不了**：先别投，等仓库有真实的 star / 下载量 / 讨论（比如发到
r/opensource、V2EX、少数派之后），再申请。**不要为了过审编数字**——这一条和我们
`docs/BENCHMARK.md` §0 的规矩是同一条。

## 4. 建议的 Artifact Configuration

根是 `<zip-file>`（因为 artifact 是 ZIP），里面匹配安装包。**具体元素名以他们的语法参考为
准**（<https://docs.signpath.io/artifact-configuration>），别直接抄一段没验证过的 YAML。
要点只有三条：

1. 根元素 `<zip-file>`，与 `upload-artifact` 的默认打包方式对应；
2. 里面用 PE 文件规则匹配 `*.exe`（安装包本身，以及安装器内嵌的 `moe-wallpaper.exe`、
   `uninstall.exe`，如果它们出现在 ZIP 里）；
3. 先留一个"签名后校验"的开关，批准后第一次跑时用 `signtool verify /pa /all` 确认。

## 5. 批准之后要做的事（5 步）

### 5.0 等审核期间就能做的（不用干等）

1. **装 SignPath GitHub App**（GitHub → Settings → Applications → SignPath）。他们的开源策略
   要用它做审计/策略评估，早装早省事。
2. **定好三个 slug**（批准后建项目时直接填，避免临时起名）：
   - project：`moewallpaper`
   - signing policy：`release-signing`
   - artifact configuration：`moewallpaper-nsis`
3. **想清楚第一个签哪个版本**：**不要**回头签 `v0.1.2`——它的安装包已经发布、哈希已经写进
   README/BENCHMARK/Release 三处，签名会改字节，等于要同时改三处记录。**从下一个 tag
   （`v0.1.3`）开始签**：先让 `release.yml` 用新 tag 跑一遍签名的完整流程，`v0.1.2` 保持
   未签名、哈希不动。

### 5.1 正式接入（5 步）

1. 在 SignPath 建 **project**（slug 建议 `moewallpaper`）、**signing policy**（`release-signing`）、
   **artifact configuration**（§4）。
2. 拿三样东西：organization id、API token（有 submitter 权限的账号）。
3. GitHub 仓库里配：
   - Secrets：`SIGNPATH_API_TOKEN`
   - Variables：`SIGNPATH_ORGANIZATION_ID`、`SIGNPATH_PROJECT_SLUG`、
     `SIGNPATH_SIGNING_POLICY_SLUG`、`SIGNPATH_ARTIFACT_CONFIGURATION_SLUG`
   - `release.yml` 里签名那一步的 `if: vars.SIGNPATH_PROJECT_SLUG != ''` 会在变量就位后自动启用。
4. 打一个新 tag（`v0.1.3`），在 Actions 页面手动跑
   `Release → Run workflow`（tag 填 `v0.1.3`），看 Summary 里的 `Signature : Valid`。
5. **签名会改变文件字节**，所以发布页/文档里的 SHA-256 要换成签名后的值：
   `README`、`docs/BENCHMARK.md` §5、Release 说明三处都要改。CI 的 Summary 会打印新哈希。

**一个已知的覆盖范围限制**：这条流水线签的是**外层安装包**（`MoeWallpaper_*_setup.exe`）。
安装器里解包出来的 `moe-wallpaper.exe` 是 `tauri build` 现编的，不经过 SignPath，所以**内层
exe 没有 Authenticode 签名**。要连内层一起签，得改成两阶段（先构建 app exe → 签名 → 再
打包），或者用 Tauri 的 `signCommand` 在打包过程中逐文件签——**都还没做**，先按"签外层"
跑通，再决定值不值得做。

## 6. 验收标准（别只看"没报错"）

```powershell
Get-AuthenticodeSignature .\MoeWallpaper_0.1.2_x64-setup.exe   # Status 必须是 Valid
signtool verify /pa /all .\MoeWallpaper_0.1.2_x64-setup.exe
```

再加一条人工检查：**换一台没装过任何证书的机器**双击安装包，确认 SmartScreen 不再说
"未知发布者"（新发布者第一次仍可能有信誉积累期，见 `docs/SIGNING.md` §2——签名不是免死金牌）。

## 7. 时间线与不做的部分

- 审核是**人工**的，几天到几周都可能；**没实测过时限，不写死**。
- 不买 EV（2024 起不再秒过 SmartScreen），也不先买 OV：先把 SignPath 申请走完，
  被拒再谈 OV（`docs/SIGNING.md` §3 有价格与资格）。
- 拿到签名之前，**每个 Release 必须写 SHA-256**——这条和签名无关，是现在的替代验证手段。
