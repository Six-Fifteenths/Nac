# nac:title 运营台 · 内容清单
# 只读：把 site/ 里的页面列出来，带上从 # nac:title 读到的页名与改动时间。

import os
import time
from pathlib import Path

import nac

root = Path(os.environ.get("NAC_ROOT") or ".")
site = root / "site"


def title_of(path):
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines()[:5]:
        line = line.strip()
        if line.startswith("# nac:title"):
            return line[len("# nac:title"):].strip() or "（空）"
    return "（未命名）"


pages = sorted(site.glob("*.py"), key=lambda p: p.name)
images = sorted(site.rglob("*"))
images = [p for p in images if p.suffix.lower() in (".png", ".jpg", ".jpeg", ".webp", ".gif", ".bmp")]

nac.h1("内容清单")
nac.p("源目录 ", nac.tt(site))
nac.gap()

nac.h2("页面 %d 个" % len(pages))
for page in pages:
    stamp = time.strftime("%m-%d %H:%M", time.localtime(page.stat().st_mtime))
    size = page.stat().st_size
    nac.p(nac.tt(page.stem.rjust(3)), "  ", nac.b(title_of(page)), "  ", nac.tt("%d B" % size), "  ", stamp)
nac.gap()

nac.h2("图片 %d 张" % len(images))
if images:
    nac.bullets(*["%s  %d B" % (p.relative_to(site).as_posix(), p.stat().st_size) for p in images])
else:
    nac.p("没有图片。")
nac.gap()

nac.link(1, "回概览")
nac.link(2, "去发布")

nac.done()
