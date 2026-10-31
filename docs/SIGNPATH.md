# 申请 SignPath Foundation（免费开源签名）——材料与接入清单

> 一句话：SignPath Foundation 给开源项目**免费**的代码签名证书，私钥在他们的 HSM 上，
> **不需要个人身份公证**；他们验证并背书的是"这个二进制由这个仓库的 GitHub 工作流构建"。
> 申请入口：<https://signpath.org/apply>（表单要到他们的 Web 应用里填）。

## 1. 为什么这一步我不能替你做

申请表需要**你自己的 signpath.io 账号、邮箱验证和项目/维护者信息**，点"提交"之后走的是
人工审核。我能做、也已经做的是：把要填的材料整理好（§3 可直接抄）、把**前置条件中的代码
部分**做完（`release.yml`）、把批准后的接入步骤写成清单（§5）。

## 2. 前置条件（按他们的文档核对过，2026-08 版）

| 条件 | 我们的状态 |
| --- | --- |
| 仓库**公开**、有明确的 OSI 许可证 | ✅ public（2026-10-31），MIT |
| 产物必须**由 GitHub 工作流构建**，且先作为 **workflow artifact** 提交给 SignPath | ⚠️ 已新增 `.github/workflows/release.yml`；**从未用过 SignPath，接上后要跑一次验证** |
| 所有 job 必须跑在 **GitHub 托管 runner** 上（开源策略要求） | ✅ `runs-on: windows-latest`，别改自托管 |
| 安装 **SignPath GitHub App**（审计日志 / 策略评估用） | ⬜ 待你在 GitHub 上安装 |
| `upload-artifact` 默认存成 **ZIP** → Artifact Configuration 的根元素必须是 `<zip-file>` | ✅ 工作流就是默认 ZIP |

**关键一条**：本地 `npm run tauri build` 出来的安装包**不在签名范围内**——它不满足"由
GitHub 工作流构建"。所以我们以前那种"本机打包、手动传 Release"的流程，要和 CI 打包并存：
`release.yml` 用 `workflow_dispatch` 手动触发（默认不自动跑），产物附到同名 Release。

## 3. 申请表可以直接抄的内容

| 字段 | 填什么 |
| --- | --- |
| Project name | `MoeWallpaper` |
| Repository | `https://github.com/Remblued/MoeWallpaper` |
| License | MIT |
| Project description | 轻量 Windows 二次元壁纸应用，Tauri 2 + 原生 TypeScript；不内置任何图片，图源用 JSON 声明。仓库里没有二进制产物，安装包全部由 `tauri build` 产出。 |
| Build system | GitHub Actions（`windows-latest`），打包命令 `npx tauri build --bundles nsis` |
| Artifact to sign | NSIS 安装包 `MoeWallpaper_<version>_x64-setup.exe`（安装器内部含 `moe-wallpaper.exe` 与 `uninstall.exe`） |
| Release cadence | 不固定；按版本发（`v0.1.2` 为当前版本），每次发布附 SHA-256 |
| Maintainer | Remblued（`https://github.com/Remblued`） |
| Signing policy | 建议单条 `release-signing`：只允许从 tag 触发、要求 workflow 成功、拒绝 re-run 之外的花样（他们 UI 里给默认模板，按需勾） |

如果表单问"为什么不开源签名就不够"：**用户没法验证下载到的 exe 就是这个仓库构建的**，
而且未签名会被 SmartScreen 警告、部分杀软直接拦（我们在 `docs/HANDOVER.md` §8 记过实例）。

## 4. 建议的 Artifact Configuration

根是 `<zip-file>`（因为 artifact 是 ZIP），里面匹配安装包。**具体元素名以他们的语法参考为
准**（<https://docs.signpath.io/artifact-configuration>），别直接抄一段没验证过的 YAML。
要点只有三条：

1. 根元素 `<zip-file>`，与 `upload-artifact` 的默认打包方式对应；
2. 里面用 PE 文件规则匹配 `*.exe`（安装包本身，以及安装器内嵌的 `moe-wallpaper.exe`、
   `uninstall.exe`，如果它们出现在 ZIP 里）；
3. 先留一个"签名后校验"的开关，批准后第一次跑时用 `signtool verify /pa /all` 确认。

## 5. 批准之后要做的事（5 步）

1. 在 SignPath 建 **project**（slug 建议 `moewallpaper`）、**signing policy**（`release-signing`）、
   **artifact configuration**（§4）。
2. 拿三样东西：organization id、API token（有 submitter 权限的账号）。
3. GitHub 仓库里配：
   - Secrets：`SIGNPATH_API_TOKEN`
   - Variables：`SIGNPATH_ORGANIZATION_ID`、`SIGNPATH_PROJECT_SLUG`、
     `SIGNPATH_SIGNING_POLICY_SLUG`、`SIGNPATH_ARTIFACT_CONFIGURATION_SLUG`
   - `release.yml` 里签名那一步的 `if: vars.SIGNPATH_PROJECT_SLUG != ''` 会在变量就位后自动启用。
4. 装 **SignPath GitHub App**，并在 Actions 页面手动跑一次
   `Release → Run workflow`（输入 tag，例如 `v0.1.2`），看 Summary 里的
   `Signature : Valid`。
5. **签名会改变文件字节**，所以发布页/文档里的 SHA-256 要换成签名后的值：
   `README`、`docs/BENCHMARK.md` §5、Release 说明三处都要改。CI 的 Summary 会打印新哈希。

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
