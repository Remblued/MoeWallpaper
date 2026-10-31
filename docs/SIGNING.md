# 代码签名怎么做，以及我们为什么现在还没签

> 本文回答两个问题：**签名到底解决什么**，以及**这个项目要签的话该怎么签**。
> 结论先放这儿：**`v0.1.2` 不签，公开分发的第一个版本再签**；真要签，最可能走的是
> **免费的开源签名计划（SignPath Foundation）**，其次是 **OV 证书 + USB token**。
> `Azure Artifact Signing` 对个人开发者只开放美国和加拿大，中国大陆的个人/组织走不通。

---

## 1. 现在的状态（实测，不是推测）

```powershell
Get-AuthenticodeSignature .\src-tauri\target\release\bundle\nsis\MoeWallpaper_0.1.2_x64-setup.exe
#   Status  : NotSigned
#   Signer  : (空)
Get-AuthenticodeSignature .\src-tauri\target\release\moe-wallpaper.exe
#   Status  : NotSigned
```

**未签名的实际后果**（这台机器上都见过）：

| 现象 | 原因 |
| --- | --- |
| 双击安装包弹「Windows 已保护你的电脑」 | SmartScreen：没有签名 + 没有下载信誉，**这是预期行为，不是 bug** |
| 第三方杀软直接拦掉进程创建（本机 Kaspersky，退出码 `-2147483645`） | 未签名 + NSIS 自解压结构是启发式的常见组合，见 `docs/HANDOVER.md` §8 |
| 从网上下载后首次运行更慢 / 弹「发布者无法验证」 | `Zone.Identifier`（"来自 Internet"标记）+ Shell 的发布者验证。这也是 `docs/BENCHMARK.md` §2.3 坚持用 CreateProcess 测量、不用 `Start-Process` 的原因 |

## 2. 签名解决什么，不解决什么

**解决：**

- 文件被改过就能被发现（Authenticode 是内容哈希 + 时间戳 + 证书链）
- 用户能看到"谁发布的"，而不是"未知发布者"
- 企业/受管设备可能只允许已签名的程序运行

**不解决（这几条最容易被误解）：**

- **不保证没有 SmartScreen 警告。** SmartScreen 看的是**发布者信誉**，信誉要按发布者身份
  慢慢积累。第一次发布、换证书、换公司名，都可能重新从零开始。
- **EV 证书不再"秒过" SmartScreen。** 微软文档写得很明确：2024 年起 EV 和 OV 一样要走
  信誉积累，为"绕过 SmartScreen"多花 EV 的钱**已经不划算**。
- **不保证杀软不误报。** 它降低概率，但不等于零。
- **时间戳必须打。** 不打时间戳，证书一过期，之前签的文件全部变成"签名过期"。
  所以任何签名命令都要带 `timestampUrl` / `/tr`。

来源：微软官方的选择指南
[Code signing options for Windows app developers](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options)
（2026-08-29 更新）。价格区间也来自这一页，**买之前请向 CA 或 Azure 再确认一次**。

## 3. 五条路，按"这个项目能不能用"排序

| 方案 | 成本（文档给的） | 资格 | 对我们的可用性 |
| --- | --- | --- | --- |
| **SignPath Foundation**（开源免费签名） | **免费** | 公开的开源项目（OSI 许可，我们是 MIT）；需要仓库公开、走它的托管流水线 | ⭐ **公开后首选**：免费、给到 OV 级别签名，正好卡在我们"要转 public"的节点 |
| **OV 证书**（DigiCert / Sectigo / GlobalSign 等） | 约 **$150–300 / 年** | 全球；需提交组织或个人身份材料；**私钥必须在硬件 token / HSM 里**（2023-06 起 CA/B 论坛强制） | 可用。要买一个 USB token，签名时插上；适合想自己掌握身份、不想依赖第三方流水线 |
| **Azure Artifact Signing**（原 Trusted Signing） | 约 **$9.99 / 月** | **组织**：美/加/欧盟/英国；**个人**：仅美国、加拿大 | ❌ **大陆走不通**。便宜的云签名对我们这个位置不开放 |
| **Microsoft Store（MSIX）** | 免费（商店重新签名） | 需要上架商店 | 能用但要改成 MSIX 打包 + 过审；和"NSIS 便携安装包"不是一条路，**不建议现在做** |
| **EV 证书** | **$400+ / 年** | 全球 | ❌ 不推荐：多花的钱换不到 SmartScreen 秒过（见 §2） |
| 自签名证书 | 免费 | — | ❌ 只能自己机器上用：别人机器会当成"不受信任"，装不上。**别发给用户** |

## 4. 真要签的时候，具体怎么做

### 4.1 先确认工具链

Windows SDK 自带 `signtool.exe`，本机已经在：

```text
C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe
```

### 4.2 OV 证书（USB token）——手动签

插上 token 之后：

