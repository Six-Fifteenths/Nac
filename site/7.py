# nac:title 留言板
# 演示：读表单、存会话、往站点私有目录里追加记录、计数。

import nac

name = nac.field_of("name")[:40]
if name:
    nac.session_set("name", name)
    nac.append_line("guestbook.txt", name)
    nac.counter("visits.json")

nac.h1("留言板")
nac.p("你上次留下的名字：", nac.session_get("name") or "（还没有）")
nac.p("本页被提交过 ", nac.b(nac.read_json("visits.json", {"n": 0})["n"]), " 次")
nac.gap()

recent = nac.read_lines("guestbook.txt")[-5:]
if recent:
    nac.quote("最近的记录")
    nac.bullets(*recent)
else:
    nac.p("还没有人留言。")

nac.gap()
nac.form(7)
nac.field("name", "你的名字")
nac.button("留下")
nac.gap()
nac.link(1, "回首页")

nac.done()