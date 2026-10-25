# nac:title 条目 · EX_MYTH
# nac:path /myth
# nac:match /myth/

import nac

from _lib import myth

arg = nac.path_arg("/myth/")
item = myth.by_id(int(arg)) if arg.isdigit() else None

nac.h1("EX_MYTH")

if item is None:
    nac.p("没有这一条。")
    nac.gap()
    nac.p(nac.inline_path("/", "回索引"))
    nac.stop()

nac.h2("#%d %s" % (item["id"], item["name"]))
nac.p(nac.i(item["domain"]))
nac.gap()
nac.p(item["line"])
nac.gap()

links = [myth.labelled(ident) for ident in item.get("kin", [])]
links = [text for text in links if text]
if links:
    nac.h3("相关")
    for text in links:
        ident = int(text.split()[0].lstrip("#"))
        nac.p(nac.inline_path("/myth/%d" % ident, text))
    nac.gap()

# 前后各一条，按编号走。到头了就只显示存在的那个。
items = myth.myths()
ids = [int(i["id"]) for i in items]
where = ids.index(item["id"])
step = []
if where > 0:
    step.append(nac.inline_path("/myth/%d" % ids[where - 1], "‹ #%d" % ids[where - 1]))
if where + 1 < len(ids):
    step.append(nac.inline_path("/myth/%d" % ids[where + 1], "#%d ›" % ids[where + 1]))
if step:
    nac.p(" · ".join(step))
    nac.gap()

nac.p(nac.inline_path("/", "回索引"))
