# nac:title 运营台 · 数据
# nac:path /data
# 看站点私有目录里有什么，按行号删留言。所有路径都由 NAC_ROOT 推出来。

import os
from pathlib import Path

import nac

root = Path(os.environ.get("NAC_ROOT") or ".")
appdata = root / "data" / "appdata"
book = appdata / "guestbook.txt"
fields = nac.fields()
note = None

if "line" in fields:
    try:
        index = int(fields["line"])
    except ValueError:
        index = -1
    if book.exists():
        lines = book.read_text(encoding="utf-8").splitlines()
        if 1 <= index <= len(lines):
            removed = lines.pop(index - 1)
            book.write_text("\n".join(lines) + ("\n" if lines else ""), encoding="utf-8")
            note = "删掉了第 %d 行：%s" % (index, removed)
        else:
            note = "没有第 %d 行" % index
    else:
        note = "留言文件还不存在"

entries = book.read_text(encoding="utf-8").splitlines() if book.exists() else []

nac.h1("数据")
if note:
    nac.quote(note)
    nac.gap()

nac.h2("留言 %d 条" % len(entries))
if entries:
    for number, line in enumerate(entries, start=1):
        nac.p(nac.tt("%2d" % number), " ", nac.esc(line))
else:
    nac.p("还没有留言。")
nac.gap()

nac.form(3)
nac.field("line", "要删的行号")
nac.button("删除")
nac.gap()

nac.h2("目录")
files = sorted(appdata.rglob("*")) if appdata.exists() else []
if files:
    nac.bullets(*["%s  %d B" % (f.relative_to(appdata).as_posix(), f.stat().st_size) for f in files if f.is_file()])
else:
    nac.p("空。")
nac.gap()

nac.link_path("/", "回概览")

nac.done()
