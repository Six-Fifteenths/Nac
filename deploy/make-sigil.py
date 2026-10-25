"""画 `site-cain.society/img/sigil.png` —— 该隐学会的圣印。

原版是一个 SVG：

    <svg viewBox="0 0 100 100" fill="var(--cain-red)">
      <path d="M50 5 L95 85 L5 85 Z M50 30 L70 70 L30 70 Z" />
    </svg>

两条子路径的绕向相同，nonzero 填充规则下里面那条被外面那条盖住 —— 所以
看上去就是一个实心三角形。客户端不解析 SVG，所以这里把它画成像素：
一个 #8a0303 的等边三角形，透明底，4×4 超采样。

用法（在仓库根目录）：

    python deploy/make-sigil.py

只有改颜色或尺寸时才需要重跑；生成的 png 已经提交。
"""

import struct
import sys
import zlib
from pathlib import Path

SIZE = 256
RED = (0x8A, 0x03, 0x03)
# 原版 viewBox 里的三个顶点（0..100）。
TRIANGLE = [(50.0, 5.0), (95.0, 85.0), (5.0, 85.0)]
SUPERSAMPLE = 4


def _edge(ax, ay, bx, by, px, py):
    return (px - ax) * (by - ay) - (py - ay) * (bx - ax)


def _inside(px, py):
    (x1, y1), (x2, y2), (x3, y3) = TRIANGLE
    d1 = _edge(x1, y1, x2, y2, px, py)
    d2 = _edge(x2, y2, x3, y3, px, py)
    d3 = _edge(x3, y3, x1, y1, px, py)
    has_neg = d1 < 0 or d2 < 0 or d3 < 0
    has_pos = d1 > 0 or d2 > 0 or d3 > 0
    return not (has_neg and has_pos)


def render():
    pixels = bytearray(SIZE * SIZE * 4)
    step = 100.0 / (SIZE * SUPERSAMPLE)
    for y in range(SIZE):
        for x in range(SIZE):
            hits = 0
            for sy in range(SUPERSAMPLE):
                for sx in range(SUPERSAMPLE):
                    px = (x * SUPERSAMPLE + sx + 0.5) * step
                    py = (y * SUPERSAMPLE + sy + 0.5) * step
                    if _inside(px, py):
                        hits += 1
            if not hits:
                continue
            alpha = round(255 * hits / (SUPERSAMPLE * SUPERSAMPLE))
            i = (y * SIZE + x) * 4
            pixels[i : i + 4] = bytes((RED[0], RED[1], RED[2], alpha))
    return pixels


def write_png(path, width, height, rgba):
    def chunk(tag, data):
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    rows = b"".join(
        b"\x00" + bytes(rgba[y * width * 4 : (y + 1) * width * 4]) for y in range(height)
    )
    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(rows, 9))
        + chunk(b"IEND", b"")
    )
    Path(path).write_bytes(png)


if __name__ == "__main__":
    out = Path(sys.argv[1] if len(sys.argv) > 1 else "site-cain.society/img/sigil.png")
    out.parent.mkdir(parents=True, exist_ok=True)
    write_png(out, SIZE, SIZE, render())
    print("wrote %s (%d bytes)" % (out, out.stat().st_size))
