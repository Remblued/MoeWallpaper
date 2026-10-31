# 代码签名怎么做，以及我们为什么现在还没签

> 本文回答两个问题：**签名到底解决什么**，以及**这个项目要签的话该怎么签**。
> 结论先放这儿：**`v0.1.2` 和 `v0.1.3` 都没签**——SignPath Foundation 在 2026-10-31 拒了申请（原因是项目还没有公开传播度，见 `docs/SIGNPATH.md` §0），所以"公开分发的第一个版本再签"这个计划没能落地。真要签，最可能走的是
> **免费的开源签名计划（SignPath Foundation）**，其次是 **OV 证书 + USB token**。
> `Azure Artifact Signing` 对个人开发者只开放美国和加拿大，中国大陆的个人/组织走不通。

---

## 1. 现在的状态（实测，不是推测）

```powershell
Get-AuthenticodeSignature .\src-tauri\target\release\bundle\nsis\MoeWallpaper_0.1.3_x64-setup.exe
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
| **SignPath Foundation**（开源免费签名） | **免费** | 公开的开源项目（OSI 许可，我们是 MIT）；**还要"传播度"**（star/fork/contributor、外部文章、独立讨论、机构背书） | ⭐ 首选，但 **2026-10-31 已因传播度不足被拒**，等有信号再申请（`docs/SIGNPATH.md` §0） |
| **Certum 开源代码签名证书**（云服务版） | **免费**（云签名，无实体卡） | 中国区运营方页面写明：**托管在 GitHub 上、Star 数超过 1k 的开源项目可免费申请**；云签名每月 5000 次上限 | ⭐ **另一扇免费的门**，而且门槛是明确的数字（1k star）。到那天再申请；云 HSM 能进 CI，比 USB token 强 |
| **Microsoft Store（MSIX）** | **免费**（微软重签；**个人开发者注册 2025-09 起免注册费**） | 上架审核；个人身份验证（政府证件 + selfie） | 可行但工作量中高：要做 MSIX 打包 + 过审。好处是**商店里的安装没有 SmartScreen 警告**，而且商店本身就是曝光渠道 |
| **OV 证书**（DigiCert / Sectigo / GlobalSign 等） | 约 **$150–300 / 年** | 全球；需提交组织或个人身份材料；**私钥必须在硬件 token / HSM 里**（2023-06 起 CA/B 论坛强制） | 有钱才考虑；签名要插 U 盘，进不了 CI |
| **Azure Artifact Signing**（原 Trusted Signing） | 约 **$9.99 / 月** | **组织**：美/加/欧盟/英国；**个人**：仅美国、加拿大 | ❌ **大陆走不通**。便宜的云签名对我们这个位置不开放 |
| **EV 证书** | **$400+ / 年** | 全球 | ❌ 不推荐：多花的钱换不到 SmartScreen 秒过（见 §2） |
| 自签名证书 | 免费 | — | ❌ 只能自己机器上用：别人机器会当成"不受信任"，装不上。**别发给用户** |

两条免费路径的来源：

- Certum 开源证书的门槛写在它的中国区运营页面（<https://certumcodesign.cn/shop/product/open-source-code-signing-cloud/>）："托管于 GITHUB.com，其 Star 数量超过 1k 的优秀开源项目可免费申请一个开源证书"，云签名每证书每月上限 5000 次。**政策会变，申请前自己去 Certum 官网确认一次。**
- Microsoft Store 个人开发者免注册费：<https://blogs.windows.com/windowsdeveloper/2025/09/10/free-developer-registration-for-individual-developers-on-microsoft-store/>（2025-09-10）。

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
Get-AuthenticodeSignature .\src-tauri\target\release\bundle\nsis\MoeWallpaper_0.1.3_x64-setup.exe
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
- **2026-10-31 申请了 SignPath Foundation，当天被拒**：理由不是材料，是**项目还没有公开
  传播度**（他们要看 star/fork/contributor、外部文章、独立讨论、机构背书、持续活跃）。
  原文摘要与重新申请的条件见 `docs/SIGNPATH.md` §0。
- **因此现在的决定是：先攒传播度，之后再申请 Foundation。** 中间不买付费方案——
  SignPath 的付费订阅 Starter 是 **$500/年**，而且它给的证书是 GlobalSign 的 **EV**，
  **只能签发给合法注册的组织**，个人主体买不了（价目与条文见 `docs/SIGNPATH.md` §0.1）。
- **不花钱的两条备选门**（都不现在做，但都比"花钱"更适合这个项目）：
  1. **Certum 开源证书**：门槛是明确的 **1k GitHub star**，免费、云 HSM、能进 CI；
  2. **Microsoft Store（MSIX）**：个人开发者注册免费、商店重签、装完没有 SmartScreen 警告，
     代价是要做 MSIX 打包 + 过审（商店本身也是曝光渠道）。
- **如果哪天真的必须先有签名又不想等**：向 CA 买 **OV 证书**（约 $150–300/年 + 硬件 token），
  签名要插 U 盘、不能全自动，接受这个代价再买。
- **不买 EV**：2024 年起它不再提供 SmartScreen 秒过（§2/§3）。
- **不自签名发给用户**，那只会在别人机器上变成"不受信任"。
- 无论签不签，**发布页必须写 SHA-256 + 状态说明**，这条已经是流程的一部分。
- 签名步骤继续留在 `.github/workflows/release.yml` 里（用 `vars.SIGNPATH_PROJECT_SLUG`
  挡着），重新申请通过后只配变量即可，不用重写流水线。

## 7. 这一版的范围：只有内置图源

产品决定（2026-10-31）：**`v0.1.x` 只提供内置的 5 个图源，用户自己接图源放到 `v0.2.0`。**

因此下面这些事**不阻塞现在的发布**，它们只在"允许用户塞任意 JSON 进来"之后才成为问题：

- `sfwQuery` 靠"记得配"（没有强制校验）——内置配置我们自己 review，用户配置才需要运行时校验
- 图源格式的向后兼容——只有我们自己写配置时，改格式是内部重构
- 加载阶段的 schema 校验与错误提示——同上

这一条写在 `docs/HANDOVER.md` §0/§7 里，避免下一个人把"自定义图源的前置条件"当成
现在必须补的洞。