```powershell
$signtool = 'C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\signtool.exe'

# 1) 找到证书的 SHA-1 指纹（token 里那张）
Get-ChildItem Cert:\CurrentUser\My | Where-Object { $_.HasPrivateKey } |
  Select-Object Subject, Thumbprint, NotAfter

# 2) 签安装包（exe 也要签，见 §4.4）
& $signtool sign /fd sha256 /td sha256 `
  /tr http://timestamp.digicert.com `
  /sha1 <证书指纹> `
  .\src-tauri\target\release\bundle\nsis\MoeWallpaper_0.1.2_x64-setup.exe

# 3) 验证（一定要自己验一次，别只看 signtool 没报错）
& $signtool verify /pa /all .\src-tauri\target\release\bundle\nsis\MoeWallpaper_0.1.2_x64-setup.exe
Get-AuthenticodeSignature .\src-tauri\target\release\bundle\nsis\MoeWallpaper_0.1.2_x64-setup.exe
#   Status 应为 Valid
```

### 4.3 让 Tauri 打包时自动签

`src-tauri/tauri.conf.json` 里加一段（**键名以本机 CLI 的 schema 为准**：`digestAlgorithm`
/ `certificateThumbprint` / `timestampUrl` / `tsp` / `signCommand`）：

```json
{
  "bundle": {
    "windows": {
      "digestAlgorithm": "sha256",
      "certificateThumbprint": "<证书 SHA-1 指纹，没有空格>",
      "timestampUrl": "http://timestamp.digicert.com"
    }
  }
}
```

- `certificateThumbprint` 是 **SHA-1 指纹**（不是 SHA-256 摘要），从证书管理器里看。
- 证书在 token 里时，打包过程会**弹 PIN 输入框**，不能全自动——这是硬件私钥的正常代价。
- 指纹是"某台机器上的某张证书"，**不要把它提交进仓库**：用环境变量或本地未跟踪的
  配置文件覆盖（Tauri 支持 `--config` 传入额外配置片段）。

### 4.4 签哪些文件

| 文件 | 要不要签 | 说明 |
| --- | --- | --- |
| `moe-wallpaper.exe` | **要** | 用户双击运行的就是它，杀软看的是它 |
| `MoeWallpaper_0.1.2_x64-setup.exe` | **要** | 用户下载的就是它，SmartScreen 拦的也是它 |
| `uninstall.exe` | 要（但麻烦） | NSIS 卸载器是安装时生成的，Tauri 默认不签它。要签得改 NSIS 模板或安装后补签 |
| `*.dll`（本项目目前没有） | — | 有原生依赖时同理 |

**顺序**：先签内部 exe，再打安装包，最后签安装包。反了的话安装包里塞的是未签名的 exe。

### 4.5 云签（SignPath / Artifact Signing）：走 `signCommand`

云签名不给你私钥，所以你本地没有指纹可以填，只能把一个**签名命令**交给 Tauri：

```json
{
  "bundle": {
    "windows": {
      "signCommand": { "cmd": "path\\to\\sign-tool", "args": ["sign", "%1"] }
    }
  }
}
```

`%1` 会被替换成待签文件路径。SignPath 的做法是装它的签名工具、用项目 token 换证书、
再对文件签名；具体参数以 SignPath 的接入文档为准，**换服务时只改这一段，不动构建脚本**。

## 5. 校验：用户怎么相信这个包

签名之外，**发布 SHA-256 仍然是必要的一步**（我们已经在做）：

```powershell
Get-FileHash .\MoeWallpaper_0.1.2_x64-setup.exe -Algorithm SHA256
#   2440651A5929BE0CF13C819F743A18C885D5193487303C27E46E02886F44FB9A
```

发布页写哈希 + `docs/BENCHMARK.md` §5 记构建信息，用户可以自己比对"下载到的"和"发布者
测过的"是不是同一个文件。有签名之后这条也保留：签名证明"是谁发的"，哈希证明"是不是这个包"。

## 6. 本项目的决定

- **`v0.1.2` 不签。** 当时仓库还是 private，签名只会在每次打包时多要一次 PIN，收益为零。
- **仓库已于 2026-10-31 转 public**，所以 **SignPath Foundation 的资格条件现在满足了**
  （公开仓库 + OSI 许可 + 有 CI）。申请材料、前置条件和批准后的接入清单写在
  `docs/SIGNPATH.md`；签名那一步已经预置在 `.github/workflows/release.yml` 里，
  变量一配就生效。
- **公开分发的第一个版本必须签。** 首选 SignPath；如果它不收（名额/资格问题），再买
  **OV 证书 + token**。
- **不买 EV**，理由见 §2/§3：2024 年起它不再提供 SmartScreen 秒过。
- **不自签名发给用户**，那只会在别人机器上变成"不受信任"。
- 无论签不签，**发布页必须写 SHA-256**，这条已经是流程的一部分。

## 7. 这一版的范围：只有内置图源

产品决定（2026-10-31）：**`v0.1.x` 只提供内置的 5 个图源，用户自己接图源放到 `v0.2.0`。**

因此下面这些事**不阻塞现在的发布**，它们只在"允许用户塞任意 JSON 进来"之后才成为问题：

- `sfwQuery` 靠"记得配"（没有强制校验）——内置配置我们自己 review，用户配置才需要运行时校验
- 图源格式的向后兼容——只有我们自己写配置时，改格式是内部重构
- 加载阶段的 schema 校验与错误提示——同上

这一条写在 `docs/HANDOVER.md` §0/§7 里，避免下一个人把"自定义图源的前置条件"当成
现在必须补的洞。
