# nac:title 运营台 · 内容清单
# nac:path /inventory
# 只读：把 site/ 里的页面列出来，带上从 # nac:title 读到的页名与改动时间。

import os
import time
from pathlib import Path

import nac

root = Path(os.environ.get("NAC_ROOT") or ".")
site = root / "site"


def head_lines(path):
    return path.read_text(encoding="utf-8", errors="replace").splitlines()[:5]


def title_of(path):
    for line in head_lines(path):
        line = line.strip()
        if line.startswith("# nac:title"):
            return line[len("# nac:title"):].strip() or "（空）"
    return "（未命名）"


def url_of(path):
    for line in head_lines(path):
        line = line.strip()
        if line.startswith("# nac:path"):
            value = line[len("# nac:path"):].strip()
            return value if value.startswith("/") else "/" + value
    rel = path.relative_to(site).with_suffix("")
    parts = list(rel.parts)
    if parts and parts[-1] == "index":
        parts.pop()
        return "/" + "/".join(parts) + ("/" if parts else "")
    return "/" + "/".join(parts)


pages = sorted(site.rglob("*.py"), key=lambda p: p.as_posix())
images = sorted(site.rglob("*"))
images = [p for p in images if p.suffix.lower() in (".png", ".jpg", ".jpeg", ".webp", ".gif", ".bmp")]

nac.h1("内容清单")
nac.p("源目录 ", nac.tt(site))
nac.gap()

nac.h2("页面 %d 个" % len(pages))
for page in pages:
    stamp = time.strftime("%m-%d %H:%M", time.localtime(page.stat().st_mtime))
    size = page.stat().st_size
    nac.p(nac.tt(url_of(page)), "  ", nac.b(title_of(page)), "  ", nac.tt("%d B" % size), "  ", stamp)
nac.gap()

nac.h2("图片 %d 张" % len(images))
if images:
    nac.bullets(*["%s  %d B" % (p.relative_to(site).as_posix(), p.stat().st_size) for p in images])
else:
    nac.p("没有图片。")
nac.gap()

nac.link_path("/", "回概览")
nac.link_path("/publish", "去发布")

nac.done()
