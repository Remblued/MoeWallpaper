"""把一张"黑底 + 圆角"的源图处理成 tauri 图标素材（透明圆角 PNG）。

用法：
    python bench/make_icon.py <源图.jpg>
    MOE_ICON_SRC=<源图.jpg> python bench/make_icon.py

**源图不在仓库里**（仓库不放图片，见 HANDOVER 的规矩），所以这个脚本只负责
"怎么把黑底图变成透明圆角"，不负责给你源图。

不猜圆角半径、不依赖 floodfill API，直接做连通域扩散：
1. 亮度 < DARK 的算"黑"
2. 从四条边向里扩散，只在"黑"里面走 → 得到真正的外部区域
3. 外部 alpha=0；边界 2px 过渡带按亮度算 alpha
4. 反预乘 F = C / a（底是纯黑，所以 C = a*F）
"""

import os
import sys

from PIL import Image, ImageFilter
import numpy as np

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

SRC = os.environ.get("MOE_ICON_SRC") or (sys.argv[1] if len(sys.argv) > 1 else "")
if not SRC:
    sys.exit("usage: python bench/make_icon.py <source.jpg>  (or set MOE_ICON_SRC)")
if not os.path.isfile(SRC):
    sys.exit(f"source image not found: {SRC}")

OUT = os.path.join(REPO, "app-icon.png")
PREVIEW = os.path.join(REPO, "bench", "icon-preview.png")
SIZE = 1024
DARK = 25

im = Image.open(SRC).convert("RGB").resize((SIZE, SIZE), Image.LANCZOS)
rgb = np.asarray(im).astype(np.float32)
lum = rgb.max(axis=2)

dark = lum < DARK
print(f"黑像素占比: {100.0*dark.mean():.2f}%   四角亮度: "
      f"{lum[0,0]:.0f} {lum[0,-1]:.0f} {lum[-1,0]:.0f} {lum[-1,-1]:.0f}")

# ---- 连通域扩散：从边界出发，只在黑里走 ----
outside = np.zeros_like(dark)
outside[0, :] = dark[0, :]
outside[-1, :] = dark[-1, :]
outside[:, 0] = dark[:, 0]
outside[:, -1] = dark[:, -1]

for step in range(1, 400):
    grown = outside.copy()
    grown[1:, :] |= outside[:-1, :]
    grown[:-1, :] |= outside[1:, :]
    grown[:, 1:] |= outside[:, :-1]
    grown[:, :-1] |= outside[:, 1:]
    grown &= dark
    if grown.sum() == outside.sum():
        break
    outside = grown
print(f"外部区域占比: {100.0*outside.mean():.2f}%  （扩散 {step} 轮）")

# ---- alpha ----
alpha = np.where(outside, 0.0, 255.0)

band_img = Image.fromarray((outside * 255).astype(np.uint8), "L").filter(ImageFilter.MaxFilter(5))
band = (np.asarray(band_img) > 0) & ~outside
alpha[band] = np.clip(lum[band] / 250.0 * 255.0, 0, 255)
print("过渡带像素数:", int(band.sum()))

a = np.asarray(
    Image.fromarray(alpha.astype(np.uint8), "L").filter(ImageFilter.GaussianBlur(0.6))
).astype(np.float32) / 255.0

# ---- 反预乘 + 全透明区填边缘色 ----
fill = np.mean([rgb[SIZE // 2, 3], rgb[SIZE // 2, SIZE - 4],
                rgb[3, SIZE // 2], rgb[SIZE - 4, SIZE // 2]], axis=0)
safe = a > 0.01
out_rgb = np.zeros_like(rgb)
out_rgb[safe] = rgb[safe] / a[safe][:, None]
out_rgb = np.clip(out_rgb, 0, 255)
out_rgb[~safe] = fill

Image.fromarray(np.dstack([out_rgb, a * 255.0]).astype(np.uint8), "RGBA").save(OUT)

# ---- 验证 ----
res = np.asarray(Image.open(OUT)).astype(np.int16)
al, res_lum = res[:, :, 3], res[:, :, :3].max(axis=2)
leftover = int(((al > 200) & (res_lum < 40)).sum())
cut = int(((al < 50) & (lum > 150)).sum())

print()
print("四角 alpha:", al[0, 0], al[0, -1], al[-1, 0], al[-1, -1], "(应全 0)")
print("中心 alpha:", al[SIZE // 2, SIZE // 2], "(应 255)")
print("【漏黑】", leftover, " 【切粉】", cut)
print("判定:", "OK" if leftover < 50 and cut < 50 else "仍需调整")

# ---- 预览 ----
sheet = Image.new("RGB", (560, 256), (120, 120, 120))
icon = Image.open(OUT)
for i, bg in enumerate(((32, 32, 36), (240, 240, 240))):
    canvas = Image.new("RGB", (SIZE, SIZE), bg)
    canvas.paste(icon, (0, 0), icon)
    sheet.paste(canvas.resize((256, 256), Image.LANCZOS), (i * 256, 0))
x = 520
for s in (64, 32, 16):
    t = icon.resize((s, s), Image.LANCZOS)
    sheet.paste(t, (x, 120 - s // 2), t)
    x += s
sheet.save(PREVIEW)
print("预览:", PREVIEW)
